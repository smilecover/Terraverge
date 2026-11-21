//! 对应 `net/minecraft/client/renderer/block/dispatch/BlockStateModelDispatcher.java`。
//!
//! 支持 `variants`（单个/加权数组）与 `multipart`（条件选择器组合）。
//! 选择结果保留“是否 multipart”与每个部件的候选列表，供网格构建阶段
//! 使用 `Mth.getSeed(BlockPos)` 做与原版一致的确定性加权抽取。

use super::multipart::MultiPartSelector;
use super::variant::{WeightedVariant, weighted_variants_from_json};
use crate::net::minecraft::util::json::{JsonError, JsonValue};
use crate::net::minecraft::world::level::block::state::{BlockState, StateDefinition};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default)]
pub struct BlockStateModelDispatcher {
    /// 选择器字符串（如 `snowy=false`）-> 候选变体列表。
    variants: Option<BTreeMap<String, Vec<WeightedVariant>>>,
    /// `multipart` 的部件选择器。
    multipart: Option<Vec<MultiPartSelector>>,
}

/// 一个方块状态最终使用的模型集合。
///
/// - 普通 variants：`parts.len() == 1`，`multipart == false`；
/// - multipart：每个条件命中的 `apply` 对应一个 part，`multipart == true`，
///   所有 part 的四边形都会参与渲染；
/// - multipart 存在但没有条件命中：`parts` 为空（合法，模型为空）。
#[derive(Clone, Debug, PartialEq)]
pub struct BlockModelSelection {
    pub multipart: bool,
    pub parts: Vec<Vec<WeightedVariant>>,
}

impl BlockStateModelDispatcher {
    pub fn from_json(value: &JsonValue) -> Result<Self, JsonError> {
        let variants = match value.get("variants") {
            None => None,
            Some(variants) => {
                let object = variants
                    .as_object()
                    .ok_or_else(|| JsonError::message("`variants` 必须是对象"))?;
                if object.is_empty() {
                    return Err(JsonError::message("`variants` 不能为空"));
                }
                let mut parsed = BTreeMap::new();
                for (selector, entry) in object {
                    parsed.insert(selector.clone(), weighted_variants_from_json(entry)?);
                }
                Some(parsed)
            }
        };

        let multipart = match value.get("multipart") {
            None => None,
            Some(multipart) => {
                let array = multipart
                    .as_array()
                    .ok_or_else(|| JsonError::message("`multipart` 必须是数组"))?;
                if array.is_empty() {
                    return Err(JsonError::message("`multipart` 不能为空"));
                }
                let mut parsed = Vec::with_capacity(array.len());
                for selector in array {
                    parsed.push(MultiPartSelector::from_json(selector)?);
                }
                Some(parsed)
            }
        };

        if variants.is_none() && multipart.is_none() {
            return Err(JsonError::message(
                "blockstate 必须包含 `variants` 或 `multipart` 之一",
            ));
        }

        Ok(Self {
            variants,
            multipart,
        })
    }

    /// 对应 `instantiate`：variants 优先，未匹配的状态回退到 multipart。
    pub fn select(
        &self,
        definition: &StateDefinition,
        state: BlockState,
    ) -> Result<Option<BlockModelSelection>, String> {
        if let Some(variants) = &self.variants {
            let state_index = state.get_state_index();
            for (selector, candidates) in variants {
                if definition.matches_selector(state_index, selector) {
                    return Ok(Some(BlockModelSelection {
                        multipart: false,
                        parts: vec![candidates.clone()],
                    }));
                }
            }
        }

        if let Some(selectors) = &self.multipart {
            let mut parts = Vec::new();
            for selector in selectors {
                let matched = match &selector.condition {
                    Some(condition) => condition.test(definition, state)?,
                    None => true,
                };
                if matched {
                    parts.push(selector.variants.clone());
                }
            }
            return Ok(Some(BlockModelSelection {
                multipart: true,
                parts,
            }));
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::minecraft::util::json::parse;
    use crate::net::minecraft::world::level::block::Blocks;
    use crate::net::minecraft::world::level::block::state::properties::PropertyValue;

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

        let block = Blocks::grass_block();
        let definition = block.get_state_definition();
        let snowy = definition.get_property("snowy").unwrap();
        let default = block.default_block_state();

        let selection = dispatcher.select(definition, default).unwrap().unwrap();
        assert!(!selection.multipart);
        assert_eq!(selection.parts.len(), 1);
        assert_eq!(selection.parts[0].len(), 2);
        assert_eq!(selection.parts[0][0].weight, 1);
        assert_eq!(
            selection.parts[0][0].variant.model.to_string(),
            "minecraft:block/grass_block"
        );

        let snowy_state = default
            .set_value(snowy, PropertyValue::Boolean(true))
            .unwrap();
        let selection = dispatcher.select(definition, snowy_state).unwrap().unwrap();
        assert_eq!(
            selection.parts[0][0].variant.model.to_string(),
            "minecraft:block/grass_block_snow"
        );
    }

    #[test]
    fn parses_multipart_and_selects_all_matching_parts() {
        let json = parse(
            r#"{
                "multipart": [
                    {"apply": {"model": "block/cube_all"}},
                    {"when": "snowy=true", "apply": {"model": "block/snow"}},
                    {"when": {"OR": ["snowy=false"]}, "apply": [
                        {"model": "block/a", "weight": 2},
                        {"model": "block/b", "weight": 1}
                    ]}
                ]
            }"#,
        )
        .unwrap();
        let dispatcher = BlockStateModelDispatcher::from_json(&json).unwrap();
        let block = Blocks::grass_block();
        let definition = block.get_state_definition();
        let default = block.default_block_state();

        let selection = dispatcher.select(definition, default).unwrap().unwrap();
        assert!(selection.multipart);
        assert_eq!(selection.parts.len(), 2);
        assert_eq!(selection.parts[1].len(), 2);
        assert_eq!(selection.parts[1][0].weight, 2);

        let snowy = definition.get_property("snowy").unwrap();
        let snowy_state = default
            .set_value(snowy, PropertyValue::Boolean(true))
            .unwrap();
        let selection = dispatcher.select(definition, snowy_state).unwrap().unwrap();
        assert_eq!(selection.parts.len(), 2);
    }

    #[test]
    fn multipart_with_no_match_is_empty_not_error() {
        let json =
            parse(r#"{"multipart": [{"when": "snowy=true", "apply": {"model": "block/a"}}]}"#)
                .unwrap();
        let dispatcher = BlockStateModelDispatcher::from_json(&json).unwrap();
        let block = Blocks::grass_block();
        let selection = dispatcher
            .select(block.get_state_definition(), block.default_block_state())
            .unwrap()
            .unwrap();
        assert!(selection.multipart);
        assert!(selection.parts.is_empty());
    }

    #[test]
    fn weighted_arrays_require_positive_weights() {
        let json = parse(r#"{"variants": {"": [{"model": "block/a", "weight": 0}]}}"#).unwrap();
        assert!(BlockStateModelDispatcher::from_json(&json).is_err());
    }
}
