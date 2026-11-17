//! 渲染通道：描述一次渲染要使用哪些附件、如何加载/存储，以及子通道之间的依赖。
//!
//! 对应教程的「Render passes」章节。当前只有一个颜色附件：
//! 每帧开始时清空，渲染结束后保留给交换链呈现。

use ash::vk;

/// 创建只含一个颜色附件的渲染通道（教程的 `createRenderPass`）。
pub(super) fn create_render_pass(
    device: &ash::Device,
    format: vk::Format,
) -> Result<vk::RenderPass, vk::Result> {
    // 附件描述：交换链的每张图像在渲染期间都是颜色附件。
    let color_attachment = vk::AttachmentDescription::default()
        .format(format)
        .samples(vk::SampleCountFlags::TYPE_1)
        // 不清空的话，上一帧内容会残留；这里每帧都用 clear color 覆盖。
        .load_op(vk::AttachmentLoadOp::CLEAR)
        // 渲染结果需要保留下来，随后交给交换链呈现。
        .store_op(vk::AttachmentStoreOp::STORE)
        // 没有模板附件，模板操作取什么都没关系。
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        // 图像内容在渲染前不需要保留，UNDEFINED 让驱动可以丢弃旧数据。
        .initial_layout(vk::ImageLayout::UNDEFINED)
        // 渲染结束时转到“呈现”布局，交换链才能把图像显示到窗口上。
        .final_layout(vk::ImageLayout::PRESENT_SRC_KHR);

    // 子通道引用附件 0，并在渲染期间把它当作颜色附件使用。
    let color_attachment_ref = vk::AttachmentReference::default()
        .attachment(0)
        .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

    let color_attachments = [color_attachment_ref];
    let subpass = vk::SubpassDescription::default()
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
        .color_attachments(&color_attachments);

    // 子通道依赖：等交换链图像可用、颜色附件写入完成后，再进入子通道。
    // 教程里 srcAccessMask 为 0，表示不需要额外等待某类内存访问。
    let dependency = vk::SubpassDependency::default()
        .src_subpass(vk::SUBPASS_EXTERNAL)
        .dst_subpass(0)
        .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
        .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
        .src_access_mask(vk::AccessFlags::empty())
        .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE);

    let attachments = [color_attachment];
    let subpasses = [subpass];
    let dependencies = [dependency];

    let create_info = vk::RenderPassCreateInfo::default()
        .attachments(&attachments)
        .subpasses(&subpasses)
        .dependencies(&dependencies);

    unsafe { device.create_render_pass(&create_info, None) }
}
