//! 对应 `net/minecraft/client/resources/model/cuboid/CuboidRotation.java`。
//!
//! 元素级旋转（火炬倾斜、门板旋转等）当前只解析不应用；烘焙时会明确
//! 报错，避免默默渲染成错误姿态。变体的 `x/y` 旋转另行处理。

use crate::net::minecraft::core::Axis;
use crate::net::minecraft::util::json::{JsonError, JsonValue};

#[derive(Clone, Debug)]
pub struct CuboidRotation {
    /// 以方块单位（0..16）表示的旋转原点。
    pub origin: [f32; 3],
    pub axis: Option<Axis>,
    pub angle: f32,
    /// `x/y/z` 三轴欧拉角形式。
    pub euler: Option<[f32; 3]>,
    pub rescale: bool,
}

impl CuboidRotation {
    pub fn from_json(value: &JsonValue) -> Result<Self, JsonError> {
        let origin_array = value.require_array("origin")?;
        if origin_array.len() != 3 {
            return Err(JsonError::message("旋转 `origin` 必须包含 3 个数字"));
        }
        let origin = [
            origin_array[0]
                .as_f32()
                .ok_or_else(|| JsonError::message("旋转原点 x 不是数字"))?,
            origin_array[1]
                .as_f32()
                .ok_or_else(|| JsonError::message("旋转原点 y 不是数字"))?,
            origin_array[2]
                .as_f32()
                .ok_or_else(|| JsonError::message("旋转原点 z 不是数字"))?,
        ];

        let axis = value.optional_str("axis").map(|name| {
            Axis::by_name(name).ok_or_else(|| JsonError::message(format!("未知旋转轴 `{name}`")))
        });
        let axis = match axis {
            Some(result) => Some(result?),
            None => None,
        };
        let angle = value.optional_f32("angle").unwrap_or(0.0);
        let has_euler =
            value.get("x").is_some() || value.get("y").is_some() || value.get("z").is_some();
        let euler = if has_euler {
            Some([
                value.optional_f32("x").unwrap_or(0.0),
                value.optional_f32("y").unwrap_or(0.0),
                value.optional_f32("z").unwrap_or(0.0),
            ])
        } else {
            None
        };
        if axis.is_none() && euler.is_none() {
            return Err(JsonError::message(
                "旋转缺少 `axis`+`angle` 或 `x/y/z` 角度",
            ));
        }

        Ok(Self {
            origin,
            axis,
            angle,
            euler,
            rescale: value.optional_bool("rescale").unwrap_or(false),
        })
    }
}
