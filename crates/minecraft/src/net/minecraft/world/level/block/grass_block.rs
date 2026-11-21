//! 对应 `net/minecraft/world/level/block/GrassBlock.java`。
//!
//! Java：`GrassBlock extends SpreadingSnowyBlock implements BonemealableBlock`。
//! 当前移植继承链与状态（`snowy`），骨粉与传播行为待光照/特性系统就绪后实现。

use super::state::block_behaviour::Properties;
use super::{Block, SpreadingSnowyBlock};

pub struct GrassBlock;

impl GrassBlock {
    pub fn new(properties: Properties) -> Block {
        SpreadingSnowyBlock::new(properties)
    }
}
