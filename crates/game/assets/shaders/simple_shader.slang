// 纹理模型着色器（「Shader modules」「Vertex buffers」「Uniform buffers」
// 「Texture mapping」「Loading models」章节），用 Slang 语法编写：
// 位置/颜色/UV 从顶点缓冲读取，MVP 来自 uniform buffer，颜色来自组合图像采样器。
//
// 顶点与片元入口放在同一个文件里。运行期由 crates/game/src/shader.rs
// 通过 shader-slang 分别按 vs_main / fs_main 编译成两个 SPIR-V。
// 手动验证等价的 slangc 命令：
//   slangc simple_shader.slang -target spirv -entry vs_main -stage vertex   -fvk-use-entrypoint-name -o vert.spv
//   slangc simple_shader.slang -target spirv -entry fs_main -stage fragment -fvk-use-entrypoint-name -o frag.spv

// 顶点输入：location 必须与 Rust 侧 `Vertex::attribute_descriptions` 一致。
// 位置是 vec3，颜色是 vec3，UV 是 vec2，由管线按 32 字节步长读取。
struct VertexInput
{
    [[vk::location(0)]] float3 position : POSITION;
    [[vk::location(1)]] float3 color    : COLOR0;
    [[vk::location(2)]] float2 texCoord : TEXCOORD0;
};

// uniform buffer：set 0 / binding 0，字段布局必须与 Rust 的
// `UniformBufferObject` 一致（三个列主序 float4x4）。
struct UniformBufferObject
{
    float4x4 model;
    float4x4 view;
    float4x4 proj;
};

[[vk::binding(0, 0)]] ConstantBuffer<UniformBufferObject> ubo;

// 组合图像采样器：set 0 / binding 1。两个声明共享同一个 binding，
// 对应 Vulkan 的 VK_DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER。
[[vk::combinedImageSampler]]
[[vk::binding(1, 0)]]
Texture2D tex;

[[vk::combinedImageSampler]]
[[vk::binding(1, 0)]]
SamplerState texSampler;

struct VertexOutput
{
    float4 position : SV_Position;              // 与 gl_Position 等价
    [[vk::location(0)]] float3 color : COLOR0;  // 与 layout(location = 0) out vec3 fragColor 等价
    [[vk::location(1)]] float2 texCoord : TEXCOORD0;
};

[shader("vertex")]
VertexOutput vs_main(VertexInput input)
{
    // 顶点数据来自顶点缓冲，MVP 来自 uniform buffer。
    VertexOutput output;
    float4 world_position = mul(ubo.model, float4(input.position, 1.0));
    float4 view_position = mul(ubo.view, world_position);
    output.position = mul(ubo.proj, view_position);
    output.color = input.color;
    output.texCoord = input.texCoord;
    return output;
}

[shader("fragment")]
float4 fs_main(VertexOutput input) : SV_Target
{
    // 方块网格把“面向明暗 × 生物群系着色”预先乘进顶点色，
    // 图集只负责纹理本身。
    float4 texel = tex.Sample(texSampler, input.texCoord);
    // 树叶等 cutout 贴图依赖 alpha 测试；玻璃等半透明贴图由混合管线处理。
    if (texel.a < 0.1)
    {
        discard;
    }
    return texel * float4(input.color, 1.0);
}
