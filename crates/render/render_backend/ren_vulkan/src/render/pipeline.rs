//! 图形管线：管线布局、固定功能状态，以及最终的 `VkPipeline`。
//!
//! 对应教程的「Fixed functions」章节。当前三角形没有顶点缓冲、uniform 或纹理，
//! 所以管线布局是空的，顶点输入状态也是空的；视口和裁剪矩形声明为动态状态，
//! 等命令缓冲章节再每帧设置。

use ash::vk;
use std::error::Error;

use super::shader_module::{ShaderModules, ShaderProgram};

/// 图形管线及其布局。布局必须在管线之后销毁，所以把两者放在一起管理。
pub(super) struct GraphicsPipeline {
    pub(super) layout: vk::PipelineLayout,
    pub(super) handle: vk::Pipeline,
}

impl GraphicsPipeline {
    /// 先销毁管线，再销毁它引用的布局。
    pub(super) fn destroy(&self, device: &ash::Device) {
        unsafe {
            device.destroy_pipeline(self.handle, None);
            device.destroy_pipeline_layout(self.layout, None);
        }
    }
}

/// 创建空管线布局（教程的 `createPipelineLayout`）。
/// 以后加 uniform/纹理时会在这里填 `set_layouts` 和 `push_constant_ranges`。
fn create_pipeline_layout(device: &ash::Device) -> Result<vk::PipelineLayout, vk::Result> {
    let create_info = vk::PipelineLayoutCreateInfo::default();
    unsafe { device.create_pipeline_layout(&create_info, None) }
}

/// 创建图形管线（教程的 `createGraphicsPipeline`）。
/// 着色器模块在管线创建完成后立即销毁，不随 `Render` 长期保存。
pub(super) fn create_graphics_pipeline(
    device: &ash::Device,
    render_pass: vk::RenderPass,
    program: &ShaderProgram<'_>,
) -> Result<GraphicsPipeline, Box<dyn Error + Send + Sync>> {
    let layout = create_pipeline_layout(device)?;

    let modules = match ShaderModules::create(device, program) {
        Ok(modules) => modules,
        Err(err) => {
            unsafe { device.destroy_pipeline_layout(layout, None) };
            return Err(err);
        }
    };

    let stages = modules.stages();

    // 暂时没有顶点缓冲：所有顶点数据都写死在着色器里。
    let vertex_input = vk::PipelineVertexInputStateCreateInfo::default();

    // 每三个顶点组成一个三角形。
    let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
        .primitive_restart_enable(false);

    // 视口和裁剪矩形是动态状态：管线里只声明数量，具体值在绘制时设置。
    let viewport_state = vk::PipelineViewportStateCreateInfo::default()
        .viewport_count(1)
        .scissor_count(1);

    // 光栅化：填充三角形，剔除背面（教程的顶点顺序是顺时针，所以正面为 CLOCKWISE）。
    let rasterization = vk::PipelineRasterizationStateCreateInfo::default()
        .depth_clamp_enable(false)
        .rasterizer_discard_enable(false)
        .polygon_mode(vk::PolygonMode::FILL)
        .line_width(1.0)
        .cull_mode(vk::CullModeFlags::BACK)
        .front_face(vk::FrontFace::CLOCKWISE)
        .depth_bias_enable(false);

    // 暂不启用多重采样。
    let multisampling = vk::PipelineMultisampleStateCreateInfo::default()
        .sample_shading_enable(false)
        .rasterization_samples(vk::SampleCountFlags::TYPE_1);

    // 颜色混合：不混合，直接把片元颜色写到附件；但必须允许写 RGBA 四个通道。
    let color_blend_attachment = vk::PipelineColorBlendAttachmentState::default()
        .blend_enable(false)
        .src_color_blend_factor(vk::BlendFactor::ONE)
        .dst_color_blend_factor(vk::BlendFactor::ZERO)
        .color_blend_op(vk::BlendOp::ADD)
        .src_alpha_blend_factor(vk::BlendFactor::ONE)
        .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
        .alpha_blend_op(vk::BlendOp::ADD)
        .color_write_mask(vk::ColorComponentFlags::RGBA);
    let color_blend_attachments = [color_blend_attachment];
    let color_blending = vk::PipelineColorBlendStateCreateInfo::default()
        .logic_op_enable(false)
        .logic_op(vk::LogicOp::COPY)
        .attachments(&color_blend_attachments);

    let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
    let dynamic_state =
        vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);

    let pipeline_info = vk::GraphicsPipelineCreateInfo::default()
        .stages(&stages)
        .vertex_input_state(&vertex_input)
        .input_assembly_state(&input_assembly)
        .viewport_state(&viewport_state)
        .rasterization_state(&rasterization)
        .multisample_state(&multisampling)
        .color_blend_state(&color_blending)
        .dynamic_state(&dynamic_state)
        .layout(layout)
        .render_pass(render_pass)
        .subpass(0)
        .base_pipeline_handle(vk::Pipeline::null())
        .base_pipeline_index(-1);

    // 管线缓存为空即可；目前只创建一条管线，也没有派生关系。
    let result = unsafe {
        device.create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_info], None)
    };

    // 无论成功与否，SPIR-V 已经进入管线，着色器模块都可以销毁了。
    modules.destroy(device);

    match result {
        Ok(mut pipelines) => {
            let handle = pipelines.pop().expect("创建成功时必然返回一条管线");
            Ok(GraphicsPipeline { layout, handle })
        }
        Err((pipelines, err)) => {
            // 管线创建失败时驱动可能已经创建了部分管线，也要一并回收。
            for pipeline in pipelines {
                unsafe { device.destroy_pipeline(pipeline, None) };
            }
            unsafe { device.destroy_pipeline_layout(layout, None) };
            Err(err.into())
        }
    }
}
