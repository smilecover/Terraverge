//! 对应 `net/minecraft/world/phys/AABB.java` 的碰撞盒部分。

use super::Vec3;
use crate::net::minecraft::core::Position;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AABB {
    pub min_x: f64,
    pub min_y: f64,
    pub min_z: f64,
    pub max_x: f64,
    pub max_y: f64,
    pub max_z: f64,
}

impl AABB {
    pub fn new(min_x: f64, min_y: f64, min_z: f64, max_x: f64, max_y: f64, max_z: f64) -> Self {
        Self {
            min_x,
            min_y,
            min_z,
            max_x,
            max_y,
            max_z,
        }
    }

    pub fn of_size(x1: f64, y1: f64, z1: f64, x2: f64, y2: f64, z2: f64) -> Self {
        Self {
            min_x: x1.min(x2),
            min_y: y1.min(y2),
            min_z: z1.min(z2),
            max_x: x1.max(x2),
            max_y: y1.max(y2),
            max_z: z1.max(z2),
        }
    }

    pub fn from_vec3(center: Vec3, half_width: f64) -> Self {
        Self::new(
            center.x - half_width,
            center.y - half_width,
            center.z - half_width,
            center.x + half_width,
            center.y + half_width,
            center.z + half_width,
        )
    }

    pub fn min_position(&self) -> Vec3 {
        Vec3::new(self.min_x, self.min_y, self.min_z)
    }

    pub fn max_position(&self) -> Vec3 {
        Vec3::new(self.max_x, self.max_y, self.max_z)
    }

    pub fn center(&self) -> Vec3 {
        Vec3::new(
            (self.min_x + self.max_x) * 0.5,
            (self.min_y + self.max_y) * 0.5,
            (self.min_z + self.max_z) * 0.5,
        )
    }

    /// 对应 `move(x,y,z)`。
    pub fn translated(&self, x: f64, y: f64, z: f64) -> Self {
        Self::new(
            self.min_x + x,
            self.min_y + y,
            self.min_z + z,
            self.max_x + x,
            self.max_y + y,
            self.max_z + z,
        )
    }

    /// 对应 `inflate(x,y,z)`。
    pub fn inflate(&self, x: f64, y: f64, z: f64) -> Self {
        Self::new(
            self.min_x - x,
            self.min_y - y,
            self.min_z - z,
            self.max_x + x,
            self.max_y + y,
            self.max_z + z,
        )
    }

    pub fn deflate(&self, x: f64) -> Self {
        self.inflate(-x, -x, -x)
    }

    /// 对应 `setMinY` / `setMaxY` 等。
    pub fn with_min_y(&self, y: f64) -> Self {
        Self::new(
            self.min_x, y, self.min_z, self.max_x, self.max_y, self.max_z,
        )
    }

    pub fn with_max_y(&self, y: f64) -> Self {
        Self::new(
            self.min_x, self.min_y, self.min_z, self.max_x, y, self.max_z,
        )
    }

    pub fn intersects(&self, other: &AABB) -> bool {
        self.min_x < other.max_x
            && self.max_x > other.min_x
            && self.min_y < other.max_y
            && self.max_y > other.min_y
            && self.min_z < other.max_z
            && self.max_z > other.min_z
    }

    pub fn intersects_xyz(
        &self,
        min_x: f64,
        min_y: f64,
        min_z: f64,
        max_x: f64,
        max_y: f64,
        max_z: f64,
    ) -> bool {
        self.min_x < max_x
            && self.max_x > min_x
            && self.min_y < max_y
            && self.max_y > min_y
            && self.min_z < max_z
            && self.max_z > min_z
    }

    pub fn contains(&self, vec: Vec3) -> bool {
        vec.x >= self.min_x
            && vec.x < self.max_x
            && vec.y >= self.min_y
            && vec.y < self.max_y
            && vec.z >= self.min_z
            && vec.z < self.max_z
    }

    pub fn contains_position(&self, pos: &impl Position) -> bool {
        pos.x() >= self.min_x
            && pos.x() < self.max_x
            && pos.y() >= self.min_y
            && pos.y() < self.max_y
            && pos.z() >= self.min_z
            && pos.z() < self.max_z
    }

    /// Java `move` 与 Rust 关键字冲突，按移植约定加 `_by` 后缀。
    pub fn move_by(&self, dx: f64, dy: f64, dz: f64) -> Self {
        self.translated(dx, dy, dz)
    }

    pub fn get_x_size(&self) -> f64 {
        self.max_x - self.min_x
    }

    pub fn get_y_size(&self) -> f64 {
        self.max_y - self.min_y
    }

    pub fn get_z_size(&self) -> f64 {
        self.max_z - self.min_z
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intersects_matches_vanilla() {
        let a = AABB::new(0.0, 0.0, 0.0, 1.0, 1.0, 1.0);
        let b = AABB::new(0.5, 0.5, 0.5, 2.0, 2.0, 2.0);
        let c = AABB::new(1.0, 1.0, 1.0, 2.0, 2.0, 2.0);
        assert!(a.intersects(&b));
        assert!(!a.intersects(&c));
    }

    #[test]
    fn inflation_and_translation() {
        let a = AABB::new(0.0, 0.0, 0.0, 1.0, 1.0, 1.0);
        assert_eq!(a.inflate(0.5, 0.0, 0.0).min_x, -0.5);
        assert_eq!(a.translated(1.0, 2.0, 3.0).max_z, 4.0);
    }
}
