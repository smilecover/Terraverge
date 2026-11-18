// 计算着色器粒子系统（「Compute Shader」章节），Slang 语法。
//
// 同一个文件里包含三个入口：
//   cs_main —— 计算阶段，用 deltaTime 更新粒子位置并处理边界反弹；
//   vs_main —— 顶点阶段，把粒子位置当作 NDC 坐标、设置点大小；
//   fs_main —— 片元阶段，用 PointCoord 画圆形软边粒子。
//
// Rust 侧分别按这三个入口编译成独立 SPIR-V。

// 与 Rust 的 Particle 结构一致：2+2+4 个 float，共 32 字节。
struct Particle
{
    float2 position;
    float2 velocity;
    float4 color;
};

struct Parameters
{
    float deltaTime;
};

// 计算描述符：set 0 的 binding 0/1/2。
[[vk::binding(0, 0)]] ConstantBuffer<Parameters> parameters;
[[vk::binding(1, 0)]] StructuredBuffer<Particle> particlesIn;
[[vk::binding(2, 0)]] RWStructuredBuffer<Particle> particlesOut;

[shader("compute")]
[numthreads(256, 1, 1)]
void cs_main(uint3 dispatchThreadID : SV_DispatchThreadID)
{
    uint index = dispatchThreadID.x;

    Particle particle = particlesIn[index];

    // 按 deltaTime 前进；碰到 [-1, 1] 边界时翻转对应方向的速度。
    float2 position = particle.position + particle.velocity * parameters.deltaTime;
    float2 velocity = particle.velocity;
    if (position.x <= -1.0 || position.x >= 1.0)
    {
        velocity.x = -velocity.x;
    }
    if (position.y <= -1.0 || position.y >= 1.0)
    {
        velocity.y = -velocity.y;
    }

    particlesOut[index].position = position;
    particlesOut[index].velocity = velocity;
    particlesOut[index].color = particle.color;
}

// 粒子缓冲同时作为顶点缓冲：只需要位置和颜色。
struct ParticleInput
{
    [[vk::location(0)]] float2 position : POSITION;
    [[vk::location(1)]] float4 color    : COLOR0;
};

struct ParticleOutput
{
    float4 position : SV_Position;
    // Slang 用 HLSL 语义 SV_PointSize 生成 SPIR-V 的 PointSize 内建输出。
    float pointSize : SV_PointSize;
    [[vk::location(0)]] float3 color : COLOR0;
};

[shader("vertex")]
ParticleOutput vs_main(ParticleInput input)
{
    ParticleOutput output;
    // 教程：gl_Position = vec4(inPosition.xy, 1.0, 1.0)，gl_PointSize = 14.0。
    output.position = float4(input.position, 1.0, 1.0);
    output.pointSize = 14.0;
    output.color = input.color.rgb;
    return output;
}

struct ParticleFragmentInput
{
    [[vk::location(0)]] float3 color : COLOR0;
    // 对应 SPIR-V 的 PointCoord 内建输入。
    float2 pointCoord : SV_PointCoord;
};

[shader("fragment")]
float4 fs_main(ParticleFragmentInput input) : SV_Target
{
    // 教程：alpha = 0.5 - length(gl_PointCoord - vec2(0.5))，形成圆形软边。
    float2 coord = input.pointCoord - float2(0.5, 0.5);
    return float4(input.color, 0.5 - length(coord));
}
