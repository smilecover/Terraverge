//! 图像视图：把交换链图像包装成可供渲染管线访问的 `VkImageView`。
//!
//! 图像本身不能直接用于渲染管线，必须先创建图像视图描述“如何解释和访问”
//! 这块图像数据（教程的 createImageViews）。

use ash::vk;
use std::error::Error;

/// 为单张图像创建 2D 颜色目标视图（教程循环体里的 `VkImageViewCreateInfo`）。
fn create_image_view(
    device: &ash::Device,
    image: vk::Image,
    format: vk::Format,
) -> Result<vk::ImageView, vk::Result> {
    // 交换链图像只有 1 层、1 个 mip 级别，作为颜色附件使用。
    // components 保持默认的 IDENTITY，即通道原样映射。
    let create_info = vk::ImageViewCreateInfo::default()
        .image(image)
        .view_type(vk::ImageViewType::TYPE_2D)
        .format(format)
        .components(vk::ComponentMapping::default())
        .subresource_range(vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        });

    unsafe { device.create_image_view(&create_info, None) }
}

/// 为交换链中的每张图像创建图像视图（教程的 createImageViews）。
/// 中途失败时回收已经创建好的视图，不留半初始化状态。
pub(super) fn create_swapchain_image_views(
    device: &ash::Device,
    images: &[vk::Image],
    format: vk::Format,
) -> Result<Vec<vk::ImageView>, Box<dyn Error + Send + Sync>> {
    let mut image_views = Vec::with_capacity(images.len());

    for &image in images {
        match create_image_view(device, image, format) {
            Ok(image_view) => image_views.push(image_view),
            Err(err) => {
                for image_view in image_views.drain(..) {
                    unsafe { device.destroy_image_view(image_view, None) };
                }
                return Err(err.into());
            }
        }
    }

    Ok(image_views)
}
