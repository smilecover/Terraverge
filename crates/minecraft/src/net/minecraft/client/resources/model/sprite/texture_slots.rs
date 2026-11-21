//! 对应 `net/minecraft/client/resources/model/sprite/TextureSlots.java`。
//!
//! 模型 JSON 的 `textures` 是“槽位名 -> 贴图引用”的映射；引用可以是
//! 直接标识符，也可以是 `#other_slot` 形式的别名链。

use super::super::model_error::ModelError;
use crate::net::minecraft::resources::Identifier;
use crate::net::minecraft::util::json::{JsonError, JsonValue};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default)]
pub struct TextureSlots {
    textures: BTreeMap<String, String>,
}

impl TextureSlots {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_json(value: &JsonValue) -> Result<Self, JsonError> {
        let object = value
            .as_object()
            .ok_or_else(|| JsonError::message("`textures` 必须是对象"))?;
        let mut textures = BTreeMap::new();
        for (slot, reference) in object {
            let reference = reference
                .as_str()
                .ok_or_else(|| JsonError::message(format!("贴图槽 `{slot}` 的值不是字符串")))?;
            textures.insert(slot.clone(), reference.to_string());
        }
        Ok(Self { textures })
    }

    pub fn get(&self, slot: &str) -> Option<&str> {
        self.textures.get(slot).map(String::as_str)
    }

    pub fn is_empty(&self) -> bool {
        self.textures.is_empty()
    }

    pub fn len(&self) -> usize {
        self.textures.len()
    }

    pub fn merge_from(&mut self, other: &TextureSlots) {
        for (slot, reference) in &other.textures {
            self.textures
                .entry(slot.clone())
                .or_insert_with(|| reference.clone());
        }
    }

    pub fn override_from(&mut self, other: &TextureSlots) {
        for (slot, reference) in &other.textures {
            self.textures.insert(slot.clone(), reference.clone());
        }
    }

    /// 把面引用的 `"#side"` 或 `"block/stone"` 解析成最终贴图标识符。
    /// 循环别名最多跟随 16 层，防止模型数据自引用导致死循环。
    pub fn resolve(&self, reference: &str) -> Result<Identifier, ModelError> {
        let mut current = reference;
        for _ in 0..16 {
            let Some(slot) = current.strip_prefix('#') else {
                return Identifier::parse(current)
                    .map_err(|err| ModelError::new(format!("非法贴图标识符 `{current}`: {err}")));
            };
            current = self.get(slot).ok_or_else(|| {
                ModelError::new(format!("模型缺少贴图槽 `#{slot}`（引用 `{reference}`）"))
            })?;
        }
        Err(ModelError::new(format!("贴图槽别名层级过深: {reference}")))
    }

    /// 收集所有最终贴图（`#` 别名不计入），用于构建图集。
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.textures
            .iter()
            .map(|(slot, reference)| (slot.as_str(), reference.as_str()))
    }
}
