//! 命令缓冲：创建命令池、分配命令缓冲，并把每帧的绘制命令录制进去。
//!
//! 对应教程的「Command buffers」章节。每个“在途帧”各有一个命令缓冲，
//! 每帧以 `RESET_COMMAND_BUFFER` 方式重置后重新录制，因此可以安全复用。

use ash::vk;
use std::error::Error;

use super::buffer::Geometry;
use super::pipeline::GraphicsPipeline;

/// 录制一次绘制需要的状态：模型管线、这一帧的 UBO 描述符集、几何缓冲，
/// 以及计算着色器生成的粒子点缓冲。
pub(super) struct DrawState<'a> {
    pub(super) pipeline: &'a GraphicsPipeline,
    pub(super) descriptor_set: vk::DescriptorSet,
    pub(super) geometry: &'a Geometry,
    pub(super) particle_pipeline: &'a GraphicsPipeline,
    pub(super) particle_buffer: vk::Buffer,
    pub(super) particle_count: u32,
}

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

/// 分配一条一次性命令缓冲，录制后提交并等待队列空闲。
/// 用于初始化阶段的缓冲/图像上传（教程的 `copyBuffer` / `transitionImageLayout`
/// 这类“录一次、立刻等完成”的操作）。
pub(super) fn submit_one_time(
    device: &ash::Device,
    command_pool: vk::CommandPool,
    graphics_queue: vk::Queue,
    record: impl FnOnce(vk::CommandBuffer),
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let allocate_info = vk::CommandBufferAllocateInfo::default()
        .level(vk::CommandBufferLevel::PRIMARY)
        .command_pool(command_pool)
        .command_buffer_count(1);
    let command_buffer = unsafe { device.allocate_command_buffers(&allocate_info)?[0] };

    let begin_info =
        vk::CommandBufferBeginInfo::default().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

    let result = (|| -> Result<(), vk::Result> {
        unsafe { device.begin_command_buffer(command_buffer, &begin_info)? };

        record(command_buffer);

        unsafe { device.end_command_buffer(command_buffer)? };

        let command_buffers = [command_buffer];
        let submit_info = vk::SubmitInfo::default().command_buffers(&command_buffers);
        unsafe {
            device.queue_submit(graphics_queue, &[submit_info], vk::Fence::null())?;
            device.queue_wait_idle(graphics_queue)?;
        }

        Ok(())
    })();

    // 一次性命令缓冲用完立即归还命令池。
    unsafe { device.free_command_buffers(command_pool, &[command_buffer]) };

    result.map_err(Into::into)
}

/// 录制一帧：开始渲染通道 → 绑定管线/描述符集/顶点缓冲/索引缓冲 →
/// 设置动态视口/裁剪 → 按索引画四边形 → 结束。（教程的 `recordCommandBuffer`）
pub(super) fn record_command_buffer(
    device: &ash::Device,
    command_buffer: vk::CommandBuffer,
    render_pass: vk::RenderPass,
    framebuffer: vk::Framebuffer,
    state: &DrawState<'_>,
    extent: vk::Extent2D,
) -> Result<(), vk::Result> {
    let begin_info = vk::CommandBufferBeginInfo::default();
    unsafe { device.begin_command_buffer(command_buffer, &begin_info)? };

    // 渲染通道的两个 loadOp 都是 CLEAR：
    // 颜色清成黑色，深度清成 1.0（Vulkan 中 1.0 是最远）。
    let clear_values = [
        vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 1.0],
            },
        },
        vk::ClearValue {
            depth_stencil: vk::ClearDepthStencilValue {
                depth: 1.0,
                stencil: 0,
            },
        },
    ];
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

        device.cmd_bind_pipeline(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            state.pipeline.handle,
        );

        // 绑定 set 0 的描述符集（里面是这一帧的 MVP uniform buffer）。
        device.cmd_bind_descriptor_sets(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            state.pipeline.layout,
            0,
            &[state.descriptor_set],
            &[],
        );

        // 绑定 binding 0 的顶点缓冲（偏移 0），以及 32 位索引缓冲。
        device.cmd_bind_vertex_buffers(
            command_buffer,
            0,
            &[state.geometry.vertex_buffer.handle],
            &[0],
        );
        device.cmd_bind_index_buffer(
            command_buffer,
            state.geometry.index_buffer.handle,
            0,
            vk::IndexType::UINT32,
        );

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

        // 按索引绘制：索引数量来自模型、1 个实例，索引从 0 开始，顶点偏移 0。
        device.cmd_draw_indexed(command_buffer, state.geometry.index_count, 1, 0, 0, 0);

        // 计算着色器生成的粒子缓冲同时是顶点缓冲，以点图元叠加绘制。
        // 粒子管线关闭了深度测试，并且使用 alpha 混合。
        device.cmd_bind_pipeline(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            state.particle_pipeline.handle,
        );
        device.cmd_bind_vertex_buffers(command_buffer, 0, &[state.particle_buffer], &[0]);
        device.cmd_draw(command_buffer, state.particle_count, 1, 0, 0);

        device.cmd_end_render_pass(command_buffer);
    }

    unsafe { device.end_command_buffer(command_buffer) }
}

/// 录制计算命令：绑定计算管线和描述符集，然后 `vkCmdDispatch`。
/// `group_count` 是工作组数量（粒子数 / local_size）。
pub(super) fn record_compute_command_buffer(
    device: &ash::Device,
    command_buffer: vk::CommandBuffer,
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    descriptor_set: vk::DescriptorSet,
    group_count: u32,
) -> Result<(), vk::Result> {
    let begin_info = vk::CommandBufferBeginInfo::default();
    unsafe {
        device.begin_command_buffer(command_buffer, &begin_info)?;

        device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::COMPUTE, pipeline);
        device.cmd_bind_descriptor_sets(
            command_buffer,
            vk::PipelineBindPoint::COMPUTE,
            pipeline_layout,
            0,
            &[descriptor_set],
            &[],
        );
        device.cmd_dispatch(command_buffer, group_count, 1, 1);

        device.end_command_buffer(command_buffer)
    }
}
