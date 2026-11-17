// 教程「Shader modules」章节的三角形着色器，改写为 Slang 语法。
//
// 顶点与片元入口放在同一个文件里。运行期由 crates/game/src/shader.rs
// 通过 shader-slang 分别按 vs_main / fs_main 编译成两个 SPIR-V。
// 手动验证等价的 slangc 命令：
//   slangc simple_shader.slang -target spirv -entry vs_main -stage vertex   -fvk-use-entrypoint-name -o vert.spv
//   slangc simple_shader.slang -target spirv -entry fs_main -stage fragment -fvk-use-entrypoint-name -o frag.spv

struct VertexOutput
{
    float4 position : SV_Position; // 与 gl_Position 等价
    float3 color    : COLOR0;      // 与 layout(location = 0) out vec3 fragColor 等价
};

[shader("vertex")]
VertexOutput vs_main(uint vertexIndex : SV_VulkanVertexID)
{
    // 教程中写死在着色器里的三角形顶点（NDC 坐标）。
    float2 positions[3] = {
        float2( 0.0, -0.5),
        float2( 0.5,  0.5),
        float2(-0.5,  0.5)
    };

    // 与顶点一一对应的颜色：红、绿、蓝。
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
    // 教程的 outColor = vec4(fragColor, 1.0)。
    return float4(input.color, 1.0);
}
