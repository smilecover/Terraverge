//! 对应 `net/minecraft/world/level/block/Blocks.java` 的注册入口。
//!
//! 首批方块与渲染/资源包冒烟测试保持一致：空气、石头、草方块、泥土、
//! 圆石、基岩、橡木木板。后续按 `docs/PROGRESS.md` 的清单继续补齐。

use super::state::block_behaviour::Properties;
use super::{AirBlock, Block, GrassBlock, MapColor, SoundType};
use crate::net::minecraft::core::registries::Registry;
use crate::net::minecraft::core::registries::builtin_registries;
use crate::net::minecraft::resources::Identifier;
use std::collections::HashMap;

/// Java 中 `Blocks` 的静态字段；Rust 用访问函数替代（注册表为运行期构建）。
pub struct Blocks;

impl Blocks {
    pub fn air() -> &'static Block {
        block_by_path("air")
    }

    pub fn stone() -> &'static Block {
        block_by_path("stone")
    }

    pub fn grass_block() -> &'static Block {
        block_by_path("grass_block")
    }

    pub fn dirt() -> &'static Block {
        block_by_path("dirt")
    }

    pub fn cobblestone() -> &'static Block {
        block_by_path("cobblestone")
    }

    pub fn bedrock() -> &'static Block {
        block_by_path("bedrock")
    }

    pub fn oak_planks() -> &'static Block {
        block_by_path("oak_planks")
    }

    /// 构建并注册首批方块。由 `BuiltInRegistries::block()` 懒初始化调用。
    pub(crate) fn bootstrap() -> Registry<Block> {
        let mut registry = BlockRegistryBuilder::new();

        registry.register(
            "minecraft:air",
            AirBlock::new,
            Properties::of()
                .map_color(MapColor::NONE)
                .replaceable()
                .no_collision()
                .no_loot_table()
                .air(),
        );
        registry.register(
            "minecraft:stone",
            Block::new,
            Properties::of()
                .map_color(MapColor::STONE)
                .requires_correct_tool_for_drops()
                .strength(1.5, 6.0),
        );
        registry.register(
            "minecraft:grass_block",
            GrassBlock::new,
            Properties::of()
                .map_color(MapColor::GRASS)
                .random_ticks()
                .strength(0.6, 0.6)
                .sound(SoundType::GRASS),
        );
        registry.register(
            "minecraft:dirt",
            Block::new,
            Properties::of()
                .map_color(MapColor::DIRT)
                .strength(0.5, 0.5)
                .sound(SoundType::GRAVEL),
        );
        registry.register(
            "minecraft:cobblestone",
            Block::new,
            Properties::of()
                .map_color(MapColor::STONE)
                .requires_correct_tool_for_drops()
                .strength(2.0, 6.0),
        );
        registry.register(
            "minecraft:bedrock",
            Block::new,
            Properties::of()
                .map_color(MapColor::STONE)
                .strength(-1.0, 3_600_000.0),
        );
        registry.register(
            "minecraft:oak_planks",
            Block::new,
            Properties::of()
                .map_color(MapColor::WOOD)
                .strength(2.0, 3.0)
                .sound(SoundType::WOOD)
                .ignited_by_lava(),
        );

        registry.build()
    }
}

fn block_by_path(path: &str) -> &'static Block {
    let id = Identifier::with_default_namespace(path).expect("硬编码的方块路径必须合法");
    builtin_registries::block_by_name(&id).unwrap_or_else(|| panic!("方块 {id} 未注册"))
}

/// `Registry.register(BuiltInRegistries.BLOCK, ...)` 的 Rust 版本。
pub(crate) struct BlockRegistryBuilder {
    values: Vec<Block>,
    by_name: HashMap<Identifier, u16>,
}

impl BlockRegistryBuilder {
    pub(crate) fn new() -> Self {
        Self {
            values: Vec::new(),
            by_name: HashMap::new(),
        }
    }

    pub(crate) fn register(
        &mut self,
        key: &str,
        factory: impl FnOnce(Properties) -> Block,
        mut properties: Properties,
    ) {
        let block_id = self.values.len() as u16;
        let id = Identifier::parse(key).expect("方块标识符必须合法");
        assert!(!self.by_name.contains_key(&id), "重复注册方块 {id}");

        properties.set_id(id.clone());
        properties.set_block_id(block_id);
        let block = factory(properties);

        self.by_name.insert(id, block_id);
        self.values.push(block);
    }

    pub(crate) fn build(self) -> Registry<Block> {
        Registry::new(self.values, self.by_name)
    }
}
