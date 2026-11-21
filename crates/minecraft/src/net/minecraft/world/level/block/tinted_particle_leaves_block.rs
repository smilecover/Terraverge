//! 对应 `net/minecraft/world/level/block/TintedParticleLeavesBlock.java`。
//!
//! 粒子行为未移植；构造函数保留橡木树叶的正确状态定义。

use super::state::block_behaviour::Properties;
use super::{Block, LeavesBlock};

pub struct TintedParticleLeavesBlock;

impl TintedParticleLeavesBlock {
    pub fn new(_chance: f32, properties: Properties) -> Block {
        LeavesBlock::new(properties)
    }
}
