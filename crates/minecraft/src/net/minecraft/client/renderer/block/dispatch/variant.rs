//! 对应 `net/minecraft/client/renderer/block/dispatch/Variant.java`。
//!
//! JSON 形如：
//! ```json
//! {"model": "minecraft:block/stone", "x": 0, "y": 90, "z": 0, "uvlock": true, "weight": 2}
//! ```
//! `weight` 只在数组形式的加权变体里生效（Java 的 `BlockStateModel.Unbaked.ELEMENT_CODEC`）。

use crate::net::minecraft::resources::Identifier;
use crate::net::minecraft::util::json::{JsonError, JsonValue};

#[derive(Clone, Debug, PartialEq)]
pub struct Variant {
    pub model: Identifier,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub uvlock: bool,
    pub weight: i32,
}

impl Variant {
    pub fn from_json(value: &JsonValue) -> Result<Self, JsonError> {
        let model_name = value.require_str("model")?;
        let model = Identifier::parse(model_name)
            .map_err(|err| JsonError::message(format!("非法模型标识符 `{model_name}`: {err}")))?;

        let x = parse_quadrant(value, "x")?;
        let y = parse_quadrant(value, "y")?;
        let z = parse_quadrant(value, "z")?;

        Ok(Self {
            model,
            x,
            y,
            z,
            uvlock: value.optional_bool("uvlock").unwrap_or(false),
            weight: value.optional_i32("weight").unwrap_or(1),
        })
    }
}

/// 一个带权重的变体。`apply`/`variants` 数组中的每个对象对应一项。
#[derive(Clone, Debug, PartialEq)]
pub struct WeightedVariant {
    pub variant: Variant,
    pub weight: i32,
}

/// 解析 Java 的 `either(HARDCODED_WEIGHTED_CODEC, SingleVariant.CODEC)`：
/// - 对象：单个变体，权重固定为 1（对象里的 `weight` 字段被忽略）；
/// - 数组：非空加权列表，每项必须带正权重。
pub fn weighted_variants_from_json(value: &JsonValue) -> Result<Vec<WeightedVariant>, JsonError> {
    match value {
        JsonValue::Object(_) => Ok(vec![WeightedVariant {
            variant: Variant::from_json(value)?,
            weight: 1,
        }]),
        JsonValue::Array(values) => {
            if values.is_empty() {
                return Err(JsonError::message("加权变体数组不能为空"));
            }
            let mut result = Vec::with_capacity(values.len());
            for item in values {
                if !matches!(item, JsonValue::Object(_)) {
                    return Err(JsonError::message("加权变体数组的每一项必须是对象"));
                }
                let variant = Variant::from_json(item)?;
                if variant.weight <= 0 {
                    return Err(JsonError::message(format!(
                        "加权变体 `{}` 的权重必须是正整数，实际为 {}",
                        variant.model, variant.weight
                    )));
                }
                let weight = variant.weight;
                result.push(WeightedVariant { variant, weight });
            }
            Ok(result)
        }
        other => Err(JsonError::message(format!(
            "变体必须是对象或数组，实际是 {}",
            other.type_name()
        ))),
    }
}

/// 对应 `Quadrant.CODEC`：只接受 0/90/180/270（允许负角度取模）。
fn parse_quadrant(value: &JsonValue, field: &str) -> Result<i32, JsonError> {
    let degrees = value.optional_i32(field).unwrap_or(0);
    if degrees.rem_euclid(90) != 0 {
        return Err(JsonError::message(format!(
            "变体旋转 `{field}` 只允许 0/90/180/270，实际为 {degrees}"
        )));
    }
    Ok(degrees.rem_euclid(360))
}
