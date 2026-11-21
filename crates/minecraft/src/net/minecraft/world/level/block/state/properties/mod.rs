//! `net.minecraft.world.level.block.state.properties` 包。
//!
//! 完整移植包含 Boolean/Integer/Enum 等属性类；当前实现统一的
//! `Property` 句柄，先支持布尔与整数，枚举属性待移植。

mod block_state_properties;
mod property;

pub use block_state_properties::BlockStateProperties;
pub use property::{Property, PropertyKind, PropertyValue};
