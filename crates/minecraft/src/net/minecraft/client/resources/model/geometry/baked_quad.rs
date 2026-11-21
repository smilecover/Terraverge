//! 对应 `net/minecraft/client/resources/model/geometry/BakedQuad.java`。
//!
//! 26.3 的 BakedQuad 保存四个顶点、打包 UV 与材质信息；这里用未压缩的
//! 数组表达，便于网格构建器直接消费。

use crate::net::minecraft::core::Direction;
use crate::net::minecraft::resources::Identifier;

#[derive(Clone, Debug)]
pub struct BakedQuad {
    /// 方块局部坐标，范围 0..1。
    pub positions: [[f32; 3]; 4],
    /// 贴图坐标，范围 0..16（与模型 JSON 的 uv 单位一致）。
    pub uvs: [[f32; 2]; 4],
    /// 面朝向，用于光照明暗。
    pub direction: Direction,
    /// 非空时与相邻方块同向的面可以剔除。
    pub cull_face: Option<Direction>,
    /// -1 表示无着色；>=0 时由生物群系/方块着色器决定颜色。
    pub tint_index: i32,
    pub sprite: Identifier,
}

#[derive(Clone, Debug, Default)]
pub struct BakedModel {
    pub quads: Vec<BakedQuad>,
}

impl BakedModel {
    pub fn new(quads: Vec<BakedQuad>) -> Self {
        Self { quads }
    }

    pub fn is_empty(&self) -> bool {
        self.quads.is_empty()
    }
}
