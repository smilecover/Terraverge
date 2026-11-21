//! 对应 `net/minecraft/client/resources/model/cuboid/CuboidFace.java`。

use crate::net::minecraft::core::Direction;
use crate::net::minecraft::util::json::{JsonError, JsonValue};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Uvs {
    pub min_u: f32,
    pub min_v: f32,
    pub max_u: f32,
    pub max_v: f32,
}

impl Uvs {
    /// 与 Java 相同的顶点 UV 顺序：
    /// 0=(min,min)、1=(min,max)、2=(max,max)、3=(max,min)。
    pub fn get_vertex_u(self, index: usize) -> f32 {
        if index == 0 || index == 1 {
            self.min_u
        } else {
            self.max_u
        }
    }

    pub fn get_vertex_v(self, index: usize) -> f32 {
        if index == 0 || index == 3 {
            self.min_v
        } else {
            self.max_v
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CuboidFace {
    pub cull_for_direction: Option<Direction>,
    pub tint_index: i32,
    /// `#slot` 或直接贴图标识符。
    pub texture: String,
    pub uvs: Option<Uvs>,
    /// 0/90/180/270 度对应的象限位移（0..3）。
    pub rotation_shift: u8,
}

impl CuboidFace {
    pub fn from_json(value: &JsonValue) -> Result<Self, JsonError> {
        let texture = value.require_str("texture")?.to_string();
        let cull_for_direction = match value.optional_str("cullface") {
            None | Some("") => None,
            Some(name) => Some(
                Direction::by_name(name)
                    .ok_or_else(|| JsonError::message(format!("未知的 cullface 方向 `{name}`")))?,
            ),
        };
        let tint_index = value.optional_i32("tintindex").unwrap_or(-1);
        let uvs = match value.get("uv") {
            None => None,
            Some(uv) => {
                let array = uv
                    .as_array()
                    .ok_or_else(|| JsonError::message("`uv` 必须是数组"))?;
                if array.len() != 4 {
                    return Err(JsonError::message("`uv` 必须包含 4 个数字"));
                }
                Some(Uvs {
                    min_u: array[0]
                        .as_f32()
                        .ok_or_else(|| JsonError::message("uv[0] 不是数字"))?,
                    min_v: array[1]
                        .as_f32()
                        .ok_or_else(|| JsonError::message("uv[1] 不是数字"))?,
                    max_u: array[2]
                        .as_f32()
                        .ok_or_else(|| JsonError::message("uv[2] 不是数字"))?,
                    max_v: array[3]
                        .as_f32()
                        .ok_or_else(|| JsonError::message("uv[3] 不是数字"))?,
                })
            }
        };

        let rotation = value.optional_i32("rotation").unwrap_or(0);
        let normalized = rotation.rem_euclid(360);
        if !matches!(normalized, 0 | 90 | 180 | 270) {
            return Err(JsonError::message(format!(
                "面 UV 旋转只允许 0/90/180/270，实际为 {rotation}"
            )));
        }
        let rotation_shift = (normalized / 90) as u8;

        Ok(Self {
            cull_for_direction,
            tint_index,
            texture,
            uvs,
            rotation_shift,
        })
    }
}
