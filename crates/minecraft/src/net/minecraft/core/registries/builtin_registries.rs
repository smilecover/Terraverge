//! 对应 `net/minecraft/core/registries/BuiltInRegistries.java`。
//!
//! Java 用可变的静态注册表并在类初始化阶段填充；Rust 用 `OnceLock` 在
//! 第一次访问时构建。所有方块注册都在 `Blocks::bootstrap` 中完成，
//! 顺序与 `Blocks.java` 的静态字段声明顺序一致（本移植只包含首批方块）。

use super::Registry;
use crate::net::minecraft::resources::Identifier;
use crate::net::minecraft::world::level::block::{Block, Blocks};
use std::sync::OnceLock;

static BLOCK_REGISTRY: OnceLock<Registry<Block>> = OnceLock::new();

/// 对应 `BuiltInRegistries.BLOCK`。
pub fn block() -> &'static Registry<Block> {
    BLOCK_REGISTRY.get_or_init(Blocks::bootstrap)
}

pub fn block_by_id(id: u16) -> &'static Block {
    block().by_id(id)
}

pub fn block_by_name(name: &Identifier) -> Option<&'static Block> {
    block().get(name)
}
