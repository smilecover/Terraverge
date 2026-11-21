//! 方块模型需要的 3×3 旋转/缩放矩阵。
//!
//! Java 使用 JOML 的 `Matrix3f/Matrix4f`；这里实现移植所需的最小集合，
//! 行主序存储，`transform_point` 计算 `M * v`，`mul` 计算 `self * rhs`。
//! 只处理 f32，与模型 JSON 的精度一致。

use crate::net::minecraft::core::Axis;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mat3 {
    rows: [[f32; 3]; 3],
}

impl Mat3 {
    pub const IDENTITY: Mat3 = Mat3 {
        rows: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
    };

    pub const fn from_rows(rows: [[f32; 3]; 3]) -> Self {
        Self { rows }
    }

    pub fn from_diagonal(diagonal: [f32; 3]) -> Self {
        Self {
            rows: [
                [diagonal[0], 0.0, 0.0],
                [0.0, diagonal[1], 0.0],
                [0.0, 0.0, diagonal[2]],
            ],
        }
    }

    /// 标准右手系绕 X 轴旋转（角度制）。
    pub fn rotation_x(degrees: f32) -> Self {
        let (sin, cos) = degrees.to_radians().sin_cos();
        Self::from_rows([[1.0, 0.0, 0.0], [0.0, cos, -sin], [0.0, sin, cos]])
    }

    pub fn rotation_y(degrees: f32) -> Self {
        let (sin, cos) = degrees.to_radians().sin_cos();
        Self::from_rows([[cos, 0.0, sin], [0.0, 1.0, 0.0], [-sin, 0.0, cos]])
    }

    pub fn rotation_z(degrees: f32) -> Self {
        let (sin, cos) = degrees.to_radians().sin_cos();
        Self::from_rows([[cos, -sin, 0.0], [sin, cos, 0.0], [0.0, 0.0, 1.0]])
    }

    pub fn rotation_axis(axis: Axis, degrees: f32) -> Self {
        match axis {
            Axis::X => Self::rotation_x(degrees),
            Axis::Y => Self::rotation_y(degrees),
            Axis::Z => Self::rotation_z(degrees),
        }
    }

    /// 对应 `Quadrant.fromXYZAngles`：
    /// `Rz(z) * Ry(y) * Rx(x)`。注意方块变体的正角度对应 JOML 的负旋转，
    /// 由调用方传入 `-x/-y/-z`。
    pub fn rotation_xyz_degrees(x: f32, y: f32, z: f32) -> Self {
        Self::rotation_z(z)
            .mul(Self::rotation_y(y))
            .mul(Self::rotation_x(x))
    }

    /// 元素旋转的 `rotationZYX` 形式（JOML 语义）。
    pub fn rotation_zyx_degrees(x: f32, y: f32, z: f32) -> Self {
        Self::rotation_z(z)
            .mul(Self::rotation_y(y))
            .mul(Self::rotation_x(x))
    }

    pub fn mul(self, rhs: Self) -> Self {
        let mut rows = [[0.0f32; 3]; 3];
        for row in 0..3 {
            for column in 0..3 {
                rows[row][column] = self.rows[row][0] * rhs.rows[0][column]
                    + self.rows[row][1] * rhs.rows[1][column]
                    + self.rows[row][2] * rhs.rows[2][column];
            }
        }
        Self { rows }
    }

    pub fn transform_point(self, point: [f32; 3]) -> [f32; 3] {
        [
            self.rows[0][0] * point[0] + self.rows[0][1] * point[1] + self.rows[0][2] * point[2],
            self.rows[1][0] * point[0] + self.rows[1][1] * point[1] + self.rows[1][2] * point[2],
            self.rows[2][0] * point[0] + self.rows[2][1] * point[1] + self.rows[2][2] * point[2],
        ]
    }

    pub fn transform_direction(self, direction: [f32; 3]) -> [f32; 3] {
        self.transform_point(direction)
    }

    pub fn transpose(self) -> Self {
        let mut rows = [[0.0f32; 3]; 3];
        for row in 0..3 {
            for column in 0..3 {
                rows[row][column] = self.rows[column][row];
            }
        }
        Self { rows }
    }

    /// 仿射逆（3×3 可逆矩阵）；奇异时返回 `None`。
    pub fn inverse(self) -> Option<Self> {
        let m = self.rows;
        let determinant = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
            - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
        if determinant.abs() < 1.0e-9 {
            return None;
        }
        let inv_det = 1.0 / determinant;
        Some(Self::from_rows([
            [
                (m[1][1] * m[2][2] - m[1][2] * m[2][1]) * inv_det,
                (m[0][2] * m[2][1] - m[0][1] * m[2][2]) * inv_det,
                (m[0][1] * m[1][2] - m[0][2] * m[1][1]) * inv_det,
            ],
            [
                (m[1][2] * m[2][0] - m[1][0] * m[2][2]) * inv_det,
                (m[0][0] * m[2][2] - m[0][2] * m[2][0]) * inv_det,
                (m[0][2] * m[1][0] - m[0][0] * m[1][2]) * inv_det,
            ],
            [
                (m[1][0] * m[2][1] - m[1][1] * m[2][0]) * inv_det,
                (m[0][1] * m[2][0] - m[0][0] * m[2][1]) * inv_det,
                (m[0][0] * m[1][1] - m[0][1] * m[1][0]) * inv_det,
            ],
        ]))
    }

    pub fn is_identity(self) -> bool {
        const EPSILON: f32 = 1.0e-6;
        for row in 0..3 {
            for column in 0..3 {
                let expected = if row == column { 1.0 } else { 0.0 };
                if (self.rows[row][column] - expected).abs() > EPSILON {
                    return false;
                }
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::minecraft::core::Direction;

    #[test]
    fn variant_rotation_matches_octahedral_group() {
        // y=90: (x,y,z) -> (-z,y,x)，与 Java BLOCK_ROT_Y_90 一致。
        let matrix = Mat3::rotation_xyz_degrees(0.0, -90.0, 0.0);
        let rotated = matrix.transform_point([1.0, 0.0, 0.0]);
        assert!(rotated[0].abs() < 1.0e-5);
        assert!(rotated[1].abs() < 1.0e-5);
        assert!((rotated[2] - 1.0).abs() < 1.0e-5);
        let rotated = matrix.transform_point([0.0, 1.0, 0.0]);
        assert!((rotated[1] - 1.0).abs() < 1.0e-5);

        let north = Direction::NORTH.get_step();
        let rotated = matrix.transform_direction([
            north.get_x() as f32,
            north.get_y() as f32,
            north.get_z() as f32,
        ]);
        assert!((rotated[0] - 1.0).abs() < 1.0e-5);
        assert!(rotated[1].abs() < 1.0e-5);
        assert!(rotated[2].abs() < 1.0e-5);
    }

    #[test]
    fn inverse_roundtrips() {
        let matrix = Mat3::rotation_xyz_degrees(90.0, 90.0, 0.0);
        let inverse = matrix.inverse().unwrap();
        let point = [0.25, -0.5, 0.75];
        let result = inverse.transform_point(matrix.transform_point(point));
        for (a, b) in result.iter().zip(point.iter()) {
            assert!((a - b).abs() < 1.0e-6);
        }
    }
}
