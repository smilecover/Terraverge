//! 对应 `net/minecraft/client/renderer/block/dispatch/Variant.java`。
//!
//! JSON 形如：
//! ```json
//! {"model": "minecraft:block/stone", "x": 0, "y": 90, "uvlock": true, "weight": 2}
//! ```

use crate::net::minecraft::resources::Identifier;
use crate::net::minecraft::util::json::{JsonError, JsonValue};

#[derive(Clone, Debug, PartialEq)]
pub struct Variant {
    pub model: Identifier,
    pub x: i32,
    pub y: i32,
    pub uvlock: bool,
    pub weight: i32,
}

impl Variant {
    pub fn from_json(value: &JsonValue) -> Result<Self, JsonError> {
        let model_name = value.require_str("model")?;
        let model = Identifier::parse(model_name)
            .map_err(|err| JsonError::message(format!("非法模型标识符 `{model_name}`: {err}")))?;
        Ok(Self {
            model,
            x: value.optional_i32("x").unwrap_or(0),
            y: value.optional_i32("y").unwrap_or(0),
            uvlock: value.optional_bool("uvlock").unwrap_or(false),
            weight: value.optional_i32("weight").unwrap_or(1),
        })
    }
}
