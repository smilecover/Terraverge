//! 对应 `net/minecraft/client/resources/model/cuboid/CuboidModelElement.java`。

use super::super::model_error::ModelError;
use super::CuboidFace;
use super::cuboid_rotation::CuboidRotation;
use crate::net::minecraft::core::Direction;
use crate::net::minecraft::util::json::{JsonError, JsonValue};
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct CuboidModelElement {
    pub from: [f32; 3],
    pub to: [f32; 3],
    pub faces: BTreeMap<Direction, CuboidFace>,
    pub rotation: Option<CuboidRotation>,
    pub shade_direction_override: Option<Direction>,
    pub light_emission: i32,
}

impl CuboidModelElement {
    pub fn from_json(value: &JsonValue) -> Result<Self, JsonError> {
        let from = parse_position(value, "from")?;
        let to = parse_position(value, "to")?;
        validate_extent(from, to)?;

        let faces_object = value.require_object("faces")?;
        let mut faces = BTreeMap::new();
        for (name, face_value) in faces_object {
            let direction = Direction::by_name(name)
                .ok_or_else(|| JsonError::message(format!("未知的模型面方向 `{name}`")))?;
            // Java 的 CuboidModelElement 允许存储退化轴上的面，
            // 由 `should_draw_face` 在烘焙阶段剔除；这里不改写模型数据。
            faces.insert(direction, CuboidFace::from_json(face_value)?);
        }

        let rotation = match value.get("rotation") {
            Some(rotation) => Some(CuboidRotation::from_json(rotation)?),
            None => None,
        };
        let shade_direction_override = match value.optional_str("shade_direction_override") {
            None => None,
            Some(name) => Some(Direction::by_name(name).ok_or_else(|| {
                JsonError::message(format!("未知的 shade_direction_override `{name}`"))
            })?),
        };
        let light_emission = value.optional_i32("light_emission").unwrap_or(0);
        if !(0..=15).contains(&light_emission) {
            return Err(JsonError::message("`light_emission` 必须在 0..=15 之间"));
        }

        Ok(Self {
            from,
            to,
            faces,
            rotation,
            shade_direction_override,
            light_emission,
        })
    }

    pub fn has_rotation(&self) -> Option<ModelError> {
        self.rotation
            .as_ref()
            .map(|_| ModelError::new("cuboid 元素旋转尚未移植（见 docs/PROGRESS.md）"))
    }
}

impl CuboidModelElement {
    /// 对应 `UnbakedCuboidGeometry.bake` 的轴剔除判定。
    pub fn should_draw_face(&self, facing: Direction) -> bool {
        let (draw_x, draw_y, draw_z) = draw_axes(self.from, self.to);
        match facing.get_axis() {
            crate::net::minecraft::core::Axis::X => draw_x,
            crate::net::minecraft::core::Axis::Y => draw_y,
            crate::net::minecraft::core::Axis::Z => draw_z,
        }
    }
}

fn draw_axes(from: [f32; 3], to: [f32; 3]) -> (bool, bool, bool) {
    let mut draw_x = true;
    let mut draw_y = true;
    let mut draw_z = true;
    if from[0] == to[0] {
        draw_y = false;
        draw_z = false;
    }
    if from[1] == to[1] {
        draw_x = false;
        draw_z = false;
    }
    if from[2] == to[2] {
        draw_x = false;
        draw_y = false;
    }
    (draw_x, draw_y, draw_z)
}

fn parse_position(value: &JsonValue, field: &str) -> Result<[f32; 3], JsonError> {
    let array = value.require_array(field)?;
    if array.len() != 3 {
        return Err(JsonError::message(format!("`{field}` 必须包含 3 个数字")));
    }
    Ok([
        array[0]
            .as_f32()
            .ok_or_else(|| JsonError::message(format!("`{field}[0]` 不是数字")))?,
        array[1]
            .as_f32()
            .ok_or_else(|| JsonError::message(format!("`{field}[1]` 不是数字")))?,
        array[2]
            .as_f32()
            .ok_or_else(|| JsonError::message(format!("`{field}[2]` 不是数字")))?,
    ])
}

fn validate_extent(from: [f32; 3], to: [f32; 3]) -> Result<(), JsonError> {
    for axis in 0..3 {
        if !(-16.0..=32.0).contains(&from[axis]) || !(-16.0..=32.0).contains(&to[axis]) {
            return Err(JsonError::message(
                "模型元素坐标必须位于 -16..=32（方块单位）",
            ));
        }
        if from[axis] > to[axis] {
            return Err(JsonError::message("模型元素的 `from` 必须不大于 `to`"));
        }
    }
    Ok(())
}
