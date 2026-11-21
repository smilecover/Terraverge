//! 对应 `net/minecraft/client/resources/model/cuboid/CuboidRotation.java`。
//!
//! 元素级旋转：`origin` 在解析时除以 16（Java 的 `origin.mul(0.0625F)`），
//! 支持单轴角度与 `x/y/z` 欧拉角，以及可选的 `rescale`。

use crate::com::math::Mat3;
use crate::net::minecraft::core::Axis;
use crate::net::minecraft::util::json::{JsonError, JsonValue};

#[derive(Clone, Debug)]
pub struct CuboidRotation {
    /// 以方块单位（0..1）表示的旋转原点。
    pub origin: [f32; 3],
    pub axis: Option<Axis>,
    pub angle: f32,
    /// `x/y/z` 三轴欧拉角。
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
                .ok_or_else(|| JsonError::message("旋转原点 x 不是数字"))?
                / 16.0,
            origin_array[1]
                .as_f32()
                .ok_or_else(|| JsonError::message("旋转原点 y 不是数字"))?
                / 16.0,
            origin_array[2]
                .as_f32()
                .ok_or_else(|| JsonError::message("旋转原点 z 不是数字"))?
                / 16.0,
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

    /// 对应 `CuboidRotation.computeTransform`。
    pub fn transfer_matrix(&self) -> Mat3 {
        let mut matrix = if let Some(axis) = self.axis {
            Mat3::rotation_axis(axis, self.angle)
        } else if let Some([x, y, z]) = self.euler {
            // JOML `rotationZYX(z, y, x)` = Rz * Ry * Rx。
            Mat3::rotation_zyx_degrees(x, y, z)
        } else {
            Mat3::IDENTITY
        };

        if self.rescale && !matrix.is_identity() {
            let mut scale = [1.0f32; 3];
            for (index, axis) in Axis::VALUES.into_iter().enumerate() {
                let positive = axis.get_positive().get_step();
                let unit = [
                    positive.get_x() as f32,
                    positive.get_y() as f32,
                    positive.get_z() as f32,
                ];
                let transformed = matrix.transform_direction(unit);
                let max_component = transformed[0]
                    .abs()
                    .max(transformed[1].abs())
                    .max(transformed[2].abs());
                scale[index] = if max_component > 1.0e-9 {
                    1.0 / max_component
                } else {
                    1.0
                };
            }
            // Java `result.scale(scale)` 等价于右乘对角缩放。
            matrix = matrix.mul(Mat3::from_diagonal(scale));
        }

        matrix
    }
}
