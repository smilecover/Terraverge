//! 帧缓冲：把一张交换链图像视图绑定到渲染通道的附件上。
//!
//! 对应教程的「Framebuffers」章节。每张交换链图像都需要一个帧缓冲，
//! 绘制时按 `vkAcquireNextImageKHR` 返回的图像索引选择对应的那一个。

use ash::vk;
use std::error::Error;

/// 为每张交换链图像创建帧缓冲（教程的 `createFramebuffers`）。
pub(super) fn create_framebuffers(
    device: &ash::Device,
    render_pass: vk::RenderPass,
    image_views: &[vk::ImageView],
    extent: vk::Extent2D,
) -> Result<Vec<vk::Framebuffer>, Box<dyn Error + Send + Sync>> {
    let mut framebuffers = Vec::with_capacity(image_views.len());

    for &image_view in image_views {
        // 渲染通道只有一个颜色附件，直接绑定这张图像的视图。
        let attachments = [image_view];
        let create_info = vk::FramebufferCreateInfo::default()
            .render_pass(render_pass)
            .attachments(&attachments)
            .width(extent.width)
            .height(extent.height)
            .layers(1);

        match unsafe { device.create_framebuffer(&create_info, None) } {
            Ok(framebuffer) => framebuffers.push(framebuffer),
            Err(err) => {
                // 中途失败时回收已经创建好的帧缓冲。
                destroy_framebuffers(device, &mut framebuffers);
                return Err(err.into());
            }
        }
    }

    Ok(framebuffers)
}

/// 销毁全部帧缓冲。重建交换链与 `Render` 析构时共用。
pub(super) fn destroy_framebuffers(device: &ash::Device, framebuffers: &mut Vec<vk::Framebuffer>) {
    for framebuffer in framebuffers.drain(..) {
        unsafe { device.destroy_framebuffer(framebuffer, None) };
    }
}
