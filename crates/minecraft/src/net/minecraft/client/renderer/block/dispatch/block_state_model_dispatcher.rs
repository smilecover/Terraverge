//! 对应 `net/minecraft/client/renderer/block/dispatch/BlockStateModelDispatcher.java`。
//!
//! 支持 `variants`（含加权数组）；`multipart` 依赖选择器表达式与
//! 多模型组合，尚未移植，遇到时会明确报错而不是静默出错。

use super::variant::Variant;
use crate::net::minecraft::util::json::{JsonError, JsonValue};
use crate::net::minecraft::world::level::block::state::{BlockState, StateDefinition};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default)]
pub struct BlockStateModelDispatcher {
    /// 选择器字符串（如 `snowy=false`）-> 候选变体列表。
    variants: BTreeMap<String, Vec<Variant>>,
}

impl BlockStateModelDispatcher {
    pub fn from_json(value: &JsonValue) -> Result<Self, JsonError> {
        if value.get("multipart").is_some() {
            return Err(JsonError::message(
                "blockstate `multipart` 尚未移植（见 docs/PROGRESS.md）",
            ));
        }

        let variants_value = value.require("variants")?;
        let variants_object = variants_value
            .as_object()
            .ok_or_else(|| JsonError::message("`variants` 必须是对象"))?;

        let mut variants = BTreeMap::new();
        for (selector, entry) in variants_object {
            let list = match entry {
                JsonValue::Array(values) => {
                    if values.is_empty() {
                        return Err(JsonError::message(format!(
                            "变体选择器 `{selector}` 的数组为空"
                        )));
                    }
                    values
                        .iter()
                        .map(Variant::from_json)
                        .collect::<Result<Vec<_>, _>>()?
                }
                JsonValue::Object(_) => vec![Variant::from_json(entry)?],
                _ => {
                    return Err(JsonError::message(format!(
                        "变体选择器 `{selector}` 必须是对象或数组"
                    )));
                }
            };
            variants.insert(selector.clone(), list);
        }

        Ok(Self { variants })
    }

    /// 找出第一个匹配该状态的选择器，并取候选列表的第一个变体。
    /// 加权变体应随机抽取，当前为了确定性先取第一个。
    pub fn select(&self, definition: &StateDefinition, state: BlockState) -> Option<Variant> {
        let state_index = state.get_state_index();
        for (selector, variants) in &self.variants {
            if definition.matches_selector(state_index, selector) {
                return variants.first().cloned();
            }
        }
        None
    }

    pub fn variant_count(&self) -> usize {
        self.variants.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::minecraft::util::json::parse;

    #[test]
    fn parses_single_and_weighted_variants() {
        let json = parse(
            r#"{
                "variants": {
                    "snowy=false": [
                        {"model": "block/grass_block"},
                        {"model": "block/grass_block", "y": 90}
                    ],
                    "snowy=true": {"model": "block/grass_block_snow"}
                }
            }"#,
        )
        .unwrap();
        let dispatcher = BlockStateModelDispatcher::from_json(&json).unwrap();
        assert_eq!(dispatcher.variant_count(), 2);

        let block = crate::net::minecraft::world::level::block::Blocks::grass_block();
        let definition = block.get_state_definition();
        let snowy = definition.get_property("snowy").unwrap();
        let default = block.default_block_state();
        let variant = dispatcher.select(definition, default).unwrap();
        assert_eq!(variant.model.to_string(), "minecraft:block/grass_block");
        assert_eq!(variant.y, 0);

        let snowy_state = default
            .set_value(snowy, crate::net::minecraft::world::level::block::state::properties::PropertyValue::Boolean(true))
            .unwrap();
        let variant = dispatcher.select(definition, snowy_state).unwrap();
        assert_eq!(
            variant.model.to_string(),
            "minecraft:block/grass_block_snow"
        );
    }

    #[test]
    fn rejects_multipart_for_now() {
        let json = parse(r#"{"multipart": []}"#).unwrap();
        assert!(BlockStateModelDispatcher::from_json(&json).is_err());
    }
}
