//! 对应 `net/minecraft/core/Vec3i.java`：不可变整数三维向量。
//!
//! Java 的 `offset(Vec3i)` 与 `offset(int,int,int)` 是重载；Rust 不支持
//! 重载，按 `docs/PORTING.md` 的约定，向量参数版本加 `_vec` 后缀。

use super::position::Position;
use super::section_pos::SectionPos;
use super::{Axis, Direction};
use crate::net::minecraft::util::mth::Mth;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Vec3i {
    x: i32,
    y: i32,
    z: i32,
}

impl Vec3i {
    pub const ZERO: Vec3i = Vec3i::new(0, 0, 0);

    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    pub const fn get_x(&self) -> i32 {
        self.x
    }

    pub const fn get_y(&self) -> i32 {
        self.y
    }

    pub const fn get_z(&self) -> i32 {
        self.z
    }

    pub const fn offset(self, x: i32, y: i32, z: i32) -> Self {
        Self::new(self.x + x, self.y + y, self.z + z)
    }

    pub const fn offset_vec(self, vec: Vec3i) -> Self {
        self.offset(vec.x, vec.y, vec.z)
    }

    pub const fn subtract(self, vec: Vec3i) -> Self {
        self.offset(-vec.x, -vec.y, -vec.z)
    }

    pub const fn multiply(self, scale: i32) -> Self {
        Self::new(self.x * scale, self.y * scale, self.z * scale)
    }

    pub const fn multiply_xyz(self, x_scale: i32, y_scale: i32, z_scale: i32) -> Self {
        Self::new(self.x * x_scale, self.y * y_scale, self.z * z_scale)
    }

    pub const fn above(self) -> Self {
        self.above_n(1)
    }

    pub const fn above_n(self, steps: i32) -> Self {
        Self::new(self.x, self.y + steps, self.z)
    }

    pub const fn below(self) -> Self {
        self.below_n(1)
    }

    pub const fn below_n(self, steps: i32) -> Self {
        Self::new(self.x, self.y - steps, self.z)
    }

    pub const fn north(self) -> Self {
        self.north_n(1)
    }

    pub const fn north_n(self, steps: i32) -> Self {
        Self::new(self.x, self.y, self.z - steps)
    }

    pub const fn south(self) -> Self {
        self.south_n(1)
    }

    pub const fn south_n(self, steps: i32) -> Self {
        Self::new(self.x, self.y, self.z + steps)
    }

    pub const fn west(self) -> Self {
        self.west_n(1)
    }

    pub const fn west_n(self, steps: i32) -> Self {
        Self::new(self.x - steps, self.y, self.z)
    }

    pub const fn east(self) -> Self {
        self.east_n(1)
    }

    pub const fn east_n(self, steps: i32) -> Self {
        Self::new(self.x + steps, self.y, self.z)
    }

    pub const fn relative(self, direction: Direction) -> Self {
        self.relative_n(direction, 1)
    }

    pub const fn relative_n(self, direction: Direction, steps: i32) -> Self {
        let (x, y, z) = direction.step_tuple();
        Self::new(self.x + x * steps, self.y + y * steps, self.z + z * steps)
    }

    pub const fn relative_axis(self, axis: Axis, steps: i32) -> Self {
        let x_step = if matches!(axis, Axis::X) { steps } else { 0 };
        let y_step = if matches!(axis, Axis::Y) { steps } else { 0 };
        let z_step = if matches!(axis, Axis::Z) { steps } else { 0 };
        Self::new(self.x + x_step, self.y + y_step, self.z + z_step)
    }

    pub const fn cross(self, up_vector: Vec3i) -> Self {
        Self::new(
            self.y * up_vector.z - self.z * up_vector.y,
            self.z * up_vector.x - self.x * up_vector.z,
            self.x * up_vector.y - self.y * up_vector.x,
        )
    }

    pub fn closer_than(self, pos: Vec3i, distance: f64) -> bool {
        self.dist_sqr(pos) < Mth::square(distance)
    }

    pub fn closer_to_center_than(self, pos: &impl Position, distance: f64) -> bool {
        self.dist_to_center_sqr(pos) < Mth::square(distance)
    }

    pub fn dist_sqr(self, pos: Vec3i) -> f64 {
        self.dist_to_low_corner_sqr(pos.x as f64, pos.y as f64, pos.z as f64)
    }

    pub fn dist_to_center_sqr(self, pos: &impl Position) -> f64 {
        self.dist_to_center_sqr_xyz(pos.x(), pos.y(), pos.z())
    }

    pub fn dist_to_center_sqr_xyz(self, x: f64, y: f64, z: f64) -> f64 {
        let dx = self.x as f64 + 0.5 - x;
        let dy = self.y as f64 + 0.5 - y;
        let dz = self.z as f64 + 0.5 - z;
        dx * dx + dy * dy + dz * dz
    }

    pub fn dist_to_low_corner_sqr(self, x: f64, y: f64, z: f64) -> f64 {
        let dx = self.x as f64 - x;
        let dy = self.y as f64 - y;
        let dz = self.z as f64 - z;
        dx * dx + dy * dy + dz * dz
    }

    pub fn dist_manhattan(self, pos: Vec3i) -> i32 {
        let xd = (pos.x - self.x).abs();
        let yd = (pos.y - self.y).abs();
        let zd = (pos.z - self.z).abs();
        xd.saturating_add(yd).saturating_add(zd)
    }

    pub fn dist_chessboard(self, pos: Vec3i) -> i32 {
        let xd = (self.x - pos.x).abs();
        let yd = (self.y - pos.y).abs();
        let zd = (self.z - pos.z).abs();
        xd.max(yd).max(zd)
    }

    pub fn differs_horizontally(self, pos: Vec3i) -> bool {
        self.x != pos.x || self.z != pos.z
    }

    pub const fn get_axis(self, axis: Axis) -> i32 {
        axis.choose_i32(self.x, self.y, self.z)
    }

    pub fn section_pos(self) -> SectionPos {
        SectionPos::of_vec3i(self)
    }

    pub fn to_short_string(self) -> String {
        format!("{}, {}, {}", self.x, self.y, self.z)
    }
}

impl PartialOrd for Vec3i {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Vec3i {
    /// Java `compareTo`：先 y，再 z，最后 x。
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.y
            .cmp(&other.y)
            .then_with(|| self.z.cmp(&other.z))
            .then_with(|| self.x.cmp(&other.x))
    }
}

impl std::fmt::Display for Vec3i {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Vec3i{{x={}, y={}, z={}}}", self.x, self.y, self.z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offsets_follow_directions() {
        let origin = Vec3i::ZERO;
        assert_eq!(origin.north(), Vec3i::new(0, 0, -1));
        assert_eq!(origin.east_n(3), Vec3i::new(3, 0, 0));
        assert_eq!(origin.relative(Direction::DOWN), Vec3i::new(0, -1, 0));
    }

    #[test]
    fn ordering_matches_java() {
        assert!(Vec3i::new(0, 1, 0) < Vec3i::new(99, 2, 0));
        assert!(Vec3i::new(10, 5, 1) < Vec3i::new(0, 5, 2));
        assert!(Vec3i::new(1, 5, 2) < Vec3i::new(2, 5, 2));
    }
}
