//! 命令缓冲：创建命令池、分配命令缓冲，并把每帧的绘制命令录制进去。
//!
//! 对应教程的「Command buffers」章节。每个“在途帧”各有一个命令缓冲，
//! 每帧以 `RESET_COMMAND_BUFFER` 方式重置后重新录制，因此可以安全复用。

use ash::vk;

/// 创建命令池（教程的 `createCommandPool`）。
/// `RESET_COMMAND_BUFFER` 允许单独重置某条命令缓冲，正好对应每帧重录。
pub(super) fn create_command_pool(
    device: &ash::Device,
    graphics_family: u32,
) -> Result<vk::CommandPool, vk::Result> {
    let create_info = vk::CommandPoolCreateInfo::default()
        .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
        .queue_family_index(graphics_family);

    unsafe { device.create_command_pool(&create_info, None) }
}

/// 从命令池分配 `count` 条主命令缓冲（教程的 `createCommandBuffers`）。
pub(super) fn allocate_command_buffers(
    device: &ash::Device,
    command_pool: vk::CommandPool,
    count: usize,
) -> Result<Vec<vk::CommandBuffer>, vk::Result> {
    let create_info = vk::CommandBufferAllocateInfo::default()
        .command_pool(command_pool)
        .level(vk::CommandBufferLevel::PRIMARY)
        .command_buffer_count(count as u32);

    unsafe { device.allocate_command_buffers(&create_info) }
}

/// 录制一帧：开始渲染通道 → 绑定管线 → 设置动态视口/裁剪 → 画三角形 → 结束。
/// （教程的 `recordCommandBuffer`）
pub(super) fn record_command_buffer(
    device: &ash::Device,
    command_buffer: vk::CommandBuffer,
    render_pass: vk::RenderPass,
    framebuffer: vk::Framebuffer,
    pipeline: vk::Pipeline,
    extent: vk::Extent2D,
) -> Result<(), vk::Result> {
    let begin_info = vk::CommandBufferBeginInfo::default();
    unsafe { device.begin_command_buffer(command_buffer, &begin_info)? };

    // 渲染通道的 loadOp 是 CLEAR，这里给出清屏颜色（黑色）。
    let clear_values = [vk::ClearValue {
        color: vk::ClearColorValue {
            float32: [0.0, 0.0, 0.0, 1.0],
        },
    }];
    let render_area = vk::Rect2D {
        offset: vk::Offset2D::default(),
        extent,
    };
    let render_pass_info = vk::RenderPassBeginInfo::default()
        .render_pass(render_pass)
        .framebuffer(framebuffer)
        .render_area(render_area)
        .clear_values(&clear_values);

    unsafe {
        device.cmd_begin_render_pass(
            command_buffer,
            &render_pass_info,
            vk::SubpassContents::INLINE,
        );

        device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, pipeline);

        // 视口和裁剪矩形在管线里声明为动态状态，必须每帧设置。
        let viewport = vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: extent.width as f32,
            height: extent.height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        };
        device.cmd_set_viewport(command_buffer, 0, &[viewport]);

        let scissor = vk::Rect2D {
            offset: vk::Offset2D::default(),
            extent,
        };
        device.cmd_set_scissor(command_buffer, 0, &[scissor]);

        // 3 个顶点、1 个实例；顶点数据写死在着色器里。
        device.cmd_draw(command_buffer, 3, 1, 0, 0);

        device.cmd_end_render_pass(command_buffer);
    }

    unsafe { device.end_command_buffer(command_buffer) }
}
