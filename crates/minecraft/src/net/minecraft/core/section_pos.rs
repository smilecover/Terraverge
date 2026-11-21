//! 对应 `net/minecraft/core/SectionPos.java`：16³ 区块段的段坐标。
//!
//! 打包布局：x(22 位) | z(22 位) | y(20 位，位于低位)。

use super::{BlockPos, Direction, Vec3i};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct SectionPos {
    x: i32,
    y: i32,
    z: i32,
}

impl SectionPos {
    pub const SECTION_BITS: u32 = 4;
    pub const SECTION_SIZE: i32 = 16;
    pub const SECTION_BLOCK_COUNT: i32 = 4096;
    pub const SECTION_MASK: i32 = 15;
    pub const SECTION_HALF_SIZE: i32 = 8;
    pub const SECTION_MAX_INDEX: i32 = 15;

    const PACKED_X_LENGTH: u32 = 22;
    const PACKED_Y_LENGTH: u32 = 20;
    const PACKED_Z_LENGTH: u32 = 22;
    const PACKED_X_MASK: i64 = (1i64 << Self::PACKED_X_LENGTH) - 1;
    const PACKED_Y_MASK: i64 = (1i64 << Self::PACKED_Y_LENGTH) - 1;
    const PACKED_Z_MASK: i64 = (1i64 << Self::PACKED_Z_LENGTH) - 1;
    const Y_OFFSET: u32 = 0;
    const Z_OFFSET: u32 = 20;
    const X_OFFSET: u32 = 42;

    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    pub const fn of(x: i32, y: i32, z: i32) -> Self {
        Self::new(x, y, z)
    }

    pub const fn of_block_pos(pos: BlockPos) -> Self {
        Self::new(
            Self::block_to_section_coord(pos.get_x()),
            Self::block_to_section_coord(pos.get_y()),
            Self::block_to_section_coord(pos.get_z()),
        )
    }

    pub const fn of_vec3i(pos: Vec3i) -> Self {
        Self::new(
            Self::block_to_section_coord(pos.get_x()),
            Self::block_to_section_coord(pos.get_y()),
            Self::block_to_section_coord(pos.get_z()),
        )
    }

    pub const fn of_long(section_node: i64) -> Self {
        Self::new(
            Self::x(section_node),
            Self::y(section_node),
            Self::z(section_node),
        )
    }

    pub fn pos_to_section_coord(pos: f64) -> i32 {
        (pos / Self::SECTION_SIZE as f64).floor() as i32
    }

    /// Java 用 `>> 4`，负数同样向下取整。
    pub const fn block_to_section_coord(block_coord: i32) -> i32 {
        block_coord >> Self::SECTION_BITS
    }

    pub fn block_to_section_coord_f64(coord: f64) -> i32 {
        (coord.floor() as i32) >> Self::SECTION_BITS
    }

    pub const fn section_relative(block_coord: i32) -> i32 {
        block_coord & Self::SECTION_MASK
    }

    pub const fn section_relative_pos(pos: BlockPos) -> i16 {
        (((pos.get_x() & 15) << 8) | ((pos.get_z() & 15) << 4) | (pos.get_y() & 15)) as i16
    }

    pub const fn section_relative_x(relative: i16) -> i32 {
        (relative as i32 >> 8) & 15
    }

    pub const fn section_relative_y(relative: i16) -> i32 {
        (relative as i32) & 15
    }

    pub const fn section_relative_z(relative: i16) -> i32 {
        (relative as i32 >> 4) & 15
    }

    pub const fn relative_to_block_x(self, relative: i16) -> i32 {
        self.min_block_x() + Self::section_relative_x(relative)
    }

    pub const fn relative_to_block_y(self, relative: i16) -> i32 {
        self.min_block_y() + Self::section_relative_y(relative)
    }

    pub const fn relative_to_block_z(self, relative: i16) -> i32 {
        self.min_block_z() + Self::section_relative_z(relative)
    }

    pub const fn relative_to_block_pos(self, relative: i16) -> BlockPos {
        BlockPos::new(
            self.relative_to_block_x(relative),
            self.relative_to_block_y(relative),
            self.relative_to_block_z(relative),
        )
    }

    pub const fn section_to_block_coord(section_coord: i32) -> i32 {
        section_coord << Self::SECTION_BITS
    }

    pub const fn section_to_block_coord_with_offset(section_coord: i32, offset: i32) -> i32 {
        Self::section_to_block_coord(section_coord) + offset
    }

    pub const fn x(section_node: i64) -> i32 {
        (section_node >> Self::X_OFFSET) as i32
    }

    pub const fn y(section_node: i64) -> i32 {
        ((section_node << (64 - Self::PACKED_Y_LENGTH)) >> (64 - Self::PACKED_Y_LENGTH)) as i32
    }

    pub const fn z(section_node: i64) -> i32 {
        ((section_node << (64 - Self::PACKED_Z_LENGTH - Self::Z_OFFSET))
            >> (64 - Self::PACKED_Z_LENGTH)) as i32
    }

    pub const fn x_coord(self) -> i32 {
        self.x
    }

    pub const fn y_coord(self) -> i32 {
        self.y
    }

    pub const fn z_coord(self) -> i32 {
        self.z
    }

    pub const fn min_block_x(self) -> i32 {
        Self::section_to_block_coord(self.x)
    }

    pub const fn min_block_y(self) -> i32 {
        Self::section_to_block_coord(self.y)
    }

    pub const fn min_block_z(self) -> i32 {
        Self::section_to_block_coord(self.z)
    }

    pub const fn max_block_x(self) -> i32 {
        Self::section_to_block_coord_with_offset(self.x, 15)
    }

    pub const fn max_block_y(self) -> i32 {
        Self::section_to_block_coord_with_offset(self.y, 15)
    }

    pub const fn max_block_z(self) -> i32 {
        Self::section_to_block_coord_with_offset(self.z, 15)
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

    pub const fn block_to_section(block_node: i64) -> i64 {
        Self::as_long_xyz(
            Self::block_to_section_coord(BlockPos::unpack_x(block_node)),
            Self::block_to_section_coord(BlockPos::unpack_y(block_node)),
            Self::block_to_section_coord(BlockPos::unpack_z(block_node)),
        )
    }

    pub const fn origin(self) -> BlockPos {
        BlockPos::new(self.min_block_x(), self.min_block_y(), self.min_block_z())
    }

    pub const fn center(self) -> BlockPos {
        BlockPos::new(
            self.min_block_x() + 8,
            self.min_block_y() + 8,
            self.min_block_z() + 8,
        )
    }

    pub const fn offset(self, x: i32, y: i32, z: i32) -> Self {
        Self::new(self.x + x, self.y + y, self.z + z)
    }

    pub const fn offset_direction(self, direction: Direction) -> Self {
        let (x, y, z) = direction.step_tuple();
        self.offset(x, y, z)
    }

    pub const fn offset_direction_n(self, direction: Direction, steps: i32) -> Self {
        let (x, y, z) = direction.step_tuple();
        self.offset(x * steps, y * steps, z * steps)
    }

    pub const fn to_vec3i(self) -> Vec3i {
        Vec3i::new(self.x, self.y, self.z)
    }
}

impl std::fmt::Display for SectionPos {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SectionPos{{x={}, y={}, z={}}}", self.x, self.y, self.z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_to_section_rounds_down() {
        assert_eq!(SectionPos::block_to_section_coord(0), 0);
        assert_eq!(SectionPos::block_to_section_coord(15), 0);
        assert_eq!(SectionPos::block_to_section_coord(16), 1);
        assert_eq!(SectionPos::block_to_section_coord(-1), -1);
        assert_eq!(SectionPos::block_to_section_coord(-16), -1);
        assert_eq!(SectionPos::block_to_section_coord(-17), -2);
    }

    #[test]
    fn packing_roundtrips() {
        for (x, y, z) in [(0, 0, 0), (5, -4, -9), (1000, 20, -1000)] {
            let pos = SectionPos::new(x, y, z);
            assert_eq!(SectionPos::of_long(pos.as_long()), pos);
        }
    }

    #[test]
    fn relative_position_packs_in_vanilla_order() {
        let pos = BlockPos::new(2, 3, 4);
        let relative = SectionPos::section_relative_pos(pos);
        assert_eq!(SectionPos::section_relative_x(relative), 2);
        assert_eq!(SectionPos::section_relative_y(relative), 3);
        assert_eq!(SectionPos::section_relative_z(relative), 4);
    }
}
