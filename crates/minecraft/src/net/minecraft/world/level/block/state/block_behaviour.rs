//! 对应 `net/minecraft/world/level/block/state/BlockBehaviour.java` 的
//! `Properties` 部分。
//!
//! Java 的建造者方法会返回 `Properties` 自身，便于链式调用；Rust 侧保持
//! 同样风格。`instrument`/`noLootTable` 等当前渲染流程用不到的方法先保留
//! 为 no-op，以免注册代码与 Java 源码产生无意义的差异。

use super::properties::Property;
use crate::net::minecraft::resources::Identifier;
use crate::net::minecraft::world::level::block::{MapColor, SoundType};

#[derive(Clone, Debug)]
pub struct Properties {
    pub(crate) id: Option<Identifier>,
    /// 注册表分配的数字 ID；Java 由 `Registry.register` 内部维护。
    pub(crate) block_id: u16,
    pub(crate) map_color: MapColor,
    pub(crate) destroy_time: f32,
    pub(crate) explosion_resistance: f32,
    pub(crate) random_ticks: bool,
    pub(crate) sound_type: SoundType,
    pub(crate) replaceable: bool,
    pub(crate) no_collision: bool,
    pub(crate) is_air: bool,
    /// 是否拥有“完整方块遮挡形状”；对应 `BlockState.isSolidRender()`。
    pub(crate) can_occlude: bool,
    /// 相邻同种方块是否跳过渲染（玻璃类）。
    pub(crate) skip_rendering_same_block: bool,
    pub(crate) state_properties: Vec<Property>,
}

impl Properties {
    pub fn of() -> Self {
        Self {
            id: None,
            block_id: 0,
            map_color: MapColor::NONE,
            destroy_time: 0.0,
            explosion_resistance: 0.0,
            random_ticks: false,
            sound_type: SoundType::STONE,
            replaceable: false,
            no_collision: false,
            is_air: false,
            can_occlude: true,
            skip_rendering_same_block: false,
            state_properties: Vec::new(),
        }
    }

    pub fn map_color(mut self, map_color: MapColor) -> Self {
        self.map_color = map_color;
        self
    }

    /// Java 有两个 `strength` 重载；Rust 不支持重载，单参数版本命名为
    /// `strength_one`，二参数版本保持 `strength`。
    pub fn strength(mut self, destroy_time: f32, explosion_resistance: f32) -> Self {
        self.destroy_time = destroy_time;
        self.explosion_resistance = explosion_resistance;
        self
    }

    pub fn strength_one(self, destroy_time: f32) -> Self {
        self.strength(destroy_time, destroy_time)
    }

    pub fn random_ticks(mut self) -> Self {
        self.random_ticks = true;
        self
    }

    pub fn sound(mut self, sound_type: SoundType) -> Self {
        self.sound_type = sound_type;
        self
    }

    pub fn replaceable(mut self) -> Self {
        self.replaceable = true;
        self
    }

    pub fn no_collision(mut self) -> Self {
        self.no_collision = true;
        self.can_occlude = false;
        self
    }

    /// 对应 `noOcclusion()`：不参与遮挡剔除（玻璃、树叶等）。
    pub fn no_occlusion(mut self) -> Self {
        self.can_occlude = false;
        self
    }

    /// 对应 `HalfTransparentBlock.skipRendering`：相邻同种方块时跳过面。
    pub fn skip_rendering_same_block(mut self) -> Self {
        self.skip_rendering_same_block = true;
        self
    }

    pub fn air(mut self) -> Self {
        self.is_air = true;
        self
    }

    // 以下方法对应 Java 建造者链中出现、但不影响当前渲染的配置。
    pub fn requires_correct_tool_for_drops(self) -> Self {
        self
    }

    pub fn no_loot_table(self) -> Self {
        self
    }

    pub fn ignited_by_lava(self) -> Self {
        self
    }

    pub fn instrument(self) -> Self {
        self
    }

    pub fn add_state_property(mut self, property: Property) -> Self {
        if !self
            .state_properties
            .iter()
            .any(|existing| existing.get_name() == property.get_name())
        {
            self.state_properties.push(property);
        }
        self
    }

    pub(crate) fn set_id(&mut self, id: Identifier) {
        self.id = Some(id);
    }

    pub(crate) fn set_block_id(&mut self, block_id: u16) {
        self.block_id = block_id;
    }

    pub fn get_id(&self) -> Option<&Identifier> {
        self.id.as_ref()
    }

    pub fn get_map_color(&self) -> MapColor {
        self.map_color
    }

    pub fn get_destroy_time(&self) -> f32 {
        self.destroy_time
    }

    pub fn get_explosion_resistance(&self) -> f32 {
        self.explosion_resistance
    }

    pub fn has_random_ticks(&self) -> bool {
        self.random_ticks
    }

    pub fn get_sound_type(&self) -> SoundType {
        self.sound_type
    }

    pub fn can_be_replaced(&self) -> bool {
        self.replaceable
    }

    pub fn has_collision(&self) -> bool {
        !self.no_collision && !self.is_air
    }

    pub fn is_air(&self) -> bool {
        self.is_air
    }

    pub fn can_occlude(&self) -> bool {
        self.can_occlude
    }

    pub fn skips_rendering_same_block(&self) -> bool {
        self.skip_rendering_same_block
    }

    pub fn get_state_properties(&self) -> &[Property] {
        &self.state_properties
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_matches_java_chain() {
        let properties = Properties::of()
            .map_color(MapColor::GRASS)
            .random_ticks()
            .strength(0.6, 0.6)
            .sound(SoundType::GRASS);
        assert_eq!(properties.get_map_color(), MapColor::GRASS);
        assert!(properties.has_random_ticks());
        assert_eq!(properties.get_destroy_time(), 0.6);
    }
}
