//! 对应 `net/minecraft/world/level/block/SpreadingSnowyBlock.java`。
//!
//! 行为（`randomTick` 的草蔓延）需要光照/流体系统，当前先保持类型与
//! 状态定义一致，行为在后续里程碑补齐。

use super::state::block_behaviour::Properties;
use super::{Block, SnowyBlock};

pub struct SpreadingSnowyBlock;

impl SpreadingSnowyBlock {
    pub fn new(properties: Properties) -> Block {
        SnowyBlock::new(properties)
    }
}
