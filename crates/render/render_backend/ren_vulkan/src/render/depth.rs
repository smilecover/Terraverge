//! 深度缓冲：与交换链同尺寸的深度图像及其视图。
//!
//! 对应教程「Depth buffering」章节。深度图像不需要 CPU 上传，
//! 也不需要手动转换布局：渲染通道第一次使用时以 `loadOp = CLEAR`
//! 从 `UNDEFINED` 自动转到 `DEPTH_STENCIL_ATTACHMENT_OPTIMAL`。
//! 交换链重建（尺寸变化）时必须跟着重建。

use ash::vk;
use std::error::Error;

use super::buffer::find_memory_type;

/// 深度附件资源：图像、内存和图像视图。
pub(super) struct DepthResources {
    image: vk::Image,
    memory: vk::DeviceMemory,
    view: vk::ImageView,
    format: vk::Format,
}

impl DepthResources {
    /// 创建深度图像并绑定设备本地内存（教程的 createDepthResources）。
    /// `format` 由 `find_depth_format` 选出，与渲染通道使用的格式一致。
    pub(super) fn create(
        instance: &ash::Instance,
        device: &ash::Device,
        physical_device: vk::PhysicalDevice,
        extent: vk::Extent2D,
        format: vk::Format,
        sample_count: vk::SampleCountFlags,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let create_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(format)
            .extent(vk::Extent3D {
                width: extent.width,
                height: extent.height,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            // 深度附件必须与颜色附件使用相同的采样数。
            .samples(sample_count)
            .tiling(vk::ImageTiling::OPTIMAL)
            // 只作为深度附件使用，不需要采样或传输。
            .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
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
                return Err("找不到满足深度缓冲要求的显存类型".into());
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

        // 深度视图只取 DEPTH 方面；即使格式带 stencil 也不使用它。
        let view_info = vk::ImageViewCreateInfo::default()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(format)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::DEPTH,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });

        let view = match unsafe { device.create_image_view(&view_info, None) } {
            Ok(view) => view,
            Err(err) => {
                destroy_image(device, image, memory);
                return Err(err.into());
            }
        };

        Ok(Self {
            image,
            memory,
            view,
            format,
        })
    }

    pub(super) fn format(&self) -> vk::Format {
        self.format
    }

    pub(super) fn view(&self) -> vk::ImageView {
        self.view
    }

    pub(super) fn destroy(&self, device: &ash::Device) {
        unsafe { device.destroy_image_view(self.view, None) };
        destroy_image(device, self.image, self.memory);
    }
}

/// 选一个支持深度附件的格式（教程的 findDepthFormat）。
/// 优先 D32_SFLOAT；带 stencil 的格式作为后备，但本工程暂不使用 stencil。
pub(super) fn find_depth_format(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
) -> Result<vk::Format, Box<dyn Error + Send + Sync>> {
    let candidates = [
        vk::Format::D32_SFLOAT,
        vk::Format::D32_SFLOAT_S8_UINT,
        vk::Format::D24_UNORM_S8_UINT,
    ];

    candidates
        .into_iter()
        .find(|&format| {
            let properties =
                unsafe { instance.get_physical_device_format_properties(physical_device, format) };
            properties
                .optimal_tiling_features
                .contains(vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT)
        })
        .ok_or_else(|| "找不到支持深度附件的图像格式".into())
}

fn destroy_image(device: &ash::Device, image: vk::Image, memory: vk::DeviceMemory) {
    unsafe {
        device.destroy_image(image, None);
        device.free_memory(memory, None);
    }
}
