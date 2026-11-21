//! 对应 `net/minecraft/world/level/block/state/BlockState.java`。
//!
//! Java 的 `BlockState` 同时是状态值容器与全局状态表中的身份对象。
//! Rust 侧用 `(block_id, state_index)` 两个 u16 组成轻量 `Copy` 值，
//! 状态定义与属性值统一由所属 `Block` 的 `StateDefinition` 提供。

use super::StateDefinition;
use super::properties::{Property, PropertyValue};
use crate::net::minecraft::core::registries::builtin_registries;
use crate::net::minecraft::resources::Identifier;
use crate::net::minecraft::world::level::block::{Block, RenderShape};
use std::fmt;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BlockState {
    block_id: u16,
    state_index: u16,
}

impl BlockState {
    pub(crate) const fn new(block_id: u16, state_index: u16) -> Self {
        Self {
            block_id,
            state_index,
        }
    }

    pub const fn get_block_id(self) -> u16 {
        self.block_id
    }

    pub const fn get_state_index(self) -> u16 {
        self.state_index
    }

    /// 对应 `getBlock()`。
    pub fn get_block(self) -> &'static Block {
        builtin_registries::block_by_id(self.block_id)
    }

    pub fn get_state_definition(self) -> &'static StateDefinition {
        self.get_block().get_state_definition()
    }

    /// 对应 `getValue(Property)`。
    pub fn get_value(self, property: &Property) -> PropertyValue {
        self.get_optional_value(property)
            .unwrap_or_else(|| panic!("BlockState 不包含属性 `{}`", property.get_name()))
    }

    pub fn get_optional_value(self, property: &Property) -> Option<PropertyValue> {
        self.get_state_definition()
            .get_value(self.state_index, property.get_name())
    }

    /// 对应泛型 `getValue(BooleanProperty)`。
    pub fn get_value_bool(self, property: &Property) -> bool {
        self.get_value(property)
            .as_bool()
            .unwrap_or_else(|| panic!("属性 `{}` 不是布尔属性", property.get_name()))
    }

    pub fn get_value_int(self, property: &Property) -> i32 {
        self.get_value(property)
            .as_i32()
            .unwrap_or_else(|| panic!("属性 `{}` 不是整数属性", property.get_name()))
    }

    /// 对应 `setValue(Property, value)`：失败时返回 `None`（Java 抛异常）。
    pub fn set_value(self, property: &Property, value: PropertyValue) -> Option<BlockState> {
        self.get_state_definition()
            .get_state_with(self, property, value)
    }

    pub fn has_property(self, name: &str) -> bool {
        self.get_state_definition().get_property(name).is_some()
    }

    pub fn is_air(self) -> bool {
        self.get_block().is_air()
    }

    pub fn has_collision(self) -> bool {
        self.get_block().has_collision(self)
    }

    pub fn get_render_shape(self) -> RenderShape {
        self.get_block().get_render_shape(self)
    }

    pub fn get_block_name(self) -> &'static Identifier {
        self.get_block().get_name()
    }

    /// 所有可能状态，顺序与 `StateDefinition` 注册顺序一致。
    pub fn possible_states(self) -> &'static [BlockState] {
        self.get_state_definition().get_possible_states()
    }

    /// `snowy=false,age=3` 形式的属性键，用于 blockstate JSON 匹配。
    pub fn property_key(self) -> String {
        self.get_state_definition().state_key(self.state_index)
    }

    pub fn values(self) -> &'static [PropertyValue] {
        self.get_state_definition().state_values(self.state_index)
    }
}

impl fmt::Display for BlockState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let block = self.get_block();
        write!(f, "{}[{}]", block.get_name(), self.property_key())
    }
}
