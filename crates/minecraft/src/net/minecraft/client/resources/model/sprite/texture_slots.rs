//! 对应 `net/minecraft/client/resources/model/sprite/TextureSlots.java`。
//!
//! 模型 JSON 的 `textures` 是“槽位名 -> 贴图”的映射：
//! - 字符串以 `#` 开头：槽位引用；
//! - 其他字符串：直接贴图标识符（`Material` 简写）；
//! - 对象：完整 `Material`（含 `force_translucent`）。
//!
//! 父子模型合并时后写入者覆盖；解析按引用链进行，最多 16 层。

use super::super::model_error::ModelError;
use super::material::Material;
use crate::net::minecraft::util::json::{JsonError, JsonValue};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
enum SlotContents {
    Reference(String),
    Value(Material),
}

#[derive(Clone, Debug, Default)]
pub struct TextureSlots {
    textures: BTreeMap<String, SlotContents>,
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
        for (slot, entry) in object {
            let contents = match entry {
                JsonValue::String(reference) if reference.starts_with('#') => {
                    SlotContents::Reference(reference[1..].to_string())
                }
                _ => SlotContents::Value(Material::from_json(entry)?),
            };
            textures.insert(slot.clone(), contents);
        }
        Ok(Self { textures })
    }

    pub fn get(&self, slot: &str) -> Option<&Material> {
        match self.textures.get(slot) {
            Some(SlotContents::Value(material)) => Some(material),
            _ => None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.textures.is_empty()
    }

    pub fn len(&self) -> usize {
        self.textures.len()
    }

    pub fn merge_from(&mut self, other: &TextureSlots) {
        for (slot, contents) in &other.textures {
            self.textures
                .entry(slot.clone())
                .or_insert_with(|| contents.clone());
        }
    }

    /// 子模型覆盖父模型，因此这里是无条件写入。
    pub fn override_from(&mut self, other: &TextureSlots) {
        for (slot, contents) in &other.textures {
            self.textures.insert(slot.clone(), contents.clone());
        }
    }

    /// 解析面引用的 `"#side"` 或直接标识符，返回带透明信息的 `Material`。
    /// 循环别名最多跟随 16 层。
    pub fn resolve(&self, reference: &str) -> Result<Material, ModelError> {
        let Some(mut slot) = reference.strip_prefix('#') else {
            return Material::from_json(&JsonValue::String(reference.to_string()))
                .map_err(|err| ModelError::new(format!("非法贴图标识符 `{reference}`: {err}")));
        };

        for _ in 0..16 {
            match self.textures.get(slot) {
                Some(SlotContents::Value(material)) => return Ok(material.clone()),
                Some(SlotContents::Reference(target)) => slot = target,
                None => {
                    return Err(ModelError::new(format!(
                        "模型缺少贴图槽 `#{slot}`（引用 `{reference}`）"
                    )));
                }
            }
        }
        Err(ModelError::new(format!("贴图槽别名层级过深: {reference}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::minecraft::util::json::parse;

    #[test]
    fn resolves_references_and_material_objects() {
        let json = parse(
            r##"{
                "all": "block/stone",
                "side": "#all",
                "glass": {"sprite": "block/glass", "force_translucent": true}
            }"##,
        )
        .unwrap();
        let slots = TextureSlots::from_json(&json).unwrap();

        let resolved = slots.resolve("#side").unwrap();
        assert_eq!(resolved.sprite.to_string(), "minecraft:block/stone");
        assert!(!resolved.force_translucent);

        let glass = slots.resolve("#glass").unwrap();
        assert_eq!(glass.sprite.to_string(), "minecraft:block/glass");
        assert!(glass.force_translucent);
    }

    #[test]
    fn detects_reference_cycles() {
        let json = parse(r##"{"a": "#b", "b": "#a"}"##).unwrap();
        let slots = TextureSlots::from_json(&json).unwrap();
        assert!(slots.resolve("#a").is_err());
    }
}
