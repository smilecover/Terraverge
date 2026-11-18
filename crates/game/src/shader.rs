//! 运行时用 Slang 把 `.slang` 源文件编译成 SPIR-V。
//!
//! 教程把着色器源代码和 SPIR-V 都当作资源文件离线编译；这里改成运行期编译：
//! 游戏负责资源路径和入口点名称，渲染后端只接收 SPIR-V 字数组，
//! 因此 `ren_vulkan` 不需要知道 Slang 或资源目录的存在。

use shader_slang::{
    CompileTarget, Downcast, GlobalSession, Module, Session, SessionDesc, TargetDesc,
};
use std::error::Error;
use std::path::Path;

/// 模型着色器的顶点/片元入口点，必须与 `simple_shader.slang` 里的函数名一致。
pub const VERTEX_ENTRY_POINT: &str = "vs_main";
pub const FRAGMENT_ENTRY_POINT: &str = "fs_main";

/// 粒子着色器入口点，必须与 `particles.slang` 里的函数名一致。
pub const PARTICLE_VERTEX_ENTRY_POINT: &str = "vs_main";
pub const PARTICLE_FRAGMENT_ENTRY_POINT: &str = "fs_main";
pub const PARTICLE_COMPUTE_ENTRY_POINT: &str = "cs_main";

/// 模型着色器的编译结果。
pub struct CompiledShader {
    pub vertex: Vec<u32>,
    pub fragment: Vec<u32>,
}

/// 粒子系统的编译结果：绘制用的顶点/片元 + 更新粒子的计算着色器。
pub struct CompiledParticleShaders {
    pub vertex: Vec<u32>,
    pub fragment: Vec<u32>,
    pub compute: Vec<u32>,
}

/// 把 `path` 指向的模型 Slang 文件按顶点/片元入口点编译成 SPIR-V。
pub fn compile(path: &Path) -> Result<CompiledShader, Box<dyn Error + Send + Sync>> {
    let mut codes = compile_file(path, &[VERTEX_ENTRY_POINT, FRAGMENT_ENTRY_POINT])?;
    let fragment = codes.pop().expect("两个入口点都已编译");
    let vertex = codes.pop().expect("两个入口点都已编译");
    Ok(CompiledShader { vertex, fragment })
}

/// 把粒子 Slang 文件按顶点/片元/计算入口点分别编译成 SPIR-V。
pub fn compile_particles(
    path: &Path,
) -> Result<CompiledParticleShaders, Box<dyn Error + Send + Sync>> {
    let mut codes = compile_file(
        path,
        &[
            PARTICLE_VERTEX_ENTRY_POINT,
            PARTICLE_FRAGMENT_ENTRY_POINT,
            PARTICLE_COMPUTE_ENTRY_POINT,
        ],
    )?;
    let compute = codes.pop().expect("三个入口点都已编译");
    let fragment = codes.pop().expect("三个入口点都已编译");
    let vertex = codes.pop().expect("三个入口点都已编译");
    Ok(CompiledParticleShaders {
        vertex,
        fragment,
        compute,
    })
}

/// 打开一个 Slang 会话，按给定入口点列表编译同一份文件。
fn compile_file(
    path: &Path,
    entry_points: &[&str],
) -> Result<Vec<Vec<u32>>, Box<dyn Error + Send + Sync>> {
    // 全局会话持有 Slang 编译器实例；会话保存目标格式、搜索路径等选项。
    let global_session = GlobalSession::new().ok_or("初始化 Slang 全局会话失败")?;

    // 默认情况下 Slang 会把 SPIR-V 入口点改名为 `main`；
    // 打开这个选项后保留源文件里的入口点名称，管线阶段才能按名字找到它们。
    // 矩阵按列主序解释，与 glam 的 Mat4 内存布局保持一致。
    let options = shader_slang::CompilerOptions::default()
        .vulkan_use_entry_point_name(true)
        .matrix_layout_column(true);

    let target = TargetDesc::default()
        .format(CompileTarget::Spirv)
        .profile(global_session.find_profile("spirv_1_5"))
        .options(&options);
    let targets = [target];
    let session_desc = SessionDesc::default().targets(&targets);

    let session = global_session
        .create_session(&session_desc)
        .ok_or("创建 Slang 会话失败")?;

    let path = path.to_str().ok_or("着色器路径包含非 UTF-8 字符")?;
    let module = session.load_module(path)?;

    entry_points
        .iter()
        .map(|name| compile_entry_point(&session, &module, name))
        .collect()
}

fn compile_entry_point(
    session: &Session,
    module: &Module,
    name: &str,
) -> Result<Vec<u32>, Box<dyn Error + Send + Sync>> {
    let entry_point = module
        .find_entry_point_by_name(name)
        .ok_or_else(|| format!("着色器里没有入口点 `{name}`"))?;

    // 模块 + 入口点组合成可链接程序；链接后按入口点取代码。
    let program = session.create_composite_component_type(&[
        module.downcast().clone(),
        entry_point.downcast().clone(),
    ])?;
    let linked = program.link()?;

    // 组合里只有一个入口点，所以入口索引固定为 0；目标也只有 SPIR-V 一个。
    let blob = linked.entry_point_code(0, 0)?;
    let bytes = blob.as_slice();
    if bytes.len() % 4 != 0 {
        return Err("Slang 输出的 SPIR-V 字节数不是 4 的倍数".into());
    }

    // SPIR-V 是小端字节流，转成 vkCreateShaderModule 需要的 u32 数组。
    Ok(bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|word| u32::from_le_bytes(*word))
        .collect())
}
