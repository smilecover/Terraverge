//! 游戏资源加载：直接读取 `minecraft-26.3-client-src` 解包目录。
//!
//! 加载顺序与 Java 客户端一致：ResourceManager → 语言 → 方块模型 →
//! 图集 → 世界网格。渲染后端只接收 SPIR-V、顶点/索引和 RGBA8 像素。

use minecraft::net::minecraft::client::multiplayer::ClientLevel;
use minecraft::net::minecraft::client::renderer::chunk::{RenderMesh, SectionCompiler};
use minecraft::net::minecraft::client::renderer::texture::{SpriteLoader, TextureAtlas};
use minecraft::net::minecraft::client::resources::language::LanguageManager;
use minecraft::net::minecraft::client::resources::model::BlockModelSet;
use minecraft::net::minecraft::server::packs::resources::ResourceManager;
use minecraft::net::minecraft::world::phys::Vec3;
use render::{
    MeshData, ParticleShaders, SceneShaders, ShaderProgram, ShaderStage, TextureData,
    TextureFilter, Vertex,
};
use std::error::Error;
use std::path::{Path, PathBuf};

use crate::{shader, shader::CompiledParticleShaders, shader::CompiledShader};

/// 平坦演示世界半径（区块）。81 个区块足以观察远处地平线与区块边界。
const WORLD_RADIUS: i32 = 4;

pub struct Assets {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
    atlas: TextureAtlas,
    level: ClientLevel,
    spawn_position: Vec3,
    models: BlockModelSet,
    /// 语言表保留给后续 UI；当前尚未绘制文字。
    #[allow(dead_code)]
    language: LanguageManager,
    model_shader: CompiledShader,
    particle_shader: CompiledParticleShaders,
}

impl Assets {
    pub fn load() -> Result<Self, Box<dyn Error + Send + Sync>> {
        let root = find_client_root()?;
        println!("Minecraft 资源根目录: {}", root.display());

        let resource_manager = ResourceManager::new(&root);
        let language = LanguageManager::load(&resource_manager, "en_us")
            .map_err(|err| format!("语言文件加载失败: {err}"))?;
        let models = BlockModelSet::load(&resource_manager)
            .map_err(|err| format!("方块模型加载失败: {err}"))?;

        let atlas = SpriteLoader::load_atlas(&resource_manager, models.textures());
        let level = ClientLevel::new_flat(WORLD_RADIUS, 0);
        let spawn_position = level.get_spawn_position();
        let mesh = SectionCompiler::compile(&level, &models, &atlas);

        println!(
            "世界: {} 个区块，出生点 ({}), 网格 {} 顶点 / {} 四边形，图集 {}x{} ({} 张)，语言 {} 键",
            level.get_chunk_count(),
            spawn_position,
            mesh.vertex_count(),
            mesh.quad_count(),
            atlas.get_width(),
            atlas.get_height(),
            atlas.sprite_count(),
            language.len()
        );

        let (vertices, indices) = convert_mesh(mesh);

        let model_shader = shader::compile(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/shaders/simple_shader.slang"),
        )
        .map_err(|err| format!("方块着色器编译失败: {err}"))?;
        let particle_shader = shader::compile_particles(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/shaders/particles.slang"),
        )
        .map_err(|err| format!("粒子着色器编译失败: {err}"))?;

        Ok(Self {
            vertices,
            indices,
            atlas,
            level,
            spawn_position,
            models,
            language,
            model_shader,
            particle_shader,
        })
    }

    /// 方块发生变化后重新编译整个世界网格。M0/M1 仍是全量重建；
    /// M2 会改为按区块脏标记与异步构建。
    pub fn rebuild_mesh(&mut self) {
        let mesh = SectionCompiler::compile(&self.level, &self.models, &self.atlas);
        let (vertices, indices) = convert_mesh(mesh);
        self.vertices = vertices;
        self.indices = indices;
    }

    pub fn level_mut(&mut self) -> &mut ClientLevel {
        &mut self.level
    }

    pub fn quad_count(&self) -> usize {
        self.indices.len() / 6
    }

    pub fn shaders(&self) -> SceneShaders<'_> {
        SceneShaders {
            model: ShaderProgram {
                vertex: ShaderStage {
                    code: &self.model_shader.vertex,
                    entry_point: shader::VERTEX_ENTRY_POINT,
                },
                fragment: ShaderStage {
                    code: &self.model_shader.fragment,
                    entry_point: shader::FRAGMENT_ENTRY_POINT,
                },
            },
            particles: ParticleShaders {
                vertex: ShaderStage {
                    code: &self.particle_shader.vertex,
                    entry_point: shader::PARTICLE_VERTEX_ENTRY_POINT,
                },
                fragment: ShaderStage {
                    code: &self.particle_shader.fragment,
                    entry_point: shader::PARTICLE_FRAGMENT_ENTRY_POINT,
                },
                compute: ShaderStage {
                    code: &self.particle_shader.compute,
                    entry_point: shader::PARTICLE_COMPUTE_ENTRY_POINT,
                },
            },
        }
    }

    pub fn mesh(&self) -> MeshData<'_> {
        MeshData {
            vertices: &self.vertices,
            indices: &self.indices,
        }
    }

    pub fn texture(&self) -> TextureData<'_> {
        TextureData {
            width: self.atlas.get_width(),
            height: self.atlas.get_height(),
            pixels: self.atlas.get_pixels(),
            filter: TextureFilter::Nearest,
            generate_mipmaps: false,
        }
    }

    pub fn level(&self) -> &ClientLevel {
        &self.level
    }

    pub fn spawn_position(&self) -> Vec3 {
        self.spawn_position
    }
}

fn convert_mesh(mesh: RenderMesh) -> (Vec<Vertex>, Vec<u32>) {
    let vertices = mesh
        .vertices
        .iter()
        .map(|vertex| Vertex::new(vertex.position, vertex.color, vertex.uv))
        .collect();
    (vertices, mesh.indices)
}

/// 找到包含 `assets/minecraft` 与 `data/minecraft` 的客户端解包目录。
///
/// 查找顺序：
/// 1. 环境变量 `TERRAVERGE_CLIENT_ROOT`；
/// 2. 本 crate 向上两级（工作区根）的 `minecraft-26.3-client-src`；
/// 3. 当前目录及其所有祖先。
pub fn find_client_root() -> Result<PathBuf, Box<dyn Error + Send + Sync>> {
    if let Ok(value) = std::env::var("TERRAVERGE_CLIENT_ROOT") {
        let path = PathBuf::from(value);
        if is_client_root(&path) {
            return Ok(path);
        }
        return Err(format!(
            "TERRAVERGE_CLIENT_ROOT 指向的目录不是有效的客户端根目录: {}",
            path.display()
        )
        .into());
    }

    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_candidate = manifest
        .join("..")
        .join("..")
        .join("minecraft-26.3-client-src");
    if is_client_root(&workspace_candidate) {
        return Ok(normalize(&workspace_candidate));
    }

    let mut current = std::env::current_dir()?;
    loop {
        let candidate = current.join("minecraft-26.3-client-src");
        if is_client_root(&candidate) {
            return Ok(candidate);
        }
        if !current.pop() {
            break;
        }
    }

    Err("找不到 minecraft-26.3-client-src；请设置 TERRAVERGE_CLIENT_ROOT".into())
}

fn is_client_root(path: &Path) -> bool {
    path.join("assets/minecraft").is_dir() && path.join("data/minecraft").is_dir()
}

fn normalize(path: &Path) -> PathBuf {
    path.components().collect()
}
