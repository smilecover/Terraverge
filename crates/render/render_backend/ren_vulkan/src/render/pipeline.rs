//! 图形管线：管线布局、固定功能状态，以及最终的 `VkPipeline`。
//!
//! 对应教程的「Fixed functions」与「Depth buffering」章节。顶点数据来自顶点缓冲，
//! 顶点输入状态声明 binding 0 的位置/颜色/纹理坐标三个属性；管线布局包含
//! set 0 的 uniform buffer 与组合图像采样器描述符集。视口和裁剪矩形声明为
//! 动态状态，由命令缓冲每帧设置；深度测试开启，`LESS` 通过并写入深度；
//! 光栅化采样数与渲染通道的 MSAA 附件保持一致。

use ash::vk;
use std::error::Error;

use super::compute::Particle;
use super::shader_module::{ShaderModules, ShaderProgram};
use super::vertex::Vertex;

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

/// 创建管线布局（教程的 `createPipelineLayout`）。
/// 现在有一个描述符集布局（set 0 的 uniform buffer）；
/// push constant 暂时没有，`push_constant_ranges` 保持为空。
fn create_pipeline_layout(
    device: &ash::Device,
    descriptor_set_layout: vk::DescriptorSetLayout,
) -> Result<vk::PipelineLayout, vk::Result> {
    let set_layouts = [descriptor_set_layout];
    let create_info = vk::PipelineLayoutCreateInfo::default().set_layouts(&set_layouts);
    unsafe { device.create_pipeline_layout(&create_info, None) }
}

/// 创建图形管线（教程的 `createGraphicsPipeline`）。
/// 着色器模块在管线创建完成后立即销毁，不随 `Render` 长期保存。
pub(super) fn create_graphics_pipeline(
    device: &ash::Device,
    render_pass: vk::RenderPass,
    descriptor_set_layout: vk::DescriptorSetLayout,
    sample_count: vk::SampleCountFlags,
    program: &ShaderProgram<'_>,
) -> Result<GraphicsPipeline, Box<dyn Error + Send + Sync>> {
    let layout = create_pipeline_layout(device, descriptor_set_layout)?;

    let modules = match ShaderModules::create(device, program) {
        Ok(modules) => modules,
        Err(err) => {
            unsafe { device.destroy_pipeline_layout(layout, None) };
            return Err(err);
        }
    };

    let stages = modules.stages();

    // 顶点输入：binding 0 每个顶点前进 32 字节，location 0/1/2 分别对应
    // 三维位置、颜色和纹理坐标。
    let vertex_bindings = [Vertex::binding_description()];
    let vertex_attributes = Vertex::attribute_descriptions();
    let vertex_input = vk::PipelineVertexInputStateCreateInfo::default()
        .vertex_binding_descriptions(&vertex_bindings)
        .vertex_attribute_descriptions(&vertex_attributes);

    // 每三个顶点组成一个三角形。
    let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
        .primitive_restart_enable(false);

    // 视口和裁剪矩形是动态状态：管线里只声明数量，具体值在绘制时设置。
    let viewport_state = vk::PipelineViewportStateCreateInfo::default()
        .viewport_count(1)
        .scissor_count(1);

    // 光栅化：填充三角形。
    //
    // 方块网格在 CPU 侧已经剔除了不可见面，这里的背面剔除只会影响
    // “从内部看”的面。移植初期关闭背面剔除，避免顶点绕序与 Vulkan
    // 正面定义不一致时整片地形消失；后续用渲染测试固定绕序后再开启 BACK。
    let rasterization = vk::PipelineRasterizationStateCreateInfo::default()
        .depth_clamp_enable(false)
        .rasterizer_discard_enable(false)
        .polygon_mode(vk::PolygonMode::FILL)
        .line_width(1.0)
        .cull_mode(vk::CullModeFlags::NONE)
        .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
        .depth_bias_enable(false);

    // 光栅化采样数必须与渲染通道/附件一致；MSAA 由渲染通道 resolve。
    let multisampling = vk::PipelineMultisampleStateCreateInfo::default()
        .sample_shading_enable(false)
        .rasterization_samples(sample_count);

    // 颜色混合：标准 alpha 混合。不透明方块 alpha=1，结果与直接覆盖一致；
    // 玻璃/水等半透明贴图按来源 alpha 混合。M2 尚未做逐层排序，深度写入保持开启。
    let color_blend_attachment = vk::PipelineColorBlendAttachmentState::default()
        .blend_enable(true)
        .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
        .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
        .color_blend_op(vk::BlendOp::ADD)
        .src_alpha_blend_factor(vk::BlendFactor::ONE)
        .dst_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
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

    // 深度测试：更近的片元（深度更小）通过并写入深度缓冲。
    // 深度范围与 Vulkan 的 [0,1] 裁剪空间一致。
    let depth_stencil = vk::PipelineDepthStencilStateCreateInfo::default()
        .depth_test_enable(true)
        .depth_write_enable(true)
        .depth_compare_op(vk::CompareOp::LESS)
        .depth_bounds_test_enable(false)
        .min_depth_bounds(0.0)
        .max_depth_bounds(1.0);

    let pipeline_info = vk::GraphicsPipelineCreateInfo::default()
        .stages(&stages)
        .vertex_input_state(&vertex_input)
        .input_assembly_state(&input_assembly)
        .viewport_state(&viewport_state)
        .rasterization_state(&rasterization)
        .multisample_state(&multisampling)
        .depth_stencil_state(&depth_stencil)
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

/// 创建粒子点精灵管线（「计算着色器」章节的绘制部分）。
///
/// 与模型管线的区别：点图元、无描述符、开启 alpha 混合、关闭深度测试，
/// 这样计算生成的粒子会叠加显示在场景之上。
pub(super) fn create_particle_pipeline(
    device: &ash::Device,
    render_pass: vk::RenderPass,
    sample_count: vk::SampleCountFlags,
    program: &ShaderProgram<'_>,
) -> Result<GraphicsPipeline, Box<dyn Error + Send + Sync>> {
    // 粒子顶点着色器只读顶点属性，不需要任何描述符。
    let layout_info = vk::PipelineLayoutCreateInfo::default();
    let layout = unsafe { device.create_pipeline_layout(&layout_info, None)? };

    let modules = match ShaderModules::create(device, program) {
        Ok(modules) => modules,
        Err(err) => {
            unsafe { device.destroy_pipeline_layout(layout, None) };
            return Err(err);
        }
    };
    let stages = modules.stages();

    // 顶点输入与计算着色器使用的 Particle 结构一致，但只读取位置和颜色。
    let vertex_bindings = [Particle::binding_description()];
    let vertex_attributes = Particle::attribute_descriptions();
    let vertex_input = vk::PipelineVertexInputStateCreateInfo::default()
        .vertex_binding_descriptions(&vertex_bindings)
        .vertex_attribute_descriptions(&vertex_attributes);

    // 每个粒子一个点，点大小由顶点着色器的 PointSize 决定。
    let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
        .topology(vk::PrimitiveTopology::POINT_LIST)
        .primitive_restart_enable(false);

    let viewport_state = vk::PipelineViewportStateCreateInfo::default()
        .viewport_count(1)
        .scissor_count(1);

    let rasterization = vk::PipelineRasterizationStateCreateInfo::default()
        .depth_clamp_enable(false)
        .rasterizer_discard_enable(false)
        .polygon_mode(vk::PolygonMode::FILL)
        .line_width(1.0)
        .cull_mode(vk::CullModeFlags::NONE)
        .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
        .depth_bias_enable(false);

    let multisampling = vk::PipelineMultisampleStateCreateInfo::default()
        .sample_shading_enable(false)
        .rasterization_samples(sample_count);

    // 粒子使用 alpha 混合；片元着色器用 PointCoord 输出圆形软边。
    let color_blend_attachment = vk::PipelineColorBlendAttachmentState::default()
        .blend_enable(true)
        .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
        .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
        .color_blend_op(vk::BlendOp::ADD)
        .src_alpha_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
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

    // 渲染通道有深度附件，即使管线不用深度也必须提供状态结构；
    // 关闭深度测试/写入，粒子直接叠加到已绘制内容上。
    let depth_stencil = vk::PipelineDepthStencilStateCreateInfo::default()
        .depth_test_enable(false)
        .depth_write_enable(false)
        .depth_compare_op(vk::CompareOp::ALWAYS)
        .depth_bounds_test_enable(false);

    let pipeline_info = vk::GraphicsPipelineCreateInfo::default()
        .stages(&stages)
        .vertex_input_state(&vertex_input)
        .input_assembly_state(&input_assembly)
        .viewport_state(&viewport_state)
        .rasterization_state(&rasterization)
        .multisample_state(&multisampling)
        .depth_stencil_state(&depth_stencil)
        .color_blend_state(&color_blending)
        .dynamic_state(&dynamic_state)
        .layout(layout)
        .render_pass(render_pass)
        .subpass(0)
        .base_pipeline_handle(vk::Pipeline::null())
        .base_pipeline_index(-1);

    let result = unsafe {
        device.create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_info], None)
    };

    modules.destroy(device);

    match result {
        Ok(mut pipelines) => {
            let handle = pipelines.pop().expect("创建成功时必然返回一条管线");
            Ok(GraphicsPipeline { layout, handle })
        }
        Err((pipelines, err)) => {
            for pipeline in pipelines {
                unsafe { device.destroy_pipeline(pipeline, None) };
            }
            unsafe { device.destroy_pipeline_layout(layout, None) };
            Err(err.into())
        }
    }
}
