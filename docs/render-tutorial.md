# 从清屏框架到绘制三角形：分步教程

当前 `crates/render/render_backend/ren_vulkan/src/lib.rs` 已经是一个能正常运行的骨架：
创建 Vulkan 实例、设备、交换链、RenderPass、Framebuffer、命令缓冲、同步对象，
每帧获取图像 → 用 CLEAR 清屏 → 提交 → 呈现。

这份教程只讲思路、API 和关键代码骨架，具体代码建议你自己敲一遍。
每一步都有「检查点」，通过后再进入下一步。

---

## 路线图

1. 汇编着色器（slangc → SPIR-V）
2. 创建 ShaderModule
3. 创建 PipelineLayout
4. 创建 GraphicsPipeline
5. 在命令缓冲里绑定并绘制
6. 热重载（notify + 重建管线）
7. 进阶方向：顶点缓冲、UBO、深度缓冲

加绘制只需要改两个地方：`Render` 的字段/初始化/Drop，以及 `record_command_buffer`。
`render()`、交换链、同步、窗口缩放都不需要动。

---

## 第 1 步：汇编着色器

新建 `shaders/triangle.slang`（文件名随意，路径记录下来）：

```hlsl
struct VertexOutput
{
    float4 position : SV_Position;
    float3 color : COLOR0;
};

[shader("vertex")]
VertexOutput vs_main(uint vertexIndex : SV_VulkanVertexID)
{
    float2 positions[3] = {
        float2(0.0, -0.5),
        float2(0.5, 0.5),
        float2(-0.5, 0.5)
    };
    float3 colors[3] = {
        float3(1.0, 0.0, 0.0),
        float3(0.0, 1.0, 0.0),
        float3(0.0, 0.0, 1.0)
    };

    VertexOutput output;
    output.position = float4(positions[vertexIndex], 0.0, 1.0);
    output.color = colors[vertexIndex];
    return output;
}

[shader("fragment")]
float4 fs_main(VertexOutput input) : SV_Target
{
    return float4(input.color, 1.0);
}
```

新建 `src/shader.rs`，对外只暴露一个函数：

```rust
pub fn compile(source: &Path, entry: &str, stage: &str) -> Result<Vec<u32>>
```

实现要点：

1. 找 `slangc`：依次检查环境变量 `SLANGC`、`VULKAN_SDK/Bin/slangc.exe`、`PATH`。
2. 用 `std::process::Command` 调它，参数和之前 `build.rs` 里一样：
   `-entry <entry> -stage <stage> -target spirv -profile spirv_1_0 -fvk-use-entrypoint-name -O2 -o <临时spv>`。
3. 读取 `.spv` 文件，按小端把字节转成 `Vec<u32>`（SPIR-V 的长度必须是 4 的倍数）。
4. 删除临时文件，把字数组返回。
5. 编译失败时把 `stderr` 一起放进 anyhow 错误，否则很难排查。

检查点：调用 `compile(..., "vs_main", "vertex")` 能打印出一段非空的 `Vec<u32>`。

---

## 第 2 步：创建 ShaderModule

在 `Render` 里加一个辅助函数（不需要字段，用完就销毁）：

```rust
fn create_shader_module(device: &ash::Device, code: &[u32]) -> Result<vk::ShaderModule> {
    let create_info = vk::ShaderModuleCreateInfo::default().code(code);
    unsafe { device.create_shader_module(&create_info, None) }
        .context("failed to create a shader module")
}
```

要点：

- `ShaderModule` 只是 SPIR-V 的容器，**管线创建完成后就可以销毁**，不需要长期保存。
- 顶点和片元各创建一个，记得两边都要销毁（包括中间失败的分支）。

检查点：两个 module 都能创建成功，销毁时不报 validation 错误。

---

## 第 3 步：创建 PipelineLayout

`Render` 加字段：

```rust
pipeline_layout: vk::PipelineLayout,
```

初始化时设成 `vk::PipelineLayout::null()`，然后新增：

```rust
fn create_pipeline_layout(&mut self) -> Result<()> {
    let layout_info = vk::PipelineLayoutCreateInfo::default();
    self.pipeline_layout = unsafe { self.device.create_pipeline_layout(&layout_info, None) }
        .context("failed to create the pipeline layout")?;
    Ok(())
}
```

要点：

- 现在没有 uniform、纹理、push constant，所以布局是空的，但 Vulkan 要求创建管线时必须提供。
- 以后加 UBO / 纹理时，`set_layouts` 和 `push_constant_ranges` 会加在这里。
- 在 `new()` 里、创建管线之前调用：帧缓冲之后、`create_command_pool()` 之前。

检查点：布局创建成功，且 `Drop` 中按相反顺序销毁。

---

## 第 4 步：创建 GraphicsPipeline

`Render` 再加字段：

```rust
pipeline: vk::Pipeline,
```

同样初始化为 `vk::Pipeline::null()`。

创建流程（建议拆成 `create_pipeline`，内部自己决定是否销毁旧管线）：

1. `shader::compile` 两次，得到顶点/片元的 SPIR-V。
2. 各创建一个 `ShaderModule`。
3. 填写 `GraphicsPipelineCreateInfo`：

```rust
let stages = [
    vk::PipelineShaderStageCreateInfo::default()
        .stage(vk::ShaderStageFlags::VERTEX)
        .module(vertex_module)
        .name(c"vs_main"),          // 必须和 slangc 输出/入口名一致
    vk::PipelineShaderStageCreateInfo::default()
        .stage(vk::ShaderStageFlags::FRAGMENT)
        .module(fragment_module)
        .name(c"fs_main"),
];

let vertex_input = vk::PipelineVertexInputStateCreateInfo::default(); // 暂时没有顶点缓冲

let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
    .topology(vk::PrimitiveTopology::TRIANGLE_LIST);

let viewport_state = vk::PipelineViewportStateCreateInfo::default()
    .viewport_count(1)
    .scissor_count(1);

let rasterization = vk::PipelineRasterizationStateCreateInfo::default()
    .polygon_mode(vk::PolygonMode::FILL)
    .cull_mode(vk::CullModeFlags::NONE)   // 先关掉剔除，避免绕序问题
    .front_face(vk::FrontFace::CLOCKWISE)
    .line_width(1.0);

let multisampling = vk::PipelineMultisampleStateCreateInfo::default()
    .rasterization_samples(vk::SampleCountFlags::TYPE_1);

let color_blend_attachment = vk::PipelineColorBlendAttachmentState {
    blend_enable: vk::FALSE,
    src_color_blend_factor: vk::BlendFactor::ONE,
    dst_color_blend_factor: vk::BlendFactor::ZERO,
    color_blend_op: vk::BlendOp::ADD,
    src_alpha_blend_factor: vk::BlendFactor::ONE,
    dst_alpha_blend_factor: vk::BlendFactor::ZERO,
    alpha_blend_op: vk::BlendOp::ADD,
    color_write_mask: vk::ColorComponentFlags::RGBA,
};
let color_blend_attachments = [color_blend_attachment];
let color_blending = vk::PipelineColorBlendStateCreateInfo::default()
    .attachments(&color_blend_attachments);

let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
let dynamic_state = vk::PipelineDynamicStateCreateInfo::default()
    .dynamic_states(&dynamic_states);

let pipeline_info = vk::GraphicsPipelineCreateInfo::default()
    .stages(&stages)
    .vertex_input_state(&vertex_input)
    .input_assembly_state(&input_assembly)
    .viewport_state(&viewport_state)
    .rasterization_state(&rasterization)
    .multisample_state(&multisampling)
    .color_blend_state(&color_blending)
    .dynamic_state(&dynamic_state)
    .layout(self.pipeline_layout)
    .render_pass(self.render_pass)
    .subpass(0);
```

4. 调用：

```rust
let pipelines = unsafe {
    self.device.create_graphics_pipelines(
        vk::PipelineCache::null(),
        &[pipeline_info],
        None,
    )
}.map_err(|(_, err)| anyhow!("failed to create the graphics pipeline: {err}"))?;
```

5. 管线创建成功后，再销毁旧的 `self.pipeline` 并赋值新的（热重载的关键）。
6. 最后销毁两个 `ShaderModule`。

常见失败原因：

- 入口名写错（`vs_main` / `fs_main`）或 `-fvk-use-entrypoint-name` 没加。
- 顶点语义不匹配：VS 的 `SV_Position`、`COLOR0` 和 FS 输入不一致。
- `render_pass` / `subpass` 与创建用的 RenderPass 不一致。
- 用了动态视口却没在 `dynamic_states` 里声明 `VIEWPORT` / `SCISSOR`。

检查点：`create_graphics_pipelines` 成功；挂上 validation layer 后没有报错。

---

## 第 5 步：在命令缓冲里绑定并绘制

在 `record_command_buffer` 里，`cmd_begin_render_pass` 之后加入：

```rust
self.device.cmd_bind_pipeline(
    command_buffer,
    vk::PipelineBindPoint::GRAPHICS,
    self.pipeline,
);

let viewport = vk::Viewport {
    x: 0.0,
    y: 0.0,
    width: self.swapchain_extent.width as f32,
    height: self.swapchain_extent.height as f32,
    min_depth: 0.0,
    max_depth: 1.0,
};
self.device.cmd_set_viewport(command_buffer, 0, &[viewport]);

let scissor = vk::Rect2D {
    offset: vk::Offset2D { x: 0, y: 0 },
    extent: self.swapchain_extent,
};
self.device.cmd_set_scissor(command_buffer, 0, &[scissor]);

self.device.cmd_draw(command_buffer, 3, 1, 0, 0);
```

要点：

- `cmd_draw(3, ...)` 是 3 个顶点、1 个实例。
- 因为管线把 viewport/scissor 声明为动态状态，所以必须每帧设置，否则行为未定义。
- 顺序不能乱：必须在 render pass 开始之后、结束之前。

检查点：窗口里出现彩色三角形；缩放窗口后依旧正常。

---

## 第 6 步：热重载

先让代码在启动时能编译一次着色器（第 4 步已经做到），再加热重载：

1. 在 `Render` 上暴露：

```rust
pub fn reload_shaders(&mut self) -> Result<()> {
    unsafe { self.device.device_wait_idle()?; }
    self.create_pipeline()
}
```

`device_wait_idle` 确保旧管线不在使用中，才能安全销毁。

2. 在 `game.rs` 里加 `notify` 依赖，启动时监听 `shaders/` 目录：

```rust
let (tx, rx) = std::sync::mpsc::channel();
let mut watcher = notify::recommended_watcher(move |event| { /* 过滤 .slang 后 tx.send(()) */ })?;
watcher.watch(shader_dir, notify::RecursiveMode::NonRecursive)?;
```

3. 在 `about_to_wait` 里用 `rx.try_recv()` 检查事件，有变化就调用 `render.reload_shaders()`。
4. watcher 要存进 `Tever` 结构体，否则函数结束就被 drop，监听会停止。

注意：

- 主线程同步编译会让保存瞬间掉一帧；以后可以把编译放线程池，用 channel 把 SPIR-V 传回渲染线程。
- 编译失败时不要销毁旧管线，继续用旧的，并打印 slangc 的 stderr。

检查点：运行中修改 `.slang` 保存，画面在一帧左右后更新；故意写错语法时旧画面保留并打印错误。

---

## 第 7 步：进阶方向

按依赖顺序建议这样扩展：

1. **顶点缓冲**
   - 创建 `vk::Buffer` + 分配设备内存（`vkAllocateMemory`，用 `vkGetBufferMemoryRequirements` 选内存类型）。
   - 用 staging buffer 上传顶点数据。
   - 管线里填 `vertex_input`（binding/attribute description），绘制时 `cmd_bind_vertex_buffers`。
2. **Uniform Buffer**
   - `DescriptorSetLayout` + `PipelineLayout.set_layouts` + descriptor pool/set。
   - 每帧 `cmd_bind_descriptor_sets`，用 push constant 或 UBO 传 MVP 矩阵。
3. **深度缓冲**
   - 创建 depth image + view，加到 RenderPass 的附件里。
   - 管线里加 `depth_stencil_state`，并在每帧清屏时给两个 clear value。
4. **纹理、实例化、MSAA、多帧并行** 都属于后面的内容，先把 1～3 走通。

---

## 调试建议

- 重新加回 Vulkan validation layer，能挡住 90% 的“画面黑屏但程序不崩”问题。
  最小改动：创建 Instance 时启用 `VK_LAYER_KHRONOS_validation` 和 `VK_EXT_debug_utils`。
- 管线创建失败时，`create_graphics_pipelines` 的 `VkResult` 通常已经说明原因。
- 画面黑屏排查顺序：命令缓冲有没有录制 draw → 管线有没有绑定 → 视口/裁剪是否覆盖屏幕 →
  着色器输出位置是否在裁剪空间内。
- 推荐工具：RenderDoc（抓帧）、Vulkan SDK 自带 `vkconfig`。

---

## 自己写时的检查清单

- [ ] 新字段都在 `new()` 里初始化，`Drop` 里销毁，顺序与创建相反。
- [ ] `create_pipeline_layout` 在 `create_pipeline` 之前调用。
- [ ] 创建管线时 `render_pass` / `subpass` 与 `self.render_pass` 一致。
- [ ] 入口名与 slangc 输出一致。
- [ ] 动态状态在管线和命令缓冲两边对应。
- [ ] 管线重建/交换链重建后，命令缓冲使用的是最新句柄。
- [ ] `ShaderModule` 在管线创建后销毁。
- [ ] 热重载失败保留旧管线。

祝顺利，卡住时优先挂 validation layer 看具体是哪个对象/阶段报错。
