//! 对应 `net/minecraft/client/resources/model/cuboid/FaceBakery.java`。
//!
//! 顺序与 Java 一致：
//! 1. 按 `FaceInfo` 顺序取面顶点（0..16），除以 16；
//! 2. 应用元素级旋转（绕元素 `origin`）；
//! 3. 应用 blockstate 变体旋转（绕方块中心 0.5）；
//! 4. 计算最终朝向（用于光照/剔除判断）；
//! 5. UV 按象限旋转；`uvlock` 时再做一次面向 UV 补偿。
//!
//! Java 在存在元素旋转时会重排顶点到最终朝向的规范顺序；由于旋转矩阵
//! 行列式为 +1，三角形绕序不会被翻转，本实现保留顶点与 UV 的绑定关系，
//! 仅重新计算 `direction`。这一点在 `docs/DECISIONS.md` 中记录。

use super::super::geometry::BakedQuad;
use super::cuboid_face::{CuboidFace, Uvs};
use super::cuboid_rotation::CuboidRotation;
use crate::com::math::Mat3;
use crate::net::minecraft::client::renderer::block::dispatch::Variant;
use crate::net::minecraft::client::resources::model::sprite::Material;
use crate::net::minecraft::core::{BlockMath, Direction};

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

    /// 烘焙一个模型面。返回的 `positions`/`uvs` 都已归一化到 0..1。
    pub fn bake_quad(
        from: [f32; 3],
        to: [f32; 3],
        face: &CuboidFace,
        material: Material,
        facing: Direction,
        variant: &Variant,
        element_rotation: Option<&CuboidRotation>,
        shade_direction_override: Option<Direction>,
    ) -> BakedQuad {
        let uvs = face
            .uvs
            .unwrap_or_else(|| Self::default_face_uv(from, to, facing));

        let variant_matrix = variant_rotation_matrix(variant);
        let uv_transform = if variant.uvlock {
            BlockMath::get_face_transformation(variant_matrix, facing)
                .inverse()
                .unwrap_or(Mat3::IDENTITY)
        } else {
            Mat3::IDENTITY
        };
        let element =
            element_rotation.map(|rotation| (rotation.origin, rotation.transfer_matrix()));

        let mut positions = face_vertices(from, to, facing)
            .map(|position| [position[0] / 16.0, position[1] / 16.0, position[2] / 16.0]);
        for position in &mut positions {
            if let Some((origin, matrix)) = &element {
                *position = rotate_around(*position, *origin, *matrix);
            }
            if !variant_matrix.is_identity() {
                *position = rotate_around(*position, [0.5, 0.5, 0.5], variant_matrix);
            }
        }

        let mut vertex_uvs = [[0.0f32; 2]; 4];
        for (index, uv) in vertex_uvs.iter_mut().enumerate() {
            let rotated = (index + face.rotation_shift as usize) % 4;
            let raw_u = uvs.get_vertex_u(rotated) / 16.0;
            let raw_v = uvs.get_vertex_v(rotated) / 16.0;
            if uv_transform.is_identity() {
                *uv = [raw_u, raw_v];
            } else {
                let transformed = uv_transform.transform_point([raw_u - 0.5, raw_v - 0.5, 0.0]);
                *uv = [transformed[0] + 0.5, transformed[1] + 0.5];
            }
        }

        let direction = calculate_facing(&positions).unwrap_or(Direction::UP);
        let cull_face = face
            .cull_for_direction
            .map(|cull| rotate_direction(variant_matrix, cull));

        BakedQuad {
            positions,
            uvs: vertex_uvs,
            direction,
            shade_direction: shade_direction_override.unwrap_or(direction),
            cull_face,
            tint_index: face.tint_index,
            sprite: material.sprite,
            force_translucent: material.force_translucent,
        }
    }
}

/// `Quadrant.fromXYZAngles`：正象限对应 JOML 的负角度旋转。
pub fn variant_rotation_matrix(variant: &Variant) -> Mat3 {
    Mat3::rotation_xyz_degrees(
        -(variant.x as f32),
        -(variant.y as f32),
        -(variant.z as f32),
    )
}

fn rotate_around(point: [f32; 3], origin: [f32; 3], matrix: Mat3) -> [f32; 3] {
    let local = [
        point[0] - origin[0],
        point[1] - origin[1],
        point[2] - origin[2],
    ];
    let rotated = matrix.transform_point(local);
    [
        rotated[0] + origin[0],
        rotated[1] + origin[1],
        rotated[2] + origin[2],
    ]
}

fn rotate_direction(matrix: Mat3, direction: Direction) -> Direction {
    if matrix.is_identity() {
        return direction;
    }
    let step = direction.get_step();
    let rotated = matrix.transform_direction([
        step.get_x() as f32,
        step.get_y() as f32,
        step.get_z() as f32,
    ]);
    Direction::get_approximate_nearest(rotated[0], rotated[1], rotated[2])
}

fn calculate_facing(positions: &[[f32; 3]; 4]) -> Option<Direction> {
    let edge_a = [
        positions[1][0] - positions[0][0],
        positions[1][1] - positions[0][1],
        positions[1][2] - positions[0][2],
    ];
    let edge_b = [
        positions[2][0] - positions[0][0],
        positions[2][1] - positions[0][1],
        positions[2][2] - positions[0][2],
    ];
    let mut normal = [
        edge_a[1] * edge_b[2] - edge_a[2] * edge_b[1],
        edge_a[2] * edge_b[0] - edge_a[0] * edge_b[2],
        edge_a[0] * edge_b[1] - edge_a[1] * edge_b[0],
    ];
    let length = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
    if !length.is_finite() || length < 1.0e-9 {
        return None;
    }
    normal = [normal[0] / length, normal[1] / length, normal[2] / length];

    let mut best = None;
    let mut closest_product = 0.0f32;
    for candidate in Direction::VALUES {
        let step = candidate.get_step();
        let product = normal[0] * step.get_x() as f32
            + normal[1] * step.get_y() as f32
            + normal[2] * step.get_z() as f32;
        if product >= 0.0 && product > closest_product {
            closest_product = product;
            best = Some(candidate);
        }
    }
    best
}

/// 每个面的 4 个顶点，顺序对应 `FaceInfo` 与 UV 的
/// (min,min)/(min,max)/(max,max)/(max,min)。
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::minecraft::core::Axis;
    use crate::net::minecraft::resources::Identifier;

    fn stone_material() -> Material {
        Material::new(Identifier::parse("minecraft:block/stone").unwrap(), false)
    }

    fn default_variant() -> Variant {
        Variant {
            model: Identifier::parse("minecraft:block/stone").unwrap(),
            x: 0,
            y: 0,
            z: 0,
            uvlock: false,
            weight: 1,
        }
    }

    fn face() -> CuboidFace {
        CuboidFace {
            cull_for_direction: Some(Direction::NORTH),
            tint_index: -1,
            texture: "#side".into(),
            uvs: None,
            rotation_shift: 0,
        }
    }

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
        let mut variant = default_variant();
        variant.y = 90;
        let quad = FaceBakery::bake_quad(
            [0.0, 0.0, 0.0],
            [16.0, 16.0, 16.0],
            &face(),
            stone_material(),
            Direction::NORTH,
            &variant,
            None,
            None,
        );
        assert_eq!(quad.direction, Direction::EAST);
        assert_eq!(quad.cull_face, Some(Direction::EAST));
        // NORTH 面原本在 z=0；变体 y=90 把该面转到 x=1（EAST 侧）。
        for position in quad.positions {
            assert!((position[0] - 1.0).abs() <= 1.0e-6);
        }
    }

    #[test]
    fn x_rotation_moves_up_to_north() {
        let mut variant = default_variant();
        variant.x = 90;
        let quad = FaceBakery::bake_quad(
            [0.0, 0.0, 0.0],
            [16.0, 16.0, 16.0],
            &CuboidFace {
                cull_for_direction: Some(Direction::UP),
                tint_index: -1,
                texture: "#top".into(),
                uvs: None,
                rotation_shift: 0,
            },
            stone_material(),
            Direction::UP,
            &variant,
            None,
            None,
        );
        // BLOCK_ROT_X_90 将 +Y 映射到 -Z。
        assert_eq!(quad.direction, Direction::NORTH);
        assert_eq!(quad.cull_face, Some(Direction::NORTH));
    }

    #[test]
    fn element_rotation_moves_geometry_around_origin() {
        let rotation = CuboidRotation {
            origin: [0.5, 0.5, 0.5],
            axis: Some(Axis::Y),
            angle: 90.0,
            euler: None,
            rescale: false,
        };
        let quad = FaceBakery::bake_quad(
            [0.0, 0.0, 0.0],
            [16.0, 16.0, 16.0],
            &face(),
            stone_material(),
            Direction::NORTH,
            &default_variant(),
            Some(&rotation),
            None,
        );
        // JOML 的正角度绕 Y 旋转把 NORTH(-Z) 转到 WEST(-X)。
        assert_eq!(quad.direction, Direction::WEST);
    }

    #[test]
    fn uvlock_keeps_uv_orientation_for_rotated_variant() {
        let mut variant = default_variant();
        variant.y = 90;
        variant.uvlock = true;
        let quad = FaceBakery::bake_quad(
            [0.0, 0.0, 0.0],
            [16.0, 16.0, 16.0],
            &CuboidFace {
                cull_for_direction: None,
                tint_index: -1,
                texture: "#side".into(),
                uvs: None,
                rotation_shift: 0,
            },
            stone_material(),
            Direction::NORTH,
            &variant,
            None,
            None,
        );
        let min_u = quad.uvs.iter().map(|uv| uv[0]).fold(f32::MAX, f32::min);
        let min_v = quad.uvs.iter().map(|uv| uv[1]).fold(f32::MAX, f32::min);
        assert!((min_u - 0.0).abs() < 1.0e-5);
        assert!((min_v - 0.0).abs() < 1.0e-5);
    }
}
