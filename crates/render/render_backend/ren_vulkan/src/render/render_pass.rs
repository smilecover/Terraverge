//! 渲染通道：描述一次渲染要使用哪些附件、如何加载/存储，以及子通道之间的依赖。
//!
//! 对应教程的「Render passes」「Depth buffering」「Multisampling」章节。
//! 现在有三个附件：
//! - 附件 0：多采样颜色图像（MSAA，渲染结束丢弃）；
//! - 附件 1：多采样深度图像（渲染结束丢弃）；
//! - 附件 2：单采样交换链图像，作为 resolve 目标接收解析结果并交给呈现。

use ash::vk;

/// 创建 MSAA 颜色 + 深度 + resolve 附件的渲染通道（教程的 `createRenderPass`）。
pub(super) fn create_render_pass(
    device: &ash::Device,
    color_format: vk::Format,
    depth_format: vk::Format,
    sample_count: vk::SampleCountFlags,
) -> Result<vk::RenderPass, vk::Result> {
    // 附件 0：多采样颜色缓冲。每帧清空，解析到交换链图像后不再需要保留。
    let color_attachment = vk::AttachmentDescription::default()
        .format(color_format)
        .samples(sample_count)
        // 不清空的话，上一帧内容会残留；这里每帧都用 clear color 覆盖。
        .load_op(vk::AttachmentLoadOp::CLEAR)
        // 内容会通过 resolve 写入附件 2，这里不需要保存。
        .store_op(vk::AttachmentStoreOp::DONT_CARE)
        // 没有模板附件，模板操作取什么都没关系。
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        // 图像内容在渲染前不需要保留，UNDEFINED 让驱动可以丢弃旧数据。
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

    // 附件 1：多采样深度缓冲。每帧清成 1.0，内容不需要保留。
    let depth_attachment = vk::AttachmentDescription::default()
        .format(depth_format)
        .samples(sample_count)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::DONT_CARE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);

    // 附件 2：单采样交换链图像，作为 resolve 目标；内容要保留给呈现。
    let resolve_attachment = vk::AttachmentDescription::default()
        .format(color_format)
        .samples(vk::SampleCountFlags::TYPE_1)
        .load_op(vk::AttachmentLoadOp::DONT_CARE)
        .store_op(vk::AttachmentStoreOp::STORE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::PRESENT_SRC_KHR);

    // 子通道引用附件 0 作为多采样颜色附件，附件 2 作为它的 resolve 目标，
    // 附件 1 作为深度附件。
    let color_attachment_ref = vk::AttachmentReference::default()
        .attachment(0)
        .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);
    let depth_attachment_ref = vk::AttachmentReference::default()
        .attachment(1)
        .layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);
    let resolve_attachment_ref = vk::AttachmentReference::default()
        .attachment(2)
        .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

    let color_attachments = [color_attachment_ref];
    let resolve_attachments = [resolve_attachment_ref];
    let subpass = vk::SubpassDescription::default()
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
        .color_attachments(&color_attachments)
        .resolve_attachments(&resolve_attachments)
        .depth_stencil_attachment(&depth_attachment_ref);

    // 子通道依赖：等颜色输出和早期深度测试都可用后再进入子通道。
    // 教程里 srcAccessMask 为 0，表示外部不需要额外等待某类内存访问。
    let src_stages = vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
        | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS;
    let dependency = vk::SubpassDependency::default()
        .src_subpass(vk::SUBPASS_EXTERNAL)
        .dst_subpass(0)
        .src_stage_mask(src_stages)
        .dst_stage_mask(src_stages)
        .src_access_mask(vk::AccessFlags::empty())
        .dst_access_mask(
            vk::AccessFlags::COLOR_ATTACHMENT_WRITE
                | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
        );

    let attachments = [color_attachment, depth_attachment, resolve_attachment];
    let subpasses = [subpass];
    let dependencies = [dependency];

    let create_info = vk::RenderPassCreateInfo::default()
        .attachments(&attachments)
        .subpasses(&subpasses)
        .dependencies(&dependencies);

    unsafe { device.create_render_pass(&create_info, None) }
}
