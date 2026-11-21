//! 对应 `net/minecraft/core/BlockPos.java`。
//!
//! 64 位打包布局与 Java 完全一致：
//!
//! ```text
//! bit 63..38 : x（26 位，含符号）
//! bit 37..12 : z（26 位，含符号）
//! bit 11..0  : y（12 位，含符号）
//! ```

use super::position::Position;
use super::{Axis, Direction, Vec3i};
use crate::net::minecraft::util::mth::Mth;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct BlockPos {
    x: i32,
    y: i32,
    z: i32,
}

impl BlockPos {
    pub const ZERO: BlockPos = BlockPos::new(0, 0, 0);

    pub const PACKED_HORIZONTAL_LENGTH: u32 =
        1 + Mth::log2(Mth::smallest_encompassing_power_of_two(30_000_000)) as u32;
    pub const PACKED_Y_LENGTH: u32 = 64 - 2 * Self::PACKED_HORIZONTAL_LENGTH;

    const PACKED_X_MASK: i64 = (1i64 << Self::PACKED_HORIZONTAL_LENGTH) - 1;
    const PACKED_Y_MASK: i64 = (1i64 << Self::PACKED_Y_LENGTH) - 1;
    const PACKED_Z_MASK: i64 = (1i64 << Self::PACKED_HORIZONTAL_LENGTH) - 1;

    const Y_OFFSET: u32 = 0;
    const Z_OFFSET: u32 = Self::PACKED_Y_LENGTH;
    const X_OFFSET: u32 = Self::PACKED_Y_LENGTH + Self::PACKED_HORIZONTAL_LENGTH;

    pub const MAX_HORIZONTAL_COORDINATE: i32 = (1i32 << Self::PACKED_HORIZONTAL_LENGTH) / 2 - 1;

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

    pub fn containing(x: f64, y: f64, z: f64) -> Self {
        Self::new(x.floor() as i32, y.floor() as i32, z.floor() as i32)
    }

    pub fn containing_position(pos: &impl Position) -> Self {
        Self::containing(pos.x(), pos.y(), pos.z())
    }

    pub fn min(a: Self, b: Self) -> Self {
        Self::new(a.x.min(b.x), a.y.min(b.y), a.z.min(b.z))
    }

    pub fn max(a: Self, b: Self) -> Self {
        Self::new(a.x.max(b.x), a.y.max(b.y), a.z.max(b.z))
    }

    pub const fn as_long(self) -> i64 {
        Self::as_long_xyz(self.x, self.y, self.z)
    }

    pub const fn as_long_xyz(x: i32, y: i32, z: i32) -> i64 {
        let mut node = 0i64;
        node |= (x as i64 & Self::PACKED_X_MASK) << Self::X_OFFSET;
        node |= (y as i64 & Self::PACKED_Y_MASK) << Self::Y_OFFSET;
        node | ((z as i64 & Self::PACKED_Z_MASK) << Self::Z_OFFSET)
    }

    pub const fn get_flat_index(neighbor_block_node: i64) -> i64 {
        neighbor_block_node & -16
    }

    pub const fn of(node: i64) -> Self {
        Self::new(
            Self::unpack_x(node),
            Self::unpack_y(node),
            Self::unpack_z(node),
        )
    }

    pub const fn unpack_x(node: i64) -> i32 {
        (node >> Self::X_OFFSET) as i32
    }

    pub const fn unpack_y(node: i64) -> i32 {
        ((node << (64 - Self::PACKED_Y_LENGTH)) >> (64 - Self::PACKED_Y_LENGTH)) as i32
    }

    pub const fn unpack_z(node: i64) -> i32 {
        ((node << (64 - Self::PACKED_HORIZONTAL_LENGTH - Self::Z_OFFSET))
            >> (64 - Self::PACKED_HORIZONTAL_LENGTH)) as i32
    }

    pub const fn offset(self, x: i32, y: i32, z: i32) -> Self {
        Self::new(self.x + x, self.y + y, self.z + z)
    }

    pub const fn offset_vec(self, vec: Vec3i) -> Self {
        self.offset(vec.get_x(), vec.get_y(), vec.get_z())
    }

    pub const fn subtract(self, vec: Vec3i) -> Self {
        self.offset(-vec.get_x(), -vec.get_y(), -vec.get_z())
    }

    pub const fn multiply(self, scale: i32) -> Self {
        Self::new(self.x * scale, self.y * scale, self.z * scale)
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
        Self::new(
            self.x + axis.choose_i32(steps, 0, 0),
            self.y + axis.choose_i32(0, steps, 0),
            self.z + axis.choose_i32(0, 0, steps),
        )
    }

    pub const fn to_vec3i(self) -> Vec3i {
        Vec3i::new(self.x, self.y, self.z)
    }

    /// `Vec3i` 上的同名方法在这里返回 `BlockPos`，与 Java 的协变返回一致。
    pub fn as_vec3i(&self) -> Vec3i {
        Vec3i::new(self.x, self.y, self.z)
    }

    pub fn dist_sqr_to_center(self, pos: BlockPos) -> f64 {
        let dx = self.x as f64 + 0.5 - (pos.x as f64 + 0.5);
        let dy = self.y as f64 + 0.5 - (pos.y as f64 + 0.5);
        let dz = self.z as f64 + 0.5 - (pos.z as f64 + 0.5);
        dx * dx + dy * dy + dz * dz
    }
}

impl Position for BlockPos {
    fn x(&self) -> f64 {
        self.x as f64
    }

    fn y(&self) -> f64 {
        self.y as f64
    }

    fn z(&self) -> f64 {
        self.z as f64
    }
}

impl From<Vec3i> for BlockPos {
    fn from(value: Vec3i) -> Self {
        Self::new(value.get_x(), value.get_y(), value.get_z())
    }
}

impl From<BlockPos> for Vec3i {
    fn from(value: BlockPos) -> Self {
        Vec3i::new(value.x, value.y, value.z)
    }
}

impl std::ops::Add<Vec3i> for BlockPos {
    type Output = BlockPos;

    fn add(self, rhs: Vec3i) -> Self::Output {
        self.offset_vec(rhs)
    }
}

impl std::ops::Sub<Vec3i> for BlockPos {
    type Output = BlockPos;

    fn sub(self, rhs: Vec3i) -> Self::Output {
        self.subtract(rhs)
    }
}

impl std::fmt::Display for BlockPos {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BlockPos{{x={}, y={}, z={}}}", self.x, self.y, self.z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packing_roundtrips() {
        let samples = [
            BlockPos::new(0, 0, 0),
            BlockPos::new(30_000_000, 2047, -30_000_000),
            BlockPos::new(-1, -64, 1),
            BlockPos::new(12_345, 320, -98_765),
        ];
        for pos in samples {
            assert_eq!(BlockPos::of(pos.as_long()), pos);
        }
    }

    #[test]
    fn packed_layout_matches_java_offsets() {
        assert_eq!(BlockPos::PACKED_HORIZONTAL_LENGTH, 26);
        assert_eq!(BlockPos::PACKED_Y_LENGTH, 12);
        let node = BlockPos::as_long_xyz(1, -1, 2);
        assert_eq!(BlockPos::unpack_x(node), 1);
        assert_eq!(BlockPos::unpack_y(node), -1);
        assert_eq!(BlockPos::unpack_z(node), 2);
    }

    #[test]
    fn offsets_are_immutable() {
        let pos = BlockPos::new(4, 5, 6);
        assert_eq!(pos.above(), BlockPos::new(4, 6, 6));
        assert_eq!(pos.east_n(2), BlockPos::new(6, 5, 6));
        assert_eq!(pos, BlockPos::new(4, 5, 6));
    }
}
