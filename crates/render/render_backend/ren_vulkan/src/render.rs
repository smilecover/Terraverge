//! Vulkan 渲染后端入口：持有实例、设备、交换链、图形管线与绘制资源，
//! 并保证正确的创建/销毁顺序。
//!
//! 按教程章节拆分为子模块：
//! - `debug`：校验层与调试信使；
//! - `instance`：`VkInstance` 创建；
//! - `surface`：窗口表面 `VkSurfaceKHR`；
//! - `device`：物理设备选择与逻辑设备；
//! - `swapchain`：交换链查询与创建；
//! - `image_view`：交换链图像的图像视图；
//! - `shader_module`：SPIR-V 着色器模块（「Shader modules」章节）；
//! - `render_pass`：渲染通道与附件（「Render passes」章节）；
//! - `pipeline`：固定功能状态与图形管线（「Fixed functions」章节）；
//! - `framebuffer`：把图像视图绑定到渲染通道（「Framebuffers」章节）；
//! - `command_buffer`：命令池、命令缓冲与录制（「Command buffers」章节）；
//! - `sync`：帧信号量/栅栏（「Frames in flight」章节）；
//! - `vertex`：顶点数据与顶点输入描述（「Vertex input description」章节）；
//! - `buffer`：顶点/索引缓冲与暂存缓冲（「Vertex buffer」「Staging buffer」
//!   「Index buffer」章节）；
//! - `descriptor`：描述符集布局、描述符池与描述符集
//!   （「Descriptor set layout and buffer」「Descriptor pool and sets」章节）；
//! - `uniform_buffer`：MVP uniform buffer 与每帧更新；
//! - `texture`：纹理图像、图像视图与采样器
//!   （「Images」「Image view and sampler」「Combined image sampler」章节）；
//! - `depth`：深度图像与深度格式选择（「Depth buffering」章节）；
//! - `msaa`：多重采样颜色附件与 resolve（「Multisampling」章节）；
//! - `compute`：计算着色器粒子系统（「Compute Shader」章节）。

mod buffer;
mod command_buffer;
mod compute;
mod debug;
mod depth;
mod descriptor;
mod device;
mod framebuffer;
mod image_view;
mod instance;
mod msaa;
mod pipeline;
mod render_pass;
mod shader_module;
mod surface;
mod swapchain;
mod sync;
mod texture;
mod uniform_buffer;
mod vertex;

use ash::{Entry, vk};
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use std::error::Error;

use self::buffer::{Geometry, create_geometry};
use self::command_buffer::{
    DrawState, allocate_command_buffers, create_command_pool, record_command_buffer,
    record_compute_command_buffer,
};
use self::compute::{ComputeResources, LOCAL_SIZE, PARTICLE_COUNT};
use self::depth::{DepthResources, find_depth_format};
use self::descriptor::{
    create_descriptor_pool, create_descriptor_set_layout, create_descriptor_sets,
};
use self::device::{LogicalDevice, create_logical_device, pick_physical_device};
use self::framebuffer::{create_framebuffers, destroy_framebuffers};
use self::image_view::create_swapchain_image_views;
use self::instance::create_instance;
use self::msaa::{MsaaColor, find_sample_count};
use self::pipeline::{GraphicsPipeline, create_graphics_pipeline, create_particle_pipeline};
use self::render_pass::create_render_pass;
use self::surface::create_surface;
use self::swapchain::{Swapchain, SwapchainOptions, create_swapchain};
use self::sync::{FrameSync, MAX_FRAMES_IN_FLIGHT};
use self::texture::Texture;
use self::uniform_buffer::UniformBuffers;

pub use self::shader_module::{ParticleShaders, SceneShaders, ShaderProgram, ShaderStage};
pub use self::texture::{TextureData, TextureFilter};
pub use self::uniform_buffer::SceneUniforms;
pub use self::vertex::{MeshData, Vertex};

pub struct Render {
    _entry: Entry,
    instance: ash::Instance,
    surface_loader: ash::khr::surface::Instance,
    surface: vk::SurfaceKHR,
    physical_device: vk::PhysicalDevice,
    device: ash::Device,
    graphics_queue: vk::Queue,
    present_queue: vk::Queue,
    swapchain: Swapchain,
    image_views: Vec<vk::ImageView>,
    msaa_color: MsaaColor,
    depth: DepthResources,
    render_pass: vk::RenderPass,
    graphics_pipeline: GraphicsPipeline,
    /// 计算着色器粒子系统的点精灵管线。
    particle_pipeline: GraphicsPipeline,
    descriptor_set_layout: vk::DescriptorSetLayout,
    framebuffers: Vec<vk::Framebuffer>,
    command_pool: vk::CommandPool,
    command_buffers: Vec<vk::CommandBuffer>,
    geometry: Geometry,
    uniform_buffers: UniformBuffers,
    texture: Texture,
    descriptor_pool: vk::DescriptorPool,
    descriptor_sets: Vec<vk::DescriptorSet>,
    /// 计算粒子系统：SSBO、计算管线与描述符。
    compute: ComputeResources,
    /// 渲染通道/管线/附件共用的多重采样数，重建时保持不变。
    msaa_samples: vk::SampleCountFlags,
    sync: FrameSync,
    /// 最近一次窗口尺寸；交换链重建时使用。
    window_width: u32,
    window_height: u32,
    /// 窗口尺寸变化后由绘制循环触发交换链重建。
    framebuffer_resized: bool,
    /// 是否绘制计算着色器生成的示例粒子。Minecraft 客户端不需要它们。
    particles_enabled: bool,
    #[cfg(debug_assertions)]
    debug_utils: Option<ash::ext::debug_utils::Instance>,
    #[cfg(debug_assertions)]
    debug_messenger: Option<vk::DebugUtilsMessengerEXT>,
}

/// 绘制阶段创建的资源，全部依赖渲染通道、交换链和图形队列族。
struct DrawingResources {
    msaa_color: MsaaColor,
    depth: DepthResources,
    framebuffers: Vec<vk::Framebuffer>,
    command_pool: vk::CommandPool,
    command_buffers: Vec<vk::CommandBuffer>,
    geometry: Geometry,
    uniform_buffers: UniformBuffers,
    texture: Texture,
    descriptor_pool: vk::DescriptorPool,
    descriptor_sets: Vec<vk::DescriptorSet>,
    compute: ComputeResources,
    sync: FrameSync,
}

/// 绘制资源依赖的渲染目标以及游戏层提供的资源。
struct DrawingSetup<'a> {
    render_pass: vk::RenderPass,
    descriptor_set_layout: vk::DescriptorSetLayout,
    image_views: &'a [vk::ImageView],
    extent: vk::Extent2D,
    color_format: vk::Format,
    depth_format: vk::Format,
    sample_count: vk::SampleCountFlags,
    mesh: MeshData<'a>,
    texture: TextureData<'a>,
    compute_shader: ShaderStage<'a>,
}

/// 创建 MSAA 颜色附件、深度缓冲、帧缓冲、命令池/缓冲、顶点/索引缓冲、
/// uniform buffer、纹理、描述符池/集和帧同步对象（教程的 createColorResources +
/// createDepthResources + createFramebuffers + createCommandPool +
/// createCommandBuffers + createVertexBuffer + createIndexBuffer +
/// createUniformBuffers + createTextureImage + createDescriptorPool +
/// createDescriptorSets + createSyncObjects）。
/// 中途失败时回收本函数已创建的对象，外层的交换链等由调用者处理。
fn create_drawing_resources(
    instance: &ash::Instance,
    logical_device: &LogicalDevice,
    physical_device: vk::PhysicalDevice,
    setup: DrawingSetup<'_>,
) -> Result<DrawingResources, Box<dyn Error + Send + Sync>> {
    let device = &logical_device.device;

    // 深度缓冲与交换链同尺寸；格式/采样数必须与渲染通道的深度附件一致。
    let depth = DepthResources::create(
        instance,
        device,
        physical_device,
        setup.extent,
        setup.depth_format,
        setup.sample_count,
    )?;

    // MSAA 颜色附件：多采样渲染，随后由渲染通道 resolve 到交换链图像。
    let msaa_color = match MsaaColor::create(
        instance,
        device,
        physical_device,
        setup.extent,
        setup.color_format,
        setup.sample_count,
    ) {
        Ok(msaa_color) => msaa_color,
        Err(err) => {
            depth.destroy(device);
            return Err(err);
        }
    };

    // 帧缓冲的附件顺序与渲染通道一致：MSAA 颜色、深度、交换链图像。
    let mut framebuffers = match create_framebuffers(
        device,
        setup.render_pass,
        msaa_color.view(),
        depth.view(),
        setup.image_views,
        setup.extent,
    ) {
        Ok(framebuffers) => framebuffers,
        Err(err) => {
            msaa_color.destroy(device);
            depth.destroy(device);
            return Err(err);
        }
    };

    let command_pool = match create_command_pool(device, logical_device.graphics_family) {
        Ok(command_pool) => command_pool,
        Err(err) => {
            destroy_framebuffers(device, &mut framebuffers);
            msaa_color.destroy(device);
            depth.destroy(device);
            return Err(err.into());
        }
    };

    let command_buffers = match allocate_command_buffers(device, command_pool, MAX_FRAMES_IN_FLIGHT)
    {
        Ok(command_buffers) => command_buffers,
        Err(err) => {
            unsafe { device.destroy_command_pool(command_pool, None) };
            destroy_framebuffers(device, &mut framebuffers);
            msaa_color.destroy(device);
            depth.destroy(device);
            return Err(err.into());
        }
    };

    // 顶点/索引缓冲通过暂存缓冲上传到设备本地内存。
    let geometry = match create_geometry(
        instance,
        device,
        physical_device,
        command_pool,
        logical_device.graphics_queue,
        setup.mesh,
    ) {
        Ok(geometry) => geometry,
        Err(err) => {
            unsafe { device.destroy_command_pool(command_pool, None) };
            destroy_framebuffers(device, &mut framebuffers);
            msaa_color.destroy(device);
            depth.destroy(device);
            return Err(err);
        }
    };

    // 每个在途帧一个 UBO，CPU 持续写入 MVP 矩阵。
    let uniform_buffers = match UniformBuffers::create(instance, device, physical_device) {
        Ok(uniform_buffers) => uniform_buffers,
        Err(err) => {
            geometry.destroy(device);
            unsafe { device.destroy_command_pool(command_pool, None) };
            destroy_framebuffers(device, &mut framebuffers);
            msaa_color.destroy(device);
            depth.destroy(device);
            return Err(err);
        }
    };

    // 纹理：创建图像、上传像素、转布局，并创建视图与采样器。
    let texture = match Texture::create(
        instance,
        device,
        physical_device,
        command_pool,
        logical_device.graphics_queue,
        &setup.texture,
    ) {
        Ok(texture) => texture,
        Err(err) => {
            uniform_buffers.destroy(device);
            geometry.destroy(device);
            unsafe { device.destroy_command_pool(command_pool, None) };
            destroy_framebuffers(device, &mut framebuffers);
            msaa_color.destroy(device);
            depth.destroy(device);
            return Err(err);
        }
    };

    // 描述符池分配完再创建描述符集，并把 UBO/纹理写进 binding 0/1。
    let descriptor_pool = match create_descriptor_pool(device) {
        Ok(descriptor_pool) => descriptor_pool,
        Err(err) => {
            texture.destroy(device);
            uniform_buffers.destroy(device);
            geometry.destroy(device);
            unsafe { device.destroy_command_pool(command_pool, None) };
            destroy_framebuffers(device, &mut framebuffers);
            msaa_color.destroy(device);
            depth.destroy(device);
            return Err(err.into());
        }
    };

    let descriptor_sets = match create_descriptor_sets(
        device,
        descriptor_pool,
        setup.descriptor_set_layout,
        uniform_buffers.buffers(),
        &texture,
    ) {
        Ok(descriptor_sets) => descriptor_sets,
        Err(err) => {
            // 销毁池会一并释放已经分配出去的描述符集。
            unsafe { device.destroy_descriptor_pool(descriptor_pool, None) };
            texture.destroy(device);
            uniform_buffers.destroy(device);
            geometry.destroy(device);
            unsafe { device.destroy_command_pool(command_pool, None) };
            destroy_framebuffers(device, &mut framebuffers);
            msaa_color.destroy(device);
            depth.destroy(device);
            return Err(err);
        }
    };

    let sync = match FrameSync::create(device, setup.image_views.len()) {
        Ok(sync) => sync,
        Err(err) => {
            unsafe { device.destroy_descriptor_pool(descriptor_pool, None) };
            texture.destroy(device);
            uniform_buffers.destroy(device);
            geometry.destroy(device);
            unsafe { device.destroy_command_pool(command_pool, None) };
            destroy_framebuffers(device, &mut framebuffers);
            msaa_color.destroy(device);
            depth.destroy(device);
            return Err(err);
        }
    };

    // 计算粒子系统：SSBO、计算管线、计算描述符和计算命令缓冲。
    // 创建失败时它会自行回收半成品，这里只需回收之前创建的资源。
    let compute = match ComputeResources::create(
        instance,
        device,
        physical_device,
        command_pool,
        logical_device.graphics_queue,
        setup.compute_shader,
    ) {
        Ok(compute) => compute,
        Err(err) => {
            sync.destroy(device);
            unsafe { device.destroy_descriptor_pool(descriptor_pool, None) };
            texture.destroy(device);
            uniform_buffers.destroy(device);
            geometry.destroy(device);
            unsafe { device.destroy_command_pool(command_pool, None) };
            destroy_framebuffers(device, &mut framebuffers);
            msaa_color.destroy(device);
            depth.destroy(device);
            return Err(err);
        }
    };

    Ok(DrawingResources {
        msaa_color,
        depth,
        framebuffers,
        command_pool,
        command_buffers,
        geometry,
        uniform_buffers,
        texture,
        descriptor_pool,
        descriptor_sets,
        compute,
        sync,
    })
}

impl Render {
    pub fn new(
        display_handle: RawDisplayHandle,
        window_handle: RawWindowHandle,
        width: u32,
        height: u32,
        shaders: SceneShaders<'_>,
        mesh: MeshData<'_>,
        texture: TextureData<'_>,
    ) -> Result<Render, Box<dyn Error + Send + Sync>> {
        // 实例创建阶段（教程的 createInstance）：Entry、VkInstance，调试构建下还有调试信使。
        let instance_context = create_instance()?;
        let entry = instance_context.entry;
        let instance = instance_context.instance;
        #[cfg(debug_assertions)]
        let debug_utils = instance_context.debug_utils;
        #[cfg(debug_assertions)]
        let debug_messenger = instance_context.debug_messenger;

        // 创建窗口表面（教程的 createSurface）。ash-window 相当于 glfwCreateWindowSurface：
        // 根据原始窗口句柄自动选择平台相关扩展（Windows 下就是 VK_KHR_win32_surface）。
        let surface_loader = ash::khr::surface::Instance::new(&entry, &instance);

        // Render 尚未构造，Drop 不会执行；任何一步失败都按“逆创建顺序”手动回收。
        let cleanup = |device: Option<&ash::Device>,
                       swapchain_loader: Option<&ash::khr::swapchain::Device>,
                       swapchain: Option<vk::SwapchainKHR>,
                       image_views: &[vk::ImageView],
                       render_pass: Option<vk::RenderPass>,
                       descriptor_set_layout: Option<vk::DescriptorSetLayout>,
                       pipeline: Option<&GraphicsPipeline>,
                       particle_pipeline: Option<&GraphicsPipeline>,
                       surface: Option<vk::SurfaceKHR>| {
            if let Some(device) = device {
                // 管线引用管线布局与渲染通道，先于二者销毁。
                if let Some(particle_pipeline) = particle_pipeline {
                    particle_pipeline.destroy(device);
                }
                if let Some(pipeline) = pipeline {
                    pipeline.destroy(device);
                }
                // 管线布局引用描述符集布局，所以在管线之后销毁。
                if let Some(descriptor_set_layout) = descriptor_set_layout {
                    unsafe { device.destroy_descriptor_set_layout(descriptor_set_layout, None) };
                }
                // 渲染通道先于交换链图像（以及它们的图像视图）销毁。
                if let Some(render_pass) = render_pass {
                    unsafe { device.destroy_render_pass(render_pass, None) };
                }
                for &image_view in image_views {
                    unsafe { device.destroy_image_view(image_view, None) };
                }
            }
            if let (Some(loader), Some(swapchain)) = (swapchain_loader, swapchain) {
                unsafe { loader.destroy_swapchain(swapchain, None) };
            }
            if let Some(device) = device {
                unsafe { device.destroy_device(None) };
            }
            #[cfg(debug_assertions)]
            if let Some(messenger) = debug_messenger {
                unsafe { debug_utils.destroy_debug_utils_messenger(messenger, None) };
            }
            if let Some(surface) = surface {
                unsafe { surface_loader.destroy_surface(surface, None) };
            }
            unsafe { instance.destroy_instance(None) };
        };

        let surface = match create_surface(&entry, &instance, display_handle, window_handle) {
            Ok(surface) => surface,
            Err(err) => {
                cleanup(None, None, None, &[], None, None, None, None, None);
                return Err(err);
            }
        };

        // 选择物理设备（教程的 pickPhysicalDevice）。
        let physical_device = match pick_physical_device(&instance, &surface_loader, surface) {
            Ok(device) => device,
            Err(err) => {
                cleanup(None, None, None, &[], None, None, None, None, Some(surface));
                return Err(err);
            }
        };

        // 选择 MSAA 采样数（教程的 getMaxUsableSampleCount）。
        // 颜色和深度都必须支持该采样数。
        let msaa_samples = find_sample_count(&instance, physical_device);
        if msaa_samples == vk::SampleCountFlags::TYPE_1 {
            cleanup(None, None, None, &[], None, None, None, None, Some(surface));
            return Err("设备不支持多重采样，无法启用 MSAA".into());
        }

        // 创建逻辑设备并取回图形/呈现队列（教程的 createLogicalDevice）。
        let logical_device =
            match create_logical_device(&instance, &surface_loader, surface, physical_device) {
                Ok(logical_device) => logical_device,
                Err(err) => {
                    cleanup(None, None, None, &[], None, None, None, None, Some(surface));
                    return Err(err);
                }
            };
        let device = &logical_device.device;

        // 创建交换链（教程的 createSwapChain）。
        let swapchain = match create_swapchain(
            &instance,
            device,
            &surface_loader,
            surface,
            physical_device,
            SwapchainOptions {
                width,
                height,
                old_swapchain: vk::SwapchainKHR::null(),
            },
        ) {
            Ok(swapchain) => swapchain,
            Err(err) => {
                cleanup(
                    Some(device),
                    None,
                    None,
                    &[],
                    None,
                    None,
                    None,
                    None,
                    Some(surface),
                );
                return Err(err);
            }
        };

        // 为交换链的每张图像创建图像视图（教程的 createImageViews）。
        let image_views =
            match create_swapchain_image_views(device, &swapchain.images, swapchain.format) {
                Ok(image_views) => image_views,
                Err(err) => {
                    cleanup(
                        Some(device),
                        Some(&swapchain.loader),
                        Some(swapchain.handle),
                        &[],
                        None,
                        None,
                        None,
                        None,
                        Some(surface),
                    );
                    return Err(err);
                }
            };

        // 创建描述符集布局（教程的 createDescriptorSetLayout）。
        // 管线布局会引用它，所以必须先于图形管线创建。
        let descriptor_set_layout = match create_descriptor_set_layout(device) {
            Ok(descriptor_set_layout) => descriptor_set_layout,
            Err(err) => {
                cleanup(
                    Some(device),
                    Some(&swapchain.loader),
                    Some(swapchain.handle),
                    &image_views,
                    None,
                    None,
                    None,
                    None,
                    Some(surface),
                );
                return Err(err.into());
            }
        };

        // 选择深度格式（教程的 findDepthFormat）。渲染通道与深度图像必须一致。
        let depth_format = match find_depth_format(&instance, physical_device) {
            Ok(depth_format) => depth_format,
            Err(err) => {
                cleanup(
                    Some(device),
                    Some(&swapchain.loader),
                    Some(swapchain.handle),
                    &image_views,
                    None,
                    Some(descriptor_set_layout),
                    None,
                    None,
                    Some(surface),
                );
                return Err(err);
            }
        };

        // 创建渲染通道（教程的 createRenderPass）。颜色/深度格式必须与实际附件一致。
        let render_pass =
            match create_render_pass(device, swapchain.format, depth_format, msaa_samples) {
                Ok(render_pass) => render_pass,
                Err(err) => {
                    cleanup(
                        Some(device),
                        Some(&swapchain.loader),
                        Some(swapchain.handle),
                        &image_views,
                        None,
                        Some(descriptor_set_layout),
                        None,
                        None,
                        Some(surface),
                    );
                    return Err(err.into());
                }
            };

        // 创建图形管线（教程的 createGraphicsPipeline）：着色器模块 + 固定功能状态。
        let graphics_pipeline = match create_graphics_pipeline(
            device,
            render_pass,
            descriptor_set_layout,
            msaa_samples,
            &shaders.model,
        ) {
            Ok(pipeline) => pipeline,
            Err(err) => {
                cleanup(
                    Some(device),
                    Some(&swapchain.loader),
                    Some(swapchain.handle),
                    &image_views,
                    Some(render_pass),
                    Some(descriptor_set_layout),
                    None,
                    None,
                    Some(surface),
                );
                return Err(err);
            }
        };

        // 计算粒子系统的点精灵管线：无描述符、点图元、alpha 混合。
        let particle_program = shaders.particles.graphics_program();
        let particle_pipeline =
            match create_particle_pipeline(device, render_pass, msaa_samples, &particle_program) {
                Ok(pipeline) => pipeline,
                Err(err) => {
                    cleanup(
                        Some(device),
                        Some(&swapchain.loader),
                        Some(swapchain.handle),
                        &image_views,
                        Some(render_pass),
                        Some(descriptor_set_layout),
                        Some(&graphics_pipeline),
                        None,
                        Some(surface),
                    );
                    return Err(err);
                }
            };

        // 创建帧缓冲、命令池/缓冲、几何、UBO、纹理、描述符池/集与同步对象。
        let drawing = match create_drawing_resources(
            &instance,
            &logical_device,
            physical_device,
            DrawingSetup {
                render_pass,
                descriptor_set_layout,
                image_views: &image_views,
                extent: swapchain.extent,
                color_format: swapchain.format,
                depth_format,
                sample_count: msaa_samples,
                mesh,
                texture,
                compute_shader: shaders.particles.compute,
            },
        ) {
            Ok(drawing) => drawing,
            Err(err) => {
                cleanup(
                    Some(device),
                    Some(&swapchain.loader),
                    Some(swapchain.handle),
                    &image_views,
                    Some(render_pass),
                    Some(descriptor_set_layout),
                    Some(&graphics_pipeline),
                    Some(&particle_pipeline),
                    Some(surface),
                );
                return Err(err);
            }
        };

        let LogicalDevice {
            device,
            graphics_queue,
            present_queue,
            ..
        } = logical_device;

        let render = Self {
            _entry: entry,
            instance,
            surface_loader,
            surface,
            physical_device,
            device,
            graphics_queue,
            present_queue,
            swapchain,
            image_views,
            msaa_color: drawing.msaa_color,
            depth: drawing.depth,
            render_pass,
            graphics_pipeline,
            particle_pipeline,
            descriptor_set_layout,
            framebuffers: drawing.framebuffers,
            command_pool: drawing.command_pool,
            command_buffers: drawing.command_buffers,
            geometry: drawing.geometry,
            uniform_buffers: drawing.uniform_buffers,
            texture: drawing.texture,
            descriptor_pool: drawing.descriptor_pool,
            descriptor_sets: drawing.descriptor_sets,
            compute: drawing.compute,
            msaa_samples,
            sync: drawing.sync,
            window_width: width,
            window_height: height,
            framebuffer_resized: false,
            particles_enabled: true,
            #[cfg(debug_assertions)]
            debug_utils: Some(debug_utils),
            #[cfg(debug_assertions)]
            debug_messenger,
        };
        Ok(render)
    }

    /// 控制示例粒子系统的绘制。计算管线仍会运行，只是不提交粒子的点精灵绘制。
    pub fn set_particles_enabled(&mut self, enabled: bool) {
        self.particles_enabled = enabled;
    }

    /// 用新网格替换世界几何。
    ///
    /// 先等待设备空闲，确保命令缓冲不再引用旧缓冲，再创建新缓冲并销毁旧缓冲。
    /// 创建失败时保留旧网格，渲染循环仍可继续。
    pub fn update_mesh(&mut self, mesh: MeshData<'_>) -> Result<(), Box<dyn Error + Send + Sync>> {
        unsafe { self.device.device_wait_idle()? };
        let geometry = create_geometry(
            &self.instance,
            &self.device,
            self.physical_device,
            self.command_pool,
            self.graphics_queue,
            mesh,
        )?;
        let old_geometry = std::mem::replace(&mut self.geometry, geometry);
        old_geometry.destroy(&self.device);
        Ok(())
    }

    /// 窗口尺寸变化时记录新尺寸，实际重建放在下一次绘制，避免连续 resize 时反复重建。
    pub fn resize(&mut self, width: u32, height: u32) {
        self.window_width = width;
        self.window_height = height;
        self.framebuffer_resized = true;
    }

    /// 绘制并呈现一帧（教程的 `drawFrame`）：
    /// 等栅栏 → 获取图像 → 录制 → 提交 → 呈现，必要时重建交换链。
    /// 绘制并呈现一帧。MVP 与粒子 deltaTime 由游戏层按场景逻辑算好传入；
    /// 渲染后端只负责把数据写进 GPU 缓冲并执行绘制/呈现。
    pub fn draw_frame(
        &mut self,
        uniforms: &SceneUniforms,
        delta_time: f32,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        // 最小化时宽高为 0，交换链无法创建/呈现；等窗口恢复后再继续。
        if self.window_width == 0 || self.window_height == 0 {
            return Ok(());
        }

        // 窗口尺寸变化后，先重建交换链再开始这一帧。
        if self.framebuffer_resized {
            self.framebuffer_resized = false;
            self.recreate_swapchain()?;
        }

        let frame = self.sync.current_frame;
        let in_flight_fence = self.sync.in_flight_fence(frame);

        // 等这一帧上一次计算完成，才能安全更新 deltaTime 和粒子 SSBO。
        let compute_fence = self.sync.compute_in_flight_fence(frame);
        unsafe {
            self.device
                .wait_for_fences(&[compute_fence], true, u64::MAX)?
        };

        // 等这一帧上一次图形提交完成，才能安全重置命令缓冲和栅栏。
        unsafe {
            self.device
                .wait_for_fences(&[in_flight_fence], true, u64::MAX)?
        };

        // 获取下一张可用的交换链图像；成功后 image_available 信号量会被发出。
        let (image_index, acquire_suboptimal) = match unsafe {
            self.swapchain.loader.acquire_next_image(
                self.swapchain.handle,
                u64::MAX,
                self.sync.image_available(frame),
                vk::Fence::null(),
            )
        } {
            Ok(result) => result,
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                self.recreate_swapchain()?;
                return Ok(());
            }
            Err(err) => return Err(err.into()),
        };
        let image_index = image_index as usize;

        // 呈现顺序不一定与获取顺序一致：这张图像可能还被更早的帧占用。
        let image_fence = self.sync.images_in_flight(image_index);
        if image_fence != vk::Fence::null() {
            unsafe {
                self.device
                    .wait_for_fences(&[image_fence], true, u64::MAX)?
            };
        }
        self.sync.set_image_in_flight(image_index, in_flight_fence);

        // ===== 计算提交：用游戏层提供的 deltaTime 更新粒子位置 =====
        self.compute.update_delta_time(frame, delta_time);

        let compute_command_buffer = self.compute.command_buffer(frame);
        unsafe {
            self.device.reset_command_buffer(
                compute_command_buffer,
                vk::CommandBufferResetFlags::empty(),
            )?
        };
        record_compute_command_buffer(
            &self.device,
            compute_command_buffer,
            self.compute.pipeline(),
            self.compute.pipeline_layout(),
            self.compute.descriptor_set(frame),
            (PARTICLE_COUNT as u32) / LOCAL_SIZE,
        )?;

        // 录制成功后才重置计算栅栏，失败时保持已发信号，下一帧不会死等。
        unsafe { self.device.reset_fences(&[compute_fence])? };

        let compute_signal_semaphores = [self.sync.compute_finished(frame)];
        let compute_command_buffers = [compute_command_buffer];
        let compute_submit_info = vk::SubmitInfo::default()
            .command_buffers(&compute_command_buffers)
            .signal_semaphores(&compute_signal_semaphores);
        unsafe {
            self.device
                .queue_submit(self.graphics_queue, &[compute_submit_info], compute_fence)?
        };

        // 这一帧的栅栏已经等过，说明 GPU 不再读它专属的 UBO，可以安全写入新 MVP。
        self.uniform_buffers.update(frame, uniforms);

        // 录制这一帧的命令缓冲。
        let command_buffer = self.command_buffers[frame];
        unsafe {
            self.device
                .reset_command_buffer(command_buffer, vk::CommandBufferResetFlags::empty())?
        };
        {
            let draw_state = DrawState {
                pipeline: &self.graphics_pipeline,
                descriptor_set: self.descriptor_sets[frame],
                geometry: &self.geometry,
                particle_pipeline: &self.particle_pipeline,
                particle_buffer: self.compute.particle_buffer(frame),
                particle_count: if self.particles_enabled {
                    PARTICLE_COUNT as u32
                } else {
                    0
                },
            };
            record_command_buffer(
                &self.device,
                command_buffer,
                self.render_pass,
                self.framebuffers[image_index],
                &draw_state,
                self.swapchain.extent,
            )?;
        }

        // 录制成功后才重置栅栏：若前面的步骤失败，栅栏保持已发信号，下一帧不会死等。
        unsafe { self.device.reset_fences(&[in_flight_fence])? };

        // 图形提交等待两件事：计算完成（顶点输入阶段读取粒子缓冲）
        // 和交换链图像可用（颜色输出阶段）。
        let wait_semaphores = [
            self.sync.compute_finished(frame),
            self.sync.image_available(frame),
        ];
        let wait_stages = [
            vk::PipelineStageFlags::VERTEX_INPUT,
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
        ];
        let signal_semaphores = [self.sync.render_finished(image_index)];
        let command_buffers = [command_buffer];
        let submit_info = vk::SubmitInfo::default()
            .wait_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&wait_stages)
            .command_buffers(&command_buffers)
            .signal_semaphores(&signal_semaphores);
        unsafe {
            self.device
                .queue_submit(self.graphics_queue, &[submit_info], in_flight_fence)?
        };

        let swapchains = [self.swapchain.handle];
        let image_indices = [image_index as u32];
        let present_info = vk::PresentInfoKHR::default()
            .wait_semaphores(&signal_semaphores)
            .swapchains(&swapchains)
            .image_indices(&image_indices);
        let present_result = unsafe {
            self.swapchain
                .loader
                .queue_present(self.present_queue, &present_info)
        };

        let recreate = match present_result {
            Ok(suboptimal) => suboptimal || acquire_suboptimal || self.framebuffer_resized,
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => true,
            Err(err) => return Err(err.into()),
        };

        self.sync.advance_frame();

        if recreate {
            self.framebuffer_resized = false;
            self.recreate_swapchain()?;
        }

        Ok(())
    }

    /// 重建交换链及其图像视图、深度缓冲和帧缓冲（教程的 `recreateSwapChain`）。
    ///
    /// 先创建新资源、成功后再销毁旧资源，这样即使重建失败，旧交换链仍然可用。
    /// 渲染通道与图形管线按教程保持不变，因此要求交换链/深度格式稳定。
    fn recreate_swapchain(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        if self.window_width == 0 || self.window_height == 0 {
            return Ok(());
        }

        // 旧交换链的图像可能仍在使用，先等 GPU 空闲。
        unsafe { self.device.device_wait_idle()? };

        let new_swapchain = create_swapchain(
            &self.instance,
            &self.device,
            &self.surface_loader,
            self.surface,
            self.physical_device,
            SwapchainOptions {
                width: self.window_width,
                height: self.window_height,
                old_swapchain: self.swapchain.handle,
            },
        )?;

        // 渲染通道和管线是按旧格式创建的，格式变化时无法直接复用。
        if new_swapchain.format != self.swapchain.format {
            unsafe {
                new_swapchain
                    .loader
                    .destroy_swapchain(new_swapchain.handle, None)
            };
            return Err("交换链格式变化，无法复用现有渲染通道和图形管线".into());
        }

        // 深度缓冲尺寸跟着交换链走，格式/采样数保持与渲染通道一致。
        let new_depth = match DepthResources::create(
            &self.instance,
            &self.device,
            self.physical_device,
            new_swapchain.extent,
            self.depth.format(),
            self.msaa_samples,
        ) {
            Ok(depth) => depth,
            Err(err) => {
                unsafe {
                    new_swapchain
                        .loader
                        .destroy_swapchain(new_swapchain.handle, None)
                };
                return Err(err);
            }
        };

        // MSAA 颜色附件同样依赖交换链尺寸与格式。
        let new_msaa_color = match MsaaColor::create(
            &self.instance,
            &self.device,
            self.physical_device,
            new_swapchain.extent,
            new_swapchain.format,
            self.msaa_samples,
        ) {
            Ok(msaa_color) => msaa_color,
            Err(err) => {
                new_depth.destroy(&self.device);
                unsafe {
                    new_swapchain
                        .loader
                        .destroy_swapchain(new_swapchain.handle, None)
                };
                return Err(err);
            }
        };

        let new_image_views = match create_swapchain_image_views(
            &self.device,
            &new_swapchain.images,
            new_swapchain.format,
        ) {
            Ok(image_views) => image_views,
            Err(err) => {
                new_msaa_color.destroy(&self.device);
                new_depth.destroy(&self.device);
                unsafe {
                    new_swapchain
                        .loader
                        .destroy_swapchain(new_swapchain.handle, None)
                };
                return Err(err);
            }
        };

        let new_framebuffers = match create_framebuffers(
            &self.device,
            self.render_pass,
            new_msaa_color.view(),
            new_depth.view(),
            &new_image_views,
            new_swapchain.extent,
        ) {
            Ok(framebuffers) => framebuffers,
            Err(err) => {
                for image_view in new_image_views {
                    unsafe { self.device.destroy_image_view(image_view, None) };
                }
                new_msaa_color.destroy(&self.device);
                new_depth.destroy(&self.device);
                unsafe {
                    new_swapchain
                        .loader
                        .destroy_swapchain(new_swapchain.handle, None)
                };
                return Err(err);
            }
        };

        // 新资源全部就绪，销毁旧交换链及其派生对象（教程的 cleanupSwapChain）。
        self.cleanup_swapchain();

        self.swapchain = new_swapchain;
        self.image_views = new_image_views;
        self.msaa_color = new_msaa_color;
        self.depth = new_depth;
        self.framebuffers = new_framebuffers;

        // 旧交换链已销毁，换一批与新图像数量一致的呈现信号量并清空占用表。
        self.sync
            .recreate_image_semaphores(&self.device, self.swapchain.images.len())?;

        Ok(())
    }

    /// 销毁交换链及其图像视图、MSAA 颜色/深度缓冲和帧缓冲
    /// （教程的 `cleanupSwapChain`）。
    /// 重建交换链与最终析构共用；调用前必须确保 GPU 已经空闲。
    fn cleanup_swapchain(&mut self) {
        // 帧缓冲引用了 MSAA 颜色视图、深度视图和交换链图像视图，先销毁。
        destroy_framebuffers(&self.device, &mut self.framebuffers);

        // MSAA 颜色/深度缓冲只在当前交换链尺寸下有效。
        self.msaa_color.destroy(&self.device);
        self.depth.destroy(&self.device);

        for image_view in self.image_views.drain(..) {
            unsafe { self.device.destroy_image_view(image_view, None) };
        }

        unsafe {
            self.swapchain
                .loader
                .destroy_swapchain(self.swapchain.handle, None)
        };
    }
}

impl Drop for Render {
    fn drop(&mut self) {
        // 销毁任何对象前先确认 GPU 已经用完它们（教程 cleanup 里的 vkDeviceWaitIdle）。
        let _ = unsafe { self.device.device_wait_idle() };

        // 同步对象不再被任何提交使用。
        self.sync.destroy(&self.device);

        // 销毁描述符池会一并释放其中的描述符集。
        unsafe {
            self.device
                .destroy_descriptor_pool(self.descriptor_pool, None)
        };
        // 描述符集已经释放，纹理和 UBO 可以安全销毁。
        self.texture.destroy(&self.device);
        self.uniform_buffers.destroy(&self.device);

        // 顶点/索引缓冲先于命令池和设备销毁。
        self.geometry.destroy(&self.device);

        // 计算资源：描述符池、粒子 SSBO/UBO、计算管线与布局；
        // 计算命令缓冲随命令池一起销毁。
        self.compute.destroy(&self.device);

        // 命令缓冲随命令池一起销毁。
        unsafe { self.device.destroy_command_pool(self.command_pool, None) };

        // 管线引用管线布局与渲染通道，必须先于二者销毁。
        self.graphics_pipeline.destroy(&self.device);
        self.particle_pipeline.destroy(&self.device);

        // 管线布局已经销毁，现在才能销毁它引用的描述符集布局。
        unsafe {
            self.device
                .destroy_descriptor_set_layout(self.descriptor_set_layout, None)
        };

        unsafe { self.device.destroy_render_pass(self.render_pass, None) };

        // 交换链及其图像视图、帧缓冲（与重建共用的 cleanupSwapChain）。
        self.cleanup_swapchain();

        // 逻辑设备先于实例销毁（教程 cleanup 的顺序）。
        unsafe { self.device.destroy_device(None) };

        // 调试信使必须先于实例销毁（教程的测试错误就是漏掉这一步）。
        #[cfg(debug_assertions)]
        if let (Some(debug_utils), Some(messenger)) =
            (self.debug_utils.take(), self.debug_messenger.take())
        {
            unsafe { debug_utils.destroy_debug_utils_messenger(messenger, None) };
        }

        // 表面也必须在实例之前销毁。
        unsafe { self.surface_loader.destroy_surface(self.surface, None) };

        unsafe { self.instance.destroy_instance(None) };
    }
}
