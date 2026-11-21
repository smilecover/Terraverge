//! 对应 `net/minecraft/world/level/block/SnowyBlock.java`。
//!
//! 只移植状态定义（`SNOWY` 属性）；草方块传播、雪融化等随机刻行为待后续。

use super::Block;
use super::state::block_behaviour::Properties;
use super::state::properties::Property;

pub struct SnowyBlock;

impl SnowyBlock {
    /// `public static final BooleanProperty SNOWY = BooleanProperty.create("snowy");`
    pub const SNOWY: Property = Property::boolean("snowy", false);

    pub fn new(properties: Properties) -> Block {
        Block::new(properties.add_state_property(Self::SNOWY))
    }
}
