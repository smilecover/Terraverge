//! `net.minecraft.core.registries` 包。
//!
//! Java 侧 `Registry<T>` 支持键值注册、标签、Holder 等大量能力；
//! 这里先移植按 ID/标识符查询的最小实现，供方块注册使用。

pub mod builtin_registries;

use super::super::resources::Identifier;
use std::collections::HashMap;

pub struct Registry<T> {
    values: Vec<T>,
    by_name: HashMap<Identifier, u16>,
}

impl<T> Registry<T> {
    pub(crate) fn new(values: Vec<T>, by_name: HashMap<Identifier, u16>) -> Self {
        Self { values, by_name }
    }

    pub fn by_id(&self, id: u16) -> &T {
        &self.values[id as usize]
    }

    pub fn get(&self, id: &Identifier) -> Option<&T> {
        self.by_name.get(id).map(|&id| &self.values[id as usize])
    }

    pub fn get_id(&self, id: &Identifier) -> Option<u16> {
        self.by_name.get(id).copied()
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.values.iter()
    }

    pub fn ids(&self) -> impl Iterator<Item = (u16, &Identifier)> + '_ {
        let names = &self.by_name;
        names
            .iter()
            .map(move |(name, &id)| (id, name))
            .collect::<Vec<_>>()
            .into_iter()
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}
