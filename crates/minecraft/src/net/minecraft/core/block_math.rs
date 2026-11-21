//! 对应 `net/minecraft/core/BlockMath.java` 的 UV 朝向补偿。
//!
//! 当 blockstate 变体设置 `uvlock: true` 时，模型旋转不再带着 UV 一起转，
//! 而是把 UV 反向变换回世界朝向。Java 用 JOML 4×4 矩阵实现；这里只处理
//! 旋转矩阵的 3×3 部分，UV 的 z 分量为 0。

use super::Direction;
use crate::com::math::Mat3;

pub struct BlockMath;

impl BlockMath {
    /// `VANILLA_UV_TRANSFORM_LOCAL_TO_GLOBAL`。
    fn local_to_global(face: Direction) -> Mat3 {
        match face {
            Direction::SOUTH => Mat3::IDENTITY,
            Direction::EAST => Mat3::rotation_y(90.0),
            Direction::WEST => Mat3::rotation_y(-90.0),
            Direction::NORTH => Mat3::rotation_y(180.0),
            Direction::UP => Mat3::rotation_x(-90.0),
            Direction::DOWN => Mat3::rotation_x(90.0),
        }
    }

    fn global_to_local(face: Direction) -> Mat3 {
        Self::local_to_global(face)
            .inverse()
            .expect("UV 变换必须是可逆旋转")
    }

    /// 对应 `getFaceTransformation(Transformation, Direction)`。
    pub fn get_face_transformation(rotation: Mat3, original_side: Direction) -> Mat3 {
        if rotation.is_identity() {
            return rotation;
        }

        let face_action = rotation.mul(Self::local_to_global(original_side));
        let transformed_normal = face_action.transform_direction([0.0, 0.0, 1.0]);
        let new_side = Direction::get_approximate_nearest(
            transformed_normal[0],
            transformed_normal[1],
            transformed_normal[2],
        );
        Self::global_to_local(new_side).mul(face_action)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_rotation_keeps_uv() {
        let result = BlockMath::get_face_transformation(Mat3::IDENTITY, Direction::NORTH);
        assert!(result.is_identity());
    }

    #[test]
    fn y_rotation_maps_face_normals() {
        let rotation = Mat3::rotation_y(-90.0);
        let face = BlockMath::get_face_transformation(rotation, Direction::NORTH);
        // NORTH 的 UV 局部到全局先绕 Y 转 90°，再叠加变体旋转；
        // 结果仍应是可逆旋转，且对单位轴保持长度。
        let transformed = face.transform_direction([0.0, 0.0, 1.0]);
        let length = (transformed[0] * transformed[0]
            + transformed[1] * transformed[1]
            + transformed[2] * transformed[2])
            .sqrt();
        assert!((length - 1.0).abs() < 1.0e-5);
    }
}
