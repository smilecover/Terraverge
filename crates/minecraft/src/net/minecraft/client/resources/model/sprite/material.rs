//! 对应 `net/minecraft/client/resources/model/sprite/Material.java`。
//!
//! 26.3 的贴图槽既可以是字符串，也可以是：
//! `{"sprite": "minecraft:block/glass", "force_translucent": true}`。

use crate::net::minecraft::resources::Identifier;
use crate::net::minecraft::util::json::{JsonError, JsonValue};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Material {
    pub sprite: Identifier,
    pub force_translucent: bool,
}

impl Material {
    pub fn new(sprite: Identifier, force_translucent: bool) -> Self {
        Self {
            sprite,
            force_translucent,
        }
    }

    /// 对应 `Material.CODEC`：`either(SIMPLE_CODEC, FULL_CODEC)`。
    pub fn from_json(value: &JsonValue) -> Result<Self, JsonError> {
        match value {
            JsonValue::String(reference) => {
                let sprite = Identifier::parse(reference).map_err(|err| {
                    JsonError::message(format!("非法贴图标识符 `{reference}`: {err}"))
                })?;
                Ok(Self::new(sprite, false))
            }
            JsonValue::Object(_) => {
                let sprite_name = value.require_str("sprite")?;
                let sprite = Identifier::parse(sprite_name).map_err(|err| {
                    JsonError::message(format!("非法贴图标识符 `{sprite_name}`: {err}"))
                })?;
                Ok(Self::new(
                    sprite,
                    value.optional_bool("force_translucent").unwrap_or(false),
                ))
            }
            other => Err(JsonError::message(format!(
                "贴图值必须是字符串或对象，实际是 {}",
                other.type_name()
            ))),
        }
    }
}
