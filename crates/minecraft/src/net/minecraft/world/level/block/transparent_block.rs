//! 对应 `net/minecraft/world/level/block/TransparentBlock.java` /
//! `HalfTransparentBlock.java` 的渲染规则：
//! - 不参与遮挡剔除；
//! - 相邻同种方块时跳过一个面。

use super::Block;
use super::state::block_behaviour::Properties;

pub struct TransparentBlock;

impl TransparentBlock {
    pub fn new(properties: Properties) -> Block {
        Block::new(properties.no_occlusion().skip_rendering_same_block())
    }
}
