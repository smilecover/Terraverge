//! 对应 `net/minecraft/world/phys/Vec3.java`。
//!
//! Java 字段是 `public final double x/y/z`；Rust 侧同样公开只读字段，
//! 派生运算返回新值，保持不可变语义。

use crate::net::minecraft::core::{BlockPos, Position, Vec3i};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    pub const ZERO: Vec3 = Vec3::new(0.0, 0.0, 0.0);
    pub const X_AXIS: Vec3 = Vec3::new(1.0, 0.0, 0.0);
    pub const Y_AXIS: Vec3 = Vec3::new(0.0, 1.0, 0.0);
    pub const Z_AXIS: Vec3 = Vec3::new(0.0, 0.0, 1.0);

    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub fn at_lower_corner_of(pos: Vec3i) -> Self {
        Self::new(pos.get_x() as f64, pos.get_y() as f64, pos.get_z() as f64)
    }

    pub fn at_lower_corner_with_offset(pos: Vec3i, x: f64, y: f64, z: f64) -> Self {
        Self::new(
            pos.get_x() as f64 + x,
            pos.get_y() as f64 + y,
            pos.get_z() as f64 + z,
        )
    }

    pub fn at_center_of(pos: Vec3i) -> Self {
        Self::at_lower_corner_with_offset(pos, 0.5, 0.5, 0.5)
    }

    pub fn at_center_of_with_y(pos: Vec3i, y: f64) -> Self {
        Self::at_lower_corner_with_offset(pos, 0.5, y, 0.5)
    }

    pub fn at_bottom_center_of(pos: Vec3i) -> Self {
        Self::at_lower_corner_with_offset(pos, 0.5, 0.0, 0.5)
    }

    pub fn up_from_bottom_center_of(pos: Vec3i, y_offset: f64) -> Self {
        Self::at_lower_corner_with_offset(pos, 0.5, y_offset, 0.5)
    }

    pub fn vector_to(self, vec: Vec3) -> Vec3 {
        Vec3::new(vec.x - self.x, vec.y - self.y, vec.z - self.z)
    }

    pub fn normalize(self) -> Vec3 {
        let length = self.length();
        if length < 1.0e-4 {
            Vec3::ZERO
        } else {
            self.scale(1.0 / length)
        }
    }

    pub fn dot(self, vec: Vec3) -> f64 {
        self.x * vec.x + self.y * vec.y + self.z * vec.z
    }

    pub fn cross(self, vec: Vec3) -> Vec3 {
        Vec3::new(
            self.y * vec.z - self.z * vec.y,
            self.z * vec.x - self.x * vec.z,
            self.x * vec.y - self.y * vec.x,
        )
    }

    pub fn subtract_vec(self, vec: Vec3) -> Vec3 {
        self.subtract_xyz(vec.x, vec.y, vec.z)
    }

    pub fn subtract(self, value: f64) -> Vec3 {
        self.subtract_xyz(value, value, value)
    }

    pub fn subtract_xyz(self, x: f64, y: f64, z: f64) -> Vec3 {
        Vec3::new(self.x - x, self.y - y, self.z - z)
    }

    pub fn add_vec(self, vec: Vec3) -> Vec3 {
        self.add_xyz(vec.x, vec.y, vec.z)
    }

    pub fn add(self, value: f64) -> Vec3 {
        self.add_xyz(value, value, value)
    }

    pub fn add_xyz(self, x: f64, y: f64, z: f64) -> Vec3 {
        Vec3::new(self.x + x, self.y + y, self.z + z)
    }

    pub fn closer_than(self, pos: &impl Position, distance: f64) -> bool {
        self.closer_than_xyz(pos.x(), pos.y(), pos.z(), distance)
    }

    pub fn closer_than_vec(self, vec: Vec3, distance: f64) -> bool {
        self.closer_than_xyz(vec.x, vec.y, vec.z, distance)
    }

    pub fn closer_than_xyz(self, x: f64, y: f64, z: f64, distance: f64) -> bool {
        let dx = self.x - x;
        let dy = self.y - y;
        let dz = self.z - z;
        dx * dx + dy * dy + dz * dz < distance * distance
    }

    pub fn distance_to(self, vec: Vec3) -> f64 {
        self.distance_to_sqr(vec).sqrt()
    }

    pub fn distance_to_sqr(self, vec: Vec3) -> f64 {
        self.distance_to_sqr_xyz(vec.x, vec.y, vec.z)
    }

    pub fn distance_to_sqr_xyz(self, x: f64, y: f64, z: f64) -> f64 {
        let dx = self.x - x;
        let dy = self.y - y;
        let dz = self.z - z;
        dx * dx + dy * dy + dz * dz
    }

    pub fn closer_than_xz_y(self, vec: Vec3, distance_xz: f64, distance_y: f64) -> bool {
        let dx = self.x - vec.x;
        let dz = self.z - vec.z;
        let dy = self.y - vec.y;
        dx * dx + dz * dz < distance_xz * distance_xz && dy.abs() < distance_y
    }

    pub fn scale(self, scale: f64) -> Vec3 {
        Vec3::new(self.x * scale, self.y * scale, self.z * scale)
    }

    pub fn reverse(self) -> Vec3 {
        self.scale(-1.0)
    }

    pub fn multiply_vec(self, scale: Vec3) -> Vec3 {
        self.multiply_xyz(scale.x, scale.y, scale.z)
    }

    pub fn multiply_xyz(self, x_scale: f64, y_scale: f64, z_scale: f64) -> Vec3 {
        Vec3::new(self.x * x_scale, self.y * y_scale, self.z * z_scale)
    }

    pub fn horizontal(self) -> Vec3 {
        Vec3::new(self.x, 0.0, self.z)
    }

    pub fn length(self) -> f64 {
        self.length_sqr().sqrt()
    }

    pub fn length_sqr(self) -> f64 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    pub fn horizontal_distance_sqr(self) -> f64 {
        self.x * self.x + self.z * self.z
    }

    pub fn horizontal_distance(self) -> f64 {
        self.horizontal_distance_sqr().sqrt()
    }

    pub fn distance_to_sqr_xz(self, vec: Vec3) -> f64 {
        let dx = self.x - vec.x;
        let dz = self.z - vec.z;
        dx * dx + dz * dz
    }

    pub fn distance_to_xz(self, vec: Vec3) -> f64 {
        self.distance_to_sqr_xz(vec).sqrt()
    }

    pub fn lerp(self, other: Vec3, delta: f64) -> Vec3 {
        Vec3::new(
            crate::net::minecraft::util::mth::Mth::lerp_f64(delta, self.x, other.x),
            crate::net::minecraft::util::mth::Mth::lerp_f64(delta, self.y, other.y),
            crate::net::minecraft::util::mth::Mth::lerp_f64(delta, self.z, other.z),
        )
    }

    pub fn x_offset(self, dx: f64) -> Vec3 {
        Vec3::new(self.x + dx, self.y, self.z)
    }

    pub fn y_offset(self, dy: f64) -> Vec3 {
        Vec3::new(self.x, self.y + dy, self.z)
    }

    pub fn z_offset(self, dz: f64) -> Vec3 {
        Vec3::new(self.x, self.y, self.z + dz)
    }

    pub fn with_x(self, x: f64) -> Vec3 {
        Vec3::new(x, self.y, self.z)
    }

    pub fn with_y(self, y: f64) -> Vec3 {
        Vec3::new(self.x, y, self.z)
    }

    pub fn with_z(self, z: f64) -> Vec3 {
        Vec3::new(self.x, self.y, z)
    }

    /// Java `Vec3.atCenterOf(BlockPos)` 等静态方法的 BlockPos 版本。
    pub fn at_center_of_pos(pos: BlockPos) -> Self {
        Self::at_center_of(pos.to_vec3i())
    }

    pub fn block_position(self) -> BlockPos {
        BlockPos::containing(self.x, self.y, self.z)
    }
}

impl Position for Vec3 {
    fn x(&self) -> f64 {
        self.x
    }

    fn y(&self) -> f64 {
        self.y
    }

    fn z(&self) -> f64 {
        self.z
    }
}

impl From<Vec3i> for Vec3 {
    fn from(value: Vec3i) -> Self {
        Self::new(
            value.get_x() as f64,
            value.get_y() as f64,
            value.get_z() as f64,
        )
    }
}

impl std::ops::Add for Vec3 {
    type Output = Vec3;

    fn add(self, rhs: Vec3) -> Self::Output {
        self.add_vec(rhs)
    }
}

impl std::ops::Sub for Vec3 {
    type Output = Vec3;

    fn sub(self, rhs: Vec3) -> Self::Output {
        self.subtract_vec(rhs)
    }
}

impl std::ops::Mul<f64> for Vec3 {
    type Output = Vec3;

    fn mul(self, rhs: f64) -> Self::Output {
        self.scale(rhs)
    }
}

impl std::ops::Neg for Vec3 {
    type Output = Vec3;

    fn neg(self) -> Self::Output {
        self.reverse()
    }
}

impl std::fmt::Display for Vec3 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_vector_math_matches_java() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(4.0, 5.0, 6.0);
        assert_eq!(a.add_vec(b), Vec3::new(5.0, 7.0, 9.0));
        assert_eq!(a.dot(b), 32.0);
        assert_eq!(a.cross(b), Vec3::new(-3.0, 6.0, -3.0));
        assert_eq!(a.length_sqr(), 14.0);
    }

    #[test]
    fn normalize_handles_zero() {
        assert_eq!(Vec3::ZERO.normalize(), Vec3::ZERO);
        assert!((Vec3::new(0.0, 5.0, 0.0).normalize().length() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn block_center_helpers() {
        let pos = Vec3i::new(2, 3, 4);
        assert_eq!(Vec3::at_center_of(pos), Vec3::new(2.5, 3.5, 4.5));
        assert_eq!(Vec3::at_bottom_center_of(pos), Vec3::new(2.5, 3.0, 4.5));
    }
}
