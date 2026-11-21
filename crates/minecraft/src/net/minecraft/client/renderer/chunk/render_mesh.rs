//! 与具体 GPU 后端无关的网格数据。
//!
//! 游戏层把 `RenderVertex` 转成 `ren_vulkan::Vertex`，把索引原样上传。
//! 这样 `minecraft` crate 不依赖 ash，渲染后端也不依赖 Minecraft 类型。

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RenderVertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
    pub uv: [f32; 2],
}

#[derive(Clone, Debug, Default)]
pub struct RenderMesh {
    pub vertices: Vec<RenderVertex>,
    pub indices: Vec<u32>,
}

impl RenderMesh {
    pub fn new() -> Self {
        Self::default()
    }

    /// 追加一个四边形（逆时针顶点顺序），并写入两个三角形。
    pub fn push_quad(&mut self, vertices: [RenderVertex; 4]) {
        let base = self.vertices.len() as u32;
        self.vertices.extend_from_slice(&vertices);
        self.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    pub fn quad_count(&self) -> usize {
        self.indices.len() / 6
    }
}
