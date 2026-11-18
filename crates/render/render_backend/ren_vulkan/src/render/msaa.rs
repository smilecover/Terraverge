//! 多重采样颜色附件。
//!
//! 对应教程「Multisampling」章节。场景先渲染到多采样颜色图像，再由渲染通道的
//! resolve 附件解析到单采样的交换链图像。多采样图像只作为瞬态附件使用，
//! 渲染结束即可丢弃，因此不需要 STORE，也不能被采样。

use ash::vk;
use std::error::Error;

use super::buffer::find_memory_type;

/// 多采样颜色图像及其视图，尺寸与交换链一致。
pub(super) struct MsaaColor {
    image: vk::Image,
    memory: vk::DeviceMemory,
    view: vk::ImageView,
}

impl MsaaColor {
    /// 创建多采样颜色附件（教程的 createColorResources）。
    pub(super) fn create(
        instance: &ash::Instance,
        device: &ash::Device,
        physical_device: vk::PhysicalDevice,
        extent: vk::Extent2D,
        color_format: vk::Format,
        sample_count: vk::SampleCountFlags,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let create_info = vk::ImageCreateInfo::default()
            .image_type(vk::ImageType::TYPE_2D)
            .format(color_format)
            .extent(vk::Extent3D {
                width: extent.width,
                height: extent.height,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .samples(sample_count)
            .tiling(vk::ImageTiling::OPTIMAL)
            // 瞬态附件：只在渲染通道内部作为颜色附件使用。
            .usage(
                vk::ImageUsageFlags::TRANSIENT_ATTACHMENT | vk::ImageUsageFlags::COLOR_ATTACHMENT,
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
                return Err("找不到满足 MSAA 颜色附件要求的显存类型".into());
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

        let view_info = vk::ImageViewCreateInfo::default()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(color_format)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
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
        })
    }

    pub(super) fn view(&self) -> vk::ImageView {
        self.view
    }

    pub(super) fn destroy(&self, device: &ash::Device) {
        unsafe { device.destroy_image_view(self.view, None) };
        destroy_image(device, self.image, self.memory);
    }
}

/// 选择设备支持的最高采样数（教程的 getMaxUsableSampleCount）。
/// 颜色和深度都必须支持该采样数，否则渲染通道无法同时使用两者。
pub(super) fn find_sample_count(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
) -> vk::SampleCountFlags {
    let properties = unsafe { instance.get_physical_device_properties(physical_device) };
    let counts = properties.limits.framebuffer_color_sample_counts
        & properties.limits.framebuffer_depth_sample_counts;

    [
        vk::SampleCountFlags::TYPE_64,
        vk::SampleCountFlags::TYPE_32,
        vk::SampleCountFlags::TYPE_16,
        vk::SampleCountFlags::TYPE_8,
        vk::SampleCountFlags::TYPE_4,
        vk::SampleCountFlags::TYPE_2,
    ]
    .into_iter()
    .find(|count| counts.contains(*count))
    .unwrap_or(vk::SampleCountFlags::TYPE_1)
}

fn destroy_image(device: &ash::Device, image: vk::Image, memory: vk::DeviceMemory) {
    unsafe {
        device.destroy_image(image, None);
        device.free_memory(memory, None);
    }
}
