//! 对应 `net/minecraft/client/resources/model/cuboid/FaceBakery.java`。
//!
//! 负责把元素面转换为四顶点 `BakedQuad`：
//! - 顶点顺序与 Java 的 `FaceInfo` 一致，UV 转角按象限位移；
//! - 默认 UV 从 `from/to` 推导，算法逐行对应 `defaultFaceUV`；
//! - 变体的 `y` 旋转在这里应用（绕方块中心，顺时针）；
//! - 元素级旋转尚未实现，由上层在发现时显式报错。

use super::super::geometry::BakedQuad;
use super::cuboid_face::{CuboidFace, Uvs};
use crate::net::minecraft::core::Direction;
use crate::net::minecraft::resources::Identifier;

pub struct FaceBakery;

impl FaceBakery {
    /// 对应 Java `defaultFaceUV`。坐标单位是 0..16。
    pub fn default_face_uv(from: [f32; 3], to: [f32; 3], facing: Direction) -> Uvs {
        match facing {
            Direction::DOWN => Uvs {
                min_u: from[0],
                min_v: 16.0 - to[2],
                max_u: to[0],
                max_v: 16.0 - from[2],
            },
            Direction::UP => Uvs {
                min_u: from[0],
                min_v: from[2],
                max_u: to[0],
                max_v: to[2],
            },
            Direction::NORTH => Uvs {
                min_u: 16.0 - to[0],
                min_v: 16.0 - to[1],
                max_u: 16.0 - from[0],
                max_v: 16.0 - from[1],
            },
            Direction::SOUTH => Uvs {
                min_u: from[0],
                min_v: 16.0 - to[1],
                max_u: to[0],
                max_v: 16.0 - from[1],
            },
            Direction::WEST => Uvs {
                min_u: from[2],
                min_v: 16.0 - to[1],
                max_u: to[2],
                max_v: 16.0 - from[1],
            },
            Direction::EAST => Uvs {
                min_u: 16.0 - to[2],
                min_v: 16.0 - to[1],
                max_u: 16.0 - from[2],
                max_v: 16.0 - from[1],
            },
        }
    }

    /// 烘焙一个模型面。`variant_y` 是 blockstate 变体的 y 旋转角度。
    pub fn bake_quad(
        from: [f32; 3],
        to: [f32; 3],
        face: &CuboidFace,
        sprite: Identifier,
        facing: Direction,
        variant_y: i32,
    ) -> BakedQuad {
        let uvs = face
            .uvs
            .unwrap_or_else(|| Self::default_face_uv(from, to, facing));
        let mut positions = face_vertices(from, to, facing);
        let mut direction = facing;
        let mut cull_face = face.cull_for_direction;

        let shift = y_shift(variant_y);
        if shift != 0 {
            for position in &mut positions {
                // 模型坐标先转换为方块单位再绕中心旋转。
                let mut normalized = [position[0] / 16.0, position[1] / 16.0, position[2] / 16.0];
                normalized = rotate_y_clockwise(normalized, shift);
                *position = [
                    normalized[0] * 16.0,
                    normalized[1] * 16.0,
                    normalized[2] * 16.0,
                ];
            }
            direction = rotate_direction_y(direction, shift);
            cull_face = cull_face.map(|cull| rotate_direction_y(cull, shift));
        }

        let mut vertex_uvs = [[0.0f32; 2]; 4];
        for (index, uv) in vertex_uvs.iter_mut().enumerate() {
            let rotated = (index + face.rotation_shift as usize) % 4;
            *uv = [uvs.get_vertex_u(rotated), uvs.get_vertex_v(rotated)];
        }

        let normalized_positions =
            positions.map(|position| [position[0] / 16.0, position[1] / 16.0, position[2] / 16.0]);

        BakedQuad {
            positions: normalized_positions,
            uvs: vertex_uvs,
            direction,
            cull_face,
            tint_index: face.tint_index,
            sprite,
        }
    }
}

/// 每个面的 4 个顶点，顺序对应 UV 的 (min,min)、(min,max)、(max,max)、(max,min)。
fn face_vertices(from: [f32; 3], to: [f32; 3], facing: Direction) -> [[f32; 3]; 4] {
    let [x0, y0, z0] = from;
    let [x1, y1, z1] = to;
    match facing {
        Direction::DOWN => [[x0, y0, z1], [x0, y0, z0], [x1, y0, z0], [x1, y0, z1]],
        Direction::UP => [[x0, y1, z0], [x0, y1, z1], [x1, y1, z1], [x1, y1, z0]],
        Direction::NORTH => [[x1, y1, z0], [x1, y0, z0], [x0, y0, z0], [x0, y1, z0]],
        Direction::SOUTH => [[x0, y1, z1], [x0, y0, z1], [x1, y0, z1], [x1, y1, z1]],
        Direction::WEST => [[x0, y1, z0], [x0, y0, z0], [x0, y0, z1], [x0, y1, z1]],
        Direction::EAST => [[x1, y1, z1], [x1, y0, z1], [x1, y0, z0], [x1, y1, z0]],
    }
}

fn y_shift(degrees: i32) -> usize {
    (degrees.rem_euclid(360) / 90) as usize
}

fn rotate_y_clockwise(position: [f32; 3], shift: usize) -> [f32; 3] {
    if shift == 0 {
        return position;
    }
    let angle = -(shift as f32) * 90.0f32.to_radians();
    let cos = angle.cos();
    let sin = angle.sin();
    let x = position[0] - 0.5;
    let z = position[2] - 0.5;
    [
        x * cos + z * sin + 0.5,
        position[1],
        -x * sin + z * cos + 0.5,
    ]
}

fn rotate_direction_y(direction: Direction, shift: usize) -> Direction {
    let mut result = direction;
    for _ in 0..shift % 4 {
        result = result.get_clock_wise_y();
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_uvs_match_java() {
        let from = [0.0, 0.0, 0.0];
        let to = [16.0, 16.0, 16.0];
        for facing in Direction::VALUES {
            let uvs = FaceBakery::default_face_uv(from, to, facing);
            assert_eq!(uvs.min_u, 0.0);
            assert_eq!(uvs.min_v, 0.0);
            assert_eq!(uvs.max_u, 16.0);
            assert_eq!(uvs.max_v, 16.0);
        }
    }

    #[test]
    fn down_face_uv_matches_java() {
        let uvs = FaceBakery::default_face_uv([2.0, 0.0, 3.0], [10.0, 16.0, 9.0], Direction::DOWN);
        assert_eq!(uvs.min_u, 2.0);
        assert_eq!(uvs.min_v, 7.0);
        assert_eq!(uvs.max_u, 10.0);
        assert_eq!(uvs.max_v, 13.0);
    }

    #[test]
    fn y_rotation_moves_north_to_east() {
        let face = CuboidFace {
            cull_for_direction: Some(Direction::NORTH),
            tint_index: -1,
            texture: "#side".into(),
            uvs: None,
            rotation_shift: 0,
        };
        let quad = FaceBakery::bake_quad(
            [0.0, 0.0, 0.0],
            [16.0, 16.0, 16.0],
            &face,
            Identifier::parse("minecraft:block/stone").unwrap(),
            Direction::NORTH,
            90,
        );
        assert_eq!(quad.direction, Direction::EAST);
        assert_eq!(quad.cull_face, Some(Direction::EAST));
    }
}
