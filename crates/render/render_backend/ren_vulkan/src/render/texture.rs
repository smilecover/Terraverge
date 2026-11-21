//! 纹理图像、图像视图、采样器与 mipmap 生成。
//!
//! 对应教程「Images」「Image view and sampler」「Combined image sampler」
//! 「Generating mipmaps」章节。流程：创建带完整 mip 链的 `VkImage` 并分配
//! 设备本地内存 → 用暂存缓冲上传第 0 级 → `vkCmdBlitImage` 逐级生成 mipmap →
//! 全部转到 `SHADER_READ_ONLY_OPTIMAL` → 创建图像视图与采样器。
//! 采样器最终和图像视图一起写入 set 0 / binding 1 的组合图像采样器描述符。

use ash::vk;
use std::error::Error;

use super::buffer::{Buffer, create_staging_buffer, find_memory_type};
use super::command_buffer::submit_one_time;

/// 纹理格式：每通道 8 位、带 sRGB 转换，与游戏层统一使用的 RGBA8 对应。
const TEXTURE_FORMAT: vk::Format = vk::Format::R8G8B8A8_SRGB;

/// 纹理采样过滤方式。Minecraft 的方块图集需要最近邻采样。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextureFilter {
    Nearest,
    Linear,
}

/// 游戏层传入的 RGBA8 像素数据。
pub struct TextureData<'a> {
    pub width: u32,
    pub height: u32,
    /// 紧凑排列，每像素 4 字节：R、G、B、A。
    pub pixels: &'a [u8],
    pub filter: TextureFilter,
    /// 是否生成完整 mip 链。像素风图集通常关闭，避免跨贴图渗色。
    pub generate_mipmaps: bool,
}

/// 一张上传完成的纹理：图像、内存、视图和采样器。
pub(super) struct Texture {
    image: vk::Image,
    memory: vk::DeviceMemory,
    view: vk::ImageView,
    sampler: vk::Sampler,
}

impl Texture {
    /// 创建纹理并完成上传（教程的 createTextureImage +
    /// createTextureImageView + createTextureSampler）。
    pub(super) fn create(
        instance: &ash::Instance,
        device: &ash::Device,
        physical_device: vk::PhysicalDevice,
        command_pool: vk::CommandPool,
        graphics_queue: vk::Queue,
        data: &TextureData<'_>,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        if data.width == 0 || data.height == 0 {
            return Err("纹理尺寸不能为 0".into());
        }

        let expected = data.width as usize * data.height as usize * 4;
        if data.pixels.len() != expected {
            return Err(format!(
                "纹理像素数量不匹配：期望 {expected} 字节，实际 {} 字节",
                data.pixels.len()
            )
            .into());
        }

        let mip_levels = if data.generate_mipmaps {
            mip_level_count(data.width, data.height)
        } else {
            1
        };

        // vkCmdBlitImage 的线性缩放在部分格式上不可用，先确认该格式支持。
        if data.generate_mipmaps {
            let format_properties = unsafe {
                instance.get_physical_device_format_properties(physical_device, TEXTURE_FORMAT)
            };
            if !format_properties
                .optimal_tiling_features
                .contains(vk::FormatFeatureFlags::SAMPLED_IMAGE_FILTER_LINEAR)
            {
                return Err("纹理格式不支持线性过滤，无法生成 mipmap".into());
            }
        }

        let device_properties = unsafe { instance.get_physical_device_properties(physical_device) };
        let max_anisotropy = device_properties.limits.max_sampler_anisotropy;

        let (image, memory) = create_image(
            instance,
            device,
            physical_device,
            data.width,
            data.height,
            mip_levels,
        )?;

        // 像素先写进 CPU 可见的暂存缓冲，再复制到设备本地图像。
        let staging = match create_staging_buffer(instance, device, physical_device, data.pixels) {
            Ok(staging) => staging,
            Err(err) => {
                destroy_image(device, image, memory);
                return Err(err);
            }
        };

        if let Err(err) = upload_image(
            device,
            command_pool,
            graphics_queue,
            &staging,
            image,
            data.width,
            data.height,
            mip_levels,
        ) {
            staging.destroy(device);
            destroy_image(device, image, memory);
            return Err(err);
        }
        staging.destroy(device);

        let view = match create_image_view(device, image, mip_levels) {
            Ok(view) => view,
            Err(err) => {
                destroy_image(device, image, memory);
                return Err(err.into());
            }
        };

        let sampler = match create_sampler(device, max_anisotropy, mip_levels, data.filter) {
            Ok(sampler) => sampler,
            Err(err) => {
                unsafe { device.destroy_image_view(view, None) };
                destroy_image(device, image, memory);
                return Err(err.into());
            }
        };

        Ok(Self {
            image,
            memory,
            view,
            sampler,
        })
    }

    pub(super) fn view(&self) -> vk::ImageView {
        self.view
    }

    pub(super) fn sampler(&self) -> vk::Sampler {
        self.sampler
    }

    /// 与创建顺序相反：采样器 → 图像视图 → 图像 → 内存。
    pub(super) fn destroy(&self, device: &ash::Device) {
        unsafe {
            device.destroy_sampler(self.sampler, None);
            device.destroy_image_view(self.view, None);
            destroy_image(device, self.image, self.memory);
        }
    }
}

/// 创建 2D 纹理图像并绑定设备本地内存（教程的 createImage）。
fn create_image(
    instance: &ash::Instance,
    device: &ash::Device,
    physical_device: vk::PhysicalDevice,
    width: u32,
    height: u32,
    mip_levels: u32,
) -> Result<(vk::Image, vk::DeviceMemory), Box<dyn Error + Send + Sync>> {
    // 完整 mip 链 + 单层 + 单采样；用途是传输源/目标以及着色器采样。
    let create_info = vk::ImageCreateInfo::default()
        .image_type(vk::ImageType::TYPE_2D)
        .format(TEXTURE_FORMAT)
        .extent(vk::Extent3D {
            width,
            height,
            depth: 1,
        })
        .mip_levels(mip_levels)
        .array_layers(1)
        .samples(vk::SampleCountFlags::TYPE_1)
        .tiling(vk::ImageTiling::OPTIMAL)
        .usage(
            vk::ImageUsageFlags::TRANSFER_SRC
                | vk::ImageUsageFlags::TRANSFER_DST
                | vk::ImageUsageFlags::SAMPLED,
        )
        .sharing_mode(vk::SharingMode::EXCLUSIVE)
        .initial_layout(vk::ImageLayout::UNDEFINED);

    let image = unsafe { device.create_image(&create_info, None)? };

    let requirements = unsafe { device.get_image_memory_requirements(image) };
    let memory_properties =
        unsafe { instance.get_physical_device_memory_properties(physical_device) };

    let memory_type_index = match find_memory_type(
        &memory_properties,
        requirements.memory_type_bits,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    ) {
        Some(index) => index,
        None => {
            unsafe { device.destroy_image(image, None) };
            return Err("找不到满足纹理要求的显存类型".into());
        }
    };

    let allocate_info = vk::MemoryAllocateInfo::default()
        .allocation_size(requirements.size)
        .memory_type_index(memory_type_index);

    let memory = match unsafe { device.allocate_memory(&allocate_info, None) } {
        Ok(memory) => memory,
        Err(err) => {
            unsafe { device.destroy_image(image, None) };
            return Err(err.into());
        }
    };

    if let Err(err) = unsafe { device.bind_image_memory(image, memory, 0) } {
        unsafe {
            device.free_memory(memory, None);
            device.destroy_image(image, None);
        }
        return Err(err.into());
    }

    Ok((image, memory))
}

/// 上传第 0 级并生成完整 mip 链（教程的 copyBufferToImage + generateMipmaps）。
fn upload_image(
    device: &ash::Device,
    command_pool: vk::CommandPool,
    graphics_queue: vk::Queue,
    staging: &Buffer,
    image: vk::Image,
    width: u32,
    height: u32,
    mip_levels: u32,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    submit_one_time(
        device,
        command_pool,
        graphics_queue,
        |command_buffer| unsafe {
            // 1. 第 0 级：图像内容未定义 → 传输目标布局。
            transition_image_layout(
                device,
                command_buffer,
                image,
                vk::ImageLayout::UNDEFINED,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::TRANSFER,
                vk::AccessFlags::empty(),
                vk::AccessFlags::TRANSFER_WRITE,
                0,
                1,
            );

            // 2. 缓冲 → 图像复制，只复制第 0 级；其余级别由 blit 生成。
            let region = vk::BufferImageCopy::default()
                .image_subresource(vk::ImageSubresourceLayers {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    mip_level: 0,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .image_extent(vk::Extent3D {
                    width,
                    height,
                    depth: 1,
                });
            device.cmd_copy_buffer_to_image(
                command_buffer,
                staging.handle,
                image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[region],
            );

            // 3. 逐级 blit 生成 mipmap，并把每一级转到着色器只读布局。
            generate_mipmaps(device, command_buffer, image, width, height, mip_levels);
        },
    )
}

/// 用 `vkCmdBlitImage` 逐级缩小生成 mipmap（教程的 generateMipmaps）。
/// 进入时第 0 级处于 TRANSFER_DST_OPTIMAL；返回时所有级别都是
/// SHADER_READ_ONLY_OPTIMAL。
unsafe fn generate_mipmaps(
    device: &ash::Device,
    command_buffer: vk::CommandBuffer,
    image: vk::Image,
    width: u32,
    height: u32,
    mip_levels: u32,
) {
    let mut mip_width = width as i32;
    let mut mip_height = height as i32;

    // 整个函数都在录制同一条已经 begin 的命令缓冲。
    unsafe {
        for level in 1..mip_levels {
            // 上一级：写入目标 → blit 源。
            transition_image_layout(
                device,
                command_buffer,
                image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::TRANSFER,
                vk::AccessFlags::TRANSFER_WRITE,
                vk::AccessFlags::TRANSFER_READ,
                level - 1,
                1,
            );

            // 当前级此前从未使用（UNDEFINED），先转到传输目标布局。
            // 教程的 generateMipmaps 没有显式做这一步，但校验层要求
            // vkCmdBlitImage 的目标子资源已经处于 dstImageLayout。
            transition_image_layout(
                device,
                command_buffer,
                image,
                vk::ImageLayout::UNDEFINED,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::TRANSFER,
                vk::AccessFlags::empty(),
                vk::AccessFlags::TRANSFER_WRITE,
                level,
                1,
            );

            // 把上一级线性缩放复制到当前级；尺寸减半（最小为 1）。
            let blit = vk::ImageBlit::default()
                .src_subresource(mip_subresource(level - 1))
                .dst_subresource(mip_subresource(level))
                .src_offsets([
                    vk::Offset3D { x: 0, y: 0, z: 0 },
                    vk::Offset3D {
                        x: mip_width,
                        y: mip_height,
                        z: 1,
                    },
                ])
                .dst_offsets([
                    vk::Offset3D { x: 0, y: 0, z: 0 },
                    vk::Offset3D {
                        x: (mip_width / 2).max(1),
                        y: (mip_height / 2).max(1),
                        z: 1,
                    },
                ]);

            device.cmd_blit_image(
                command_buffer,
                image,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[blit],
                vk::Filter::LINEAR,
            );

            // 上一级已经不再作为源，交给着色器读取。
            transition_image_layout(
                device,
                command_buffer,
                image,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
                vk::AccessFlags::TRANSFER_READ,
                vk::AccessFlags::SHADER_READ,
                level - 1,
                1,
            );

            mip_width = (mip_width / 2).max(1);
            mip_height = (mip_height / 2).max(1);
        }

        // 最后一级仍停在 TRANSFER_DST，转到着色器只读。
        // 只有一级时，这里处理的正是刚上传的第 0 级。
        transition_image_layout(
            device,
            command_buffer,
            image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
            vk::AccessFlags::TRANSFER_WRITE,
            vk::AccessFlags::SHADER_READ,
            mip_levels - 1,
            1,
        );
    }
}

/// 完整 mip 链长度：floor(log2(max(width, height))) + 1。
fn mip_level_count(width: u32, height: u32) -> u32 {
    (width.max(height) as f32).log2().floor() as u32 + 1
}

/// blit 源/目标使用的子资源：颜色方面、指定 mip、单层。
fn mip_subresource(mip_level: u32) -> vk::ImageSubresourceLayers {
    vk::ImageSubresourceLayers {
        aspect_mask: vk::ImageAspectFlags::COLOR,
        mip_level,
        base_array_layer: 0,
        layer_count: 1,
    }
}

/// 插入一次图像布局转换屏障（教程的 transitionImageLayout）。
/// 阶段与访问掩码由调用方显式给出；`mip` 范围允许逐级处理 mipmap。
#[allow(clippy::too_many_arguments)]
unsafe fn transition_image_layout(
    device: &ash::Device,
    command_buffer: vk::CommandBuffer,
    image: vk::Image,
    old_layout: vk::ImageLayout,
    new_layout: vk::ImageLayout,
    src_stage: vk::PipelineStageFlags,
    dst_stage: vk::PipelineStageFlags,
    src_access: vk::AccessFlags,
    dst_access: vk::AccessFlags,
    base_mip_level: u32,
    level_count: u32,
) {
    let barrier = vk::ImageMemoryBarrier::default()
        .old_layout(old_layout)
        .new_layout(new_layout)
        // 队列族忽略表示图像只属于当前队列族，无需转移所有权。
        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .image(image)
        .subresource_range(vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level,
            level_count,
            base_array_layer: 0,
            layer_count: 1,
        })
        .src_access_mask(src_access)
        .dst_access_mask(dst_access);

    unsafe {
        device.cmd_pipeline_barrier(
            command_buffer,
            src_stage,
            dst_stage,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &[barrier],
        )
    };
}

/// 创建纹理图像视图（教程的 createTextureImageView）。
/// 视图必须覆盖完整 mip 链，采样器才能访问所有级别。
fn create_image_view(
    device: &ash::Device,
    image: vk::Image,
    mip_levels: u32,
) -> Result<vk::ImageView, vk::Result> {
    let create_info = vk::ImageViewCreateInfo::default()
        .image(image)
        .view_type(vk::ImageViewType::TYPE_2D)
        .format(TEXTURE_FORMAT)
        .subresource_range(vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: mip_levels,
            base_array_layer: 0,
            layer_count: 1,
        });

    unsafe { device.create_image_view(&create_info, None) }
}

/// 创建采样器（教程的 createTextureSampler）：线性过滤、UV 重复、
/// 各向异性过滤，`max_lod` 覆盖整条 mip 链。
fn create_sampler(
    device: &ash::Device,
    max_anisotropy: f32,
    mip_levels: u32,
    filter: TextureFilter,
) -> Result<vk::Sampler, vk::Result> {
    let vk_filter = match filter {
        TextureFilter::Nearest => vk::Filter::NEAREST,
        TextureFilter::Linear => vk::Filter::LINEAR,
    };
    let mipmap_mode = match filter {
        TextureFilter::Nearest => vk::SamplerMipmapMode::NEAREST,
        TextureFilter::Linear => vk::SamplerMipmapMode::LINEAR,
    };
    let create_info = vk::SamplerCreateInfo::default()
        .mag_filter(vk_filter)
        .min_filter(vk_filter)
        .mipmap_mode(mipmap_mode)
        // 图集坐标恰好落在边缘时应夹取，而不是循环到另一张贴图。
        .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
        .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
        .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE)
        // 没有 mip 链时各向异性过滤没有意义。
        .anisotropy_enable(mip_levels > 1 && filter == TextureFilter::Linear)
        .max_anisotropy(if mip_levels > 1 {
            max_anisotropy.max(1.0)
        } else {
            1.0
        })
        .compare_enable(false)
        .compare_op(vk::CompareOp::ALWAYS)
        .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
        .unnormalized_coordinates(false)
        .min_lod(0.0)
        .max_lod(mip_levels as f32);

    unsafe { device.create_sampler(&create_info, None) }
}

/// 销毁图像和它绑定的内存。
fn destroy_image(device: &ash::Device, image: vk::Image, memory: vk::DeviceMemory) {
    unsafe {
        device.destroy_image(image, None);
        device.free_memory(memory, None);
    }
}
