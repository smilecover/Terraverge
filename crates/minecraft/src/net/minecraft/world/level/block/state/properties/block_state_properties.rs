//! 对应 `net/minecraft/world/level/block/state/properties/BlockStateProperties.java`
//! 的常用属性常量。

use super::Property;

pub struct BlockStateProperties;

impl BlockStateProperties {
    pub const MAX_DISTANCE: i32 = 7;
    pub const DISTANCE: Property = Property::integer("distance", 7, 1, 7);
    pub const PERSISTENT: Property = Property::boolean("persistent", false);
    pub const WATERLOGGED: Property = Property::boolean("waterlogged", false);
}
