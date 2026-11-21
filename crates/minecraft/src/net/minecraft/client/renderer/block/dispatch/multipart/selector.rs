//! 对应 `multipart/Selector.java`：`when` 条件 + `apply` 加权变体。

use super::Condition;
use crate::net::minecraft::client::renderer::block::dispatch::variant::{
    WeightedVariant, weighted_variants_from_json,
};
use crate::net::minecraft::util::json::{JsonError, JsonValue};

#[derive(Clone, Debug, PartialEq)]
pub struct MultiPartSelector {
    /// `None` 表示无条件（永远匹配）。
    pub condition: Option<Condition>,
    pub variants: Vec<WeightedVariant>,
}

impl MultiPartSelector {
    pub fn from_json(value: &JsonValue) -> Result<Self, JsonError> {
        let condition = match value.get("when") {
            Some(when) => Some(Condition::from_json(when)?),
            None => None,
        };
        let apply = value.require("apply")?;
        let variants = weighted_variants_from_json(apply)?;
        Ok(Self {
            condition,
            variants,
        })
    }
}
