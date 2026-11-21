//! 对应 `net/minecraft/world/level/block/LeavesBlock.java` 的状态与渲染部分。
//!
//! 完整行为（distance 衰减、随机刻腐烂、音效）待刻调度系统；这里先把
//! 属性与默认值注册出来，使 blockstate/模型能正确加载。

use super::Block;
use super::state::block_behaviour::Properties;
use super::state::properties::{BlockStateProperties, Property};

pub struct LeavesBlock;

impl LeavesBlock {
    pub const DECAY_DISTANCE: i32 = BlockStateProperties::MAX_DISTANCE;
    pub const DISTANCE: Property = BlockStateProperties::DISTANCE;
    pub const PERSISTENT: Property = BlockStateProperties::PERSISTENT;
    pub const WATERLOGGED: Property = BlockStateProperties::WATERLOGGED;

    pub fn new(properties: Properties) -> Block {
        Block::new(
            properties
                .add_state_property(Self::DISTANCE)
                .add_state_property(Self::PERSISTENT)
                .add_state_property(Self::WATERLOGGED),
        )
    }
}
