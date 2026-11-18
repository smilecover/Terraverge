//! 游戏资源加载：着色器、模型和纹理。
//!
//! 资源路径与 CPU 侧数据属于游戏层；渲染后端只接收 SPIR-V、网格切片和像素。
//! `Game` 初始化时加载一次，`Render::new` 会把它们转换成 GPU 资源。

use crate::{model, shader, texture};
use render::{MeshData, ParticleShaders, SceneShaders, ShaderProgram, ShaderStage, TextureData};
use std::error::Error;
use std::path::Path;

/// 一份完整的启动资源，按需借出给渲染后端。
pub struct Assets {
    model: model::ModelData,
    texture: texture::Texture,
    model_shader: shader::CompiledShader,
    particle_shader: shader::CompiledParticleShaders,
}

impl Assets {
    /// 从 `assets/` 目录加载全部资源，并为每一步附加可读的错误信息。
    pub fn load() -> Result<Self, Box<dyn Error + Send + Sync>> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));

        let model_shader = shader::compile(&root.join("assets/shaders/simple_shader.slang"))
            .map_err(|err| format!("着色器编译失败: {err}"))?;
        let particle_shader =
            shader::compile_particles(&root.join("assets/shaders/particles.slang"))
                .map_err(|err| format!("粒子着色器编译失败: {err}"))?;
        let model = model::load(&root.join("assets/models/cube.obj"))
            .map_err(|err| format!("模型加载失败: {err}"))?;
        let texture = texture::load(&root.join("assets/textures/texture.png"))
            .map_err(|err| format!("纹理加载失败: {err}"))?;

        Ok(Self {
            model,
            texture,
            model_shader,
            particle_shader,
        })
    }

    /// 组装模型 + 粒子系统的全部着色器阶段。
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
        self.model.as_mesh()
    }

    pub fn texture(&self) -> TextureData<'_> {
        self.texture.as_render_data()
    }
}
