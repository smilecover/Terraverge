//! 计算着色器粒子系统。
//!
//! 对应教程「计算着色器」章节：粒子存放在 SSBO 中，由计算着色器每帧更新；
//! 同一个缓冲同时用作顶点缓冲，以点图元绘制。每帧两个缓冲做 ping-pong：
//! 计算着色器读上一帧的缓冲、写当前帧的缓冲，图形阶段再读当前帧缓冲。
//! 图形提交通过信号量等待计算完成，避免顶点阶段读到尚未更新的粒子。

use ash::vk;
use std::error::Error;
use std::ffi::CString;
use std::mem::{offset_of, size_of};
use std::slice;

use super::buffer::{Buffer, create_buffer, create_buffer_with_data};
use super::shader_module::{ShaderStage, create_shader_module};
use super::sync::MAX_FRAMES_IN_FLIGHT;

/// 粒子数量。必须能被计算着色器的 `local_size_x` 整除。
pub(super) const PARTICLE_COUNT: usize = 8192;
/// 每个工作组中的调用数，与计算着色器的 numthreads 一致。
pub(super) const LOCAL_SIZE: u32 = 256;

/// 与计算着色器里的 Particle 结构一致（2+2+4 个 float，共 32 字节）。
#[repr(C)]
#[derive(Clone, Copy)]
pub(super) struct Particle {
    position: [f32; 2],
    velocity: [f32; 2],
    color: [f32; 4],
}

impl Particle {
    /// 粒子缓冲也是顶点缓冲：只暴露位置和颜色，速度仅供计算阶段使用。
    pub(super) fn binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(size_of::<Self>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
    }

    pub(super) fn attribute_descriptions() -> [vk::VertexInputAttributeDescription; 2] {
        [
            vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(0)
                .format(vk::Format::R32G32_SFLOAT)
                .offset(offset_of!(Self, position) as u32),
            vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(1)
                .format(vk::Format::R32G32B32A32_SFLOAT)
                .offset(offset_of!(Self, color) as u32),
        ]
    }
}

/// 计算着色器的 uniform buffer：只有一个 deltaTime。
/// 补齐到 16 字节，满足 std140 对 uniform block 的对齐要求。
#[repr(C)]
#[derive(Clone, Copy)]
struct DeltaTimeUniform {
    delta_time: f32,
    _padding: [f32; 3],
}

/// 计算管线、粒子 SSBO、描述符和计算命令缓冲。
pub(super) struct ComputeResources {
    descriptor_set_layout: vk::DescriptorSetLayout,
    pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
    /// 每帧一个，ping-pong：描述符里绑定上一帧和当前帧的缓冲。
    particle_buffers: Vec<Buffer>,
    /// 每帧一个 deltaTime UBO，持久映射。
    uniform_buffers: Vec<Buffer>,
    uniform_mapped: Vec<*mut u8>,
    descriptor_pool: vk::DescriptorPool,
    descriptor_sets: Vec<vk::DescriptorSet>,
    command_buffers: Vec<vk::CommandBuffer>,
}

impl ComputeResources {
    /// 创建计算管线、粒子缓冲、描述符与每帧命令缓冲。
    pub(super) fn create(
        instance: &ash::Instance,
        device: &ash::Device,
        physical_device: vk::PhysicalDevice,
        command_pool: vk::CommandPool,
        graphics_queue: vk::Queue,
        compute_shader: ShaderStage<'_>,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let descriptor_set_layout = create_descriptor_set_layout(device)?;

        let pipeline_layout = match create_pipeline_layout(device, descriptor_set_layout) {
            Ok(layout) => layout,
            Err(err) => {
                unsafe { device.destroy_descriptor_set_layout(descriptor_set_layout, None) };
                return Err(err.into());
            }
        };

        let pipeline = match create_compute_pipeline(device, pipeline_layout, compute_shader) {
            Ok(pipeline) => pipeline,
            Err(err) => {
                destroy_pipeline_objects(device, pipeline_layout, descriptor_set_layout);
                return Err(err);
            }
        };

        // 初始粒子数据：圆盘内随机分布，速度沿半径向外。
        let particles = initial_particles();
        let particle_data = as_bytes(&particles);
        let mut particle_buffers = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);
        for _ in 0..MAX_FRAMES_IN_FLIGHT {
            // 同一缓冲既做 SSBO 又做顶点缓冲；上传辅助函数会额外加上 TRANSFER_DST。
            let usage = vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::VERTEX_BUFFER;
            match create_buffer_with_data(
                instance,
                device,
                physical_device,
                command_pool,
                graphics_queue,
                particle_data,
                usage,
            ) {
                Ok(buffer) => particle_buffers.push(buffer),
                Err(err) => {
                    destroy_buffers(device, &particle_buffers);
                    destroy_pipeline_and_layouts(
                        device,
                        pipeline,
                        pipeline_layout,
                        descriptor_set_layout,
                    );
                    return Err(err);
                }
            }
        }

        // deltaTime UBO：CPU 每帧写入，所以放在 host-visible 的持久映射内存里。
        let uniform_size = size_of::<DeltaTimeUniform>() as vk::DeviceSize;
        let mut uniform_buffers = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);
        let mut uniform_mapped = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);
        for _ in 0..MAX_FRAMES_IN_FLIGHT {
            let buffer = match create_buffer(
                instance,
                device,
                physical_device,
                uniform_size,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            ) {
                Ok(buffer) => buffer,
                Err(err) => {
                    destroy_mapped_buffers(device, &uniform_buffers);
                    destroy_buffers(device, &particle_buffers);
                    destroy_pipeline_and_layouts(
                        device,
                        pipeline,
                        pipeline_layout,
                        descriptor_set_layout,
                    );
                    return Err(err);
                }
            };

            let pointer = match unsafe {
                device.map_memory(buffer.memory, 0, uniform_size, vk::MemoryMapFlags::empty())
            } {
                Ok(pointer) => pointer.cast::<u8>(),
                Err(err) => {
                    buffer.destroy(device);
                    destroy_mapped_buffers(device, &uniform_buffers);
                    destroy_buffers(device, &particle_buffers);
                    destroy_pipeline_and_layouts(
                        device,
                        pipeline,
                        pipeline_layout,
                        descriptor_set_layout,
                    );
                    return Err(err.into());
                }
            };

            uniform_buffers.push(buffer);
            uniform_mapped.push(pointer);
        }

        let descriptor_pool = match create_descriptor_pool(device) {
            Ok(pool) => pool,
            Err(err) => {
                destroy_mapped_buffers(device, &uniform_buffers);
                destroy_buffers(device, &particle_buffers);
                destroy_pipeline_and_layouts(
                    device,
                    pipeline,
                    pipeline_layout,
                    descriptor_set_layout,
                );
                return Err(err.into());
            }
        };

        let descriptor_sets = match create_descriptor_sets(
            device,
            descriptor_pool,
            descriptor_set_layout,
            &uniform_buffers,
            &particle_buffers,
        ) {
            Ok(sets) => sets,
            Err(err) => {
                unsafe { device.destroy_descriptor_pool(descriptor_pool, None) };
                destroy_mapped_buffers(device, &uniform_buffers);
                destroy_buffers(device, &particle_buffers);
                destroy_pipeline_and_layouts(
                    device,
                    pipeline,
                    pipeline_layout,
                    descriptor_set_layout,
                );
                return Err(err);
            }
        };

        let command_buffers = match super::command_buffer::allocate_command_buffers(
            device,
            command_pool,
            MAX_FRAMES_IN_FLIGHT,
        ) {
            Ok(command_buffers) => command_buffers,
            Err(err) => {
                unsafe { device.destroy_descriptor_pool(descriptor_pool, None) };
                destroy_mapped_buffers(device, &uniform_buffers);
                destroy_buffers(device, &particle_buffers);
                destroy_pipeline_and_layouts(
                    device,
                    pipeline,
                    pipeline_layout,
                    descriptor_set_layout,
                );
                return Err(err.into());
            }
        };

        Ok(Self {
            descriptor_set_layout,
            pipeline_layout,
            pipeline,
            particle_buffers,
            uniform_buffers,
            uniform_mapped,
            descriptor_pool,
            descriptor_sets,
            command_buffers,
        })
    }

    /// 每帧更新计算 UBO 里的 deltaTime（毫秒）。
    pub(super) fn update_delta_time(&self, frame: usize, delta_time: f32) {
        let uniform = DeltaTimeUniform {
            delta_time,
            _padding: [0.0; 3],
        };
        // HOST_COHERENT 内存写入后无需 flush；帧栅栏已保证计算不再读它。
        unsafe {
            (self.uniform_mapped[frame] as *mut DeltaTimeUniform).write(uniform);
        }
    }

    pub(super) fn command_buffer(&self, frame: usize) -> vk::CommandBuffer {
        self.command_buffers[frame]
    }

    pub(super) fn pipeline(&self) -> vk::Pipeline {
        self.pipeline
    }

    pub(super) fn pipeline_layout(&self) -> vk::PipelineLayout {
        self.pipeline_layout
    }

    pub(super) fn descriptor_set(&self, frame: usize) -> vk::DescriptorSet {
        self.descriptor_sets[frame]
    }

    /// 当前帧计算写出的粒子缓冲，图形阶段把它当作顶点缓冲。
    pub(super) fn particle_buffer(&self, frame: usize) -> vk::Buffer {
        self.particle_buffers[frame].handle
    }

    pub(super) fn destroy(&self, device: &ash::Device) {
        // 先释放描述符集/UBO/粒子缓冲，再按“管线 → 布局”的顺序销毁。
        unsafe { device.destroy_descriptor_pool(self.descriptor_pool, None) };
        for buffer in &self.uniform_buffers {
            unsafe { device.unmap_memory(buffer.memory) };
            buffer.destroy(device);
        }
        for buffer in &self.particle_buffers {
            buffer.destroy(device);
        }
        unsafe { device.destroy_pipeline(self.pipeline, None) };
        destroy_pipeline_objects(device, self.pipeline_layout, self.descriptor_set_layout);
    }
}

fn create_descriptor_set_layout(
    device: &ash::Device,
) -> Result<vk::DescriptorSetLayout, vk::Result> {
    // binding 0：deltaTime UBO；binding 1/2：上一帧/当前帧粒子 SSBO。
    let bindings = [
        vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::COMPUTE),
        vk::DescriptorSetLayoutBinding::default()
            .binding(1)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::COMPUTE),
        vk::DescriptorSetLayoutBinding::default()
            .binding(2)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::COMPUTE),
    ];

    let create_info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings);
    unsafe { device.create_descriptor_set_layout(&create_info, None) }
}

fn create_pipeline_layout(
    device: &ash::Device,
    descriptor_set_layout: vk::DescriptorSetLayout,
) -> Result<vk::PipelineLayout, vk::Result> {
    let set_layouts = [descriptor_set_layout];
    let create_info = vk::PipelineLayoutCreateInfo::default().set_layouts(&set_layouts);
    unsafe { device.create_pipeline_layout(&create_info, None) }
}

fn create_compute_pipeline(
    device: &ash::Device,
    pipeline_layout: vk::PipelineLayout,
    shader: ShaderStage<'_>,
) -> Result<vk::Pipeline, Box<dyn Error + Send + Sync>> {
    let module = create_shader_module(device, shader.code)?;
    let entry_point = CString::new(shader.entry_point)?;

    let stage = vk::PipelineShaderStageCreateInfo::default()
        .stage(vk::ShaderStageFlags::COMPUTE)
        .module(module)
        .name(&entry_point);

    let pipeline_info = vk::ComputePipelineCreateInfo::default()
        .layout(pipeline_layout)
        .stage(stage)
        .base_pipeline_index(-1);

    let result = unsafe {
        device.create_compute_pipelines(vk::PipelineCache::null(), &[pipeline_info], None)
    };

    // SPIR-V 已经被管线吸收，着色器模块可以立即销毁。
    unsafe { device.destroy_shader_module(module, None) };

    match result {
        Ok(mut pipelines) => Ok(pipelines.pop().expect("创建成功时必然返回一条计算管线")),
        Err((pipelines, err)) => {
            for pipeline in pipelines {
                unsafe { device.destroy_pipeline(pipeline, None) };
            }
            Err(err.into())
        }
    }
}

fn create_descriptor_pool(device: &ash::Device) -> Result<vk::DescriptorPool, vk::Result> {
    let pool_sizes = [
        vk::DescriptorPoolSize::default()
            .ty(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(MAX_FRAMES_IN_FLIGHT as u32),
        // 每个集合两个 SSBO。
        vk::DescriptorPoolSize::default()
            .ty(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count((MAX_FRAMES_IN_FLIGHT * 2) as u32),
    ];

    let create_info = vk::DescriptorPoolCreateInfo::default()
        .pool_sizes(&pool_sizes)
        .max_sets(MAX_FRAMES_IN_FLIGHT as u32);

    unsafe { device.create_descriptor_pool(&create_info, None) }
}

fn create_descriptor_sets(
    device: &ash::Device,
    descriptor_pool: vk::DescriptorPool,
    descriptor_set_layout: vk::DescriptorSetLayout,
    uniform_buffers: &[Buffer],
    particle_buffers: &[Buffer],
) -> Result<Vec<vk::DescriptorSet>, Box<dyn Error + Send + Sync>> {
    let layouts = vec![descriptor_set_layout; MAX_FRAMES_IN_FLIGHT];
    let allocate_info = vk::DescriptorSetAllocateInfo::default()
        .descriptor_pool(descriptor_pool)
        .set_layouts(&layouts);

    let descriptor_sets = unsafe { device.allocate_descriptor_sets(&allocate_info)? };
    let particle_range = (PARTICLE_COUNT * size_of::<Particle>()) as vk::DeviceSize;

    for (frame, descriptor_set) in descriptor_sets.iter().enumerate() {
        // ping-pong：读上一帧缓冲，写当前帧缓冲。
        let previous_frame = (frame + MAX_FRAMES_IN_FLIGHT - 1) % MAX_FRAMES_IN_FLIGHT;

        let uniform_info = vk::DescriptorBufferInfo::default()
            .buffer(uniform_buffers[frame].handle)
            .offset(0)
            .range(size_of::<DeltaTimeUniform>() as vk::DeviceSize);
        let previous_particles_info = vk::DescriptorBufferInfo::default()
            .buffer(particle_buffers[previous_frame].handle)
            .offset(0)
            .range(particle_range);
        let current_particles_info = vk::DescriptorBufferInfo::default()
            .buffer(particle_buffers[frame].handle)
            .offset(0)
            .range(particle_range);

        let uniform_infos = [uniform_info];
        let previous_particles_infos = [previous_particles_info];
        let current_particles_infos = [current_particles_info];

        let writes = [
            vk::WriteDescriptorSet::default()
                .dst_set(*descriptor_set)
                .dst_binding(0)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(&uniform_infos),
            vk::WriteDescriptorSet::default()
                .dst_set(*descriptor_set)
                .dst_binding(1)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .buffer_info(&previous_particles_infos),
            vk::WriteDescriptorSet::default()
                .dst_set(*descriptor_set)
                .dst_binding(2)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .buffer_info(&current_particles_infos),
        ];

        unsafe { device.update_descriptor_sets(&writes, &[]) };
    }

    Ok(descriptor_sets)
}

/// 生成教程里的初始粒子：圆盘内随机位置、沿半径向外的速度、随机颜色。
fn initial_particles() -> Vec<Particle> {
    // 固定种子的 LCG，避免额外引入随机数依赖；每次启动图案一致。
    let mut seed = 0x5EED_1234u32;
    let mut random = || {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (seed >> 8) as f32 / 16_777_216.0
    };

    let aspect = 600.0 / 800.0;
    let mut particles = Vec::with_capacity(PARTICLE_COUNT);

    for _ in 0..PARTICLE_COUNT {
        let radius = 0.25 * random().sqrt();
        let theta = random() * std::f32::consts::TAU;
        let x = radius * theta.cos() * aspect;
        let y = radius * theta.sin();

        let length = (x * x + y * y).sqrt();
        let velocity = if length > f32::EPSILON {
            [x / length * 0.00025, y / length * 0.00025]
        } else {
            [0.0, 0.0]
        };

        particles.push(Particle {
            position: [x, y],
            velocity,
            color: [random(), random(), random(), 1.0],
        });
    }

    particles
}

/// 回收已经创建的计算管线，再销毁它引用的管线布局和描述符集布局。
fn destroy_pipeline_and_layouts(
    device: &ash::Device,
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    descriptor_set_layout: vk::DescriptorSetLayout,
) {
    unsafe { device.destroy_pipeline(pipeline, None) };
    destroy_pipeline_objects(device, pipeline_layout, descriptor_set_layout);
}

fn destroy_pipeline_objects(
    device: &ash::Device,
    pipeline_layout: vk::PipelineLayout,
    descriptor_set_layout: vk::DescriptorSetLayout,
) {
    unsafe {
        device.destroy_pipeline_layout(pipeline_layout, None);
        device.destroy_descriptor_set_layout(descriptor_set_layout, None);
    }
}

fn destroy_buffers(device: &ash::Device, buffers: &[Buffer]) {
    for buffer in buffers {
        buffer.destroy(device);
    }
}

fn destroy_mapped_buffers(device: &ash::Device, buffers: &[Buffer]) {
    for buffer in buffers {
        unsafe { device.unmap_memory(buffer.memory) };
        buffer.destroy(device);
    }
}

/// 把 `Particle` 数组按字节看待，用于上传到 SSBO。
///
/// SAFETY（调用方保证）：`Particle` 是 `#[repr(C)]` 的纯数据、无指针/`Drop`。
fn as_bytes<T>(data: &[T]) -> &[u8] {
    unsafe { slice::from_raw_parts(data.as_ptr().cast(), std::mem::size_of_val(data)) }
}
