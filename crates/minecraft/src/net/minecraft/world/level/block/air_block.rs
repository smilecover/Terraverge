//! 对应 `net/minecraft/world/level/block/AirBlock.java`。

use super::state::block_behaviour::Properties;
use super::{Block, RenderShape};

/// Java 是 `class AirBlock extends Block`；Rust 暂用返回基础 `Block` 的
/// 构造函数表达“同一数据模型 + 覆写渲染形状”。
pub struct AirBlock;

impl AirBlock {
    pub fn new(properties: Properties) -> Block {
        Block::new_with_render_shape(properties, RenderShape::INVISIBLE)
    }
}
