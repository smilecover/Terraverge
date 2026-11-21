//! 对应 `net/minecraft/world/level/ChunkPos.java` 的核心部分。

use crate::net::minecraft::core::{BlockPos, SectionPos};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ChunkPos {
    pub x: i32,
    pub z: i32,
}

impl ChunkPos {
    pub const ZERO: ChunkPos = ChunkPos { x: 0, z: 0 };
    pub const REGION_SIZE: i32 = 32;
    pub const REGION_MASK: i32 = 31;
    pub const REGION_MAX_INDEX: i32 = 31;

    pub const fn new(x: i32, z: i32) -> Self {
        Self { x, z }
    }

    pub const fn containing(pos: BlockPos) -> Self {
        Self::new(
            SectionPos::block_to_section_coord(pos.get_x()),
            SectionPos::block_to_section_coord(pos.get_z()),
        )
    }

    pub const fn unpack(key: i64) -> Self {
        Self::new(key as i32, (key >> 32) as i32)
    }

    pub const fn pack(self) -> i64 {
        Self::pack_xyz(self.x, self.z)
    }

    pub const fn pack_xyz(x: i32, z: i32) -> i64 {
        (x as i64 & 0xFFFF_FFFF) | ((z as i64 & 0xFFFF_FFFF) << 32)
    }

    pub const fn get_min_block_x(self) -> i32 {
        self.x << 4
    }

    pub const fn get_min_block_z(self) -> i32 {
        self.z << 4
    }

    pub const fn get_max_block_x(self) -> i32 {
        (self.x << 4) + 15
    }

    pub const fn get_max_block_z(self) -> i32 {
        (self.z << 4) + 15
    }

    pub const fn get_middle_block_x(self) -> i32 {
        (self.x << 4) + 8
    }

    pub const fn get_middle_block_z(self) -> i32 {
        (self.z << 4) + 8
    }

    pub const fn get_block_at(self, x: i32, y: i32, z: i32) -> BlockPos {
        BlockPos::new((self.x << 4) + x, y, (self.z << 4) + z)
    }

    pub const fn get_world_position(self) -> BlockPos {
        self.get_block_at(0, 0, 0)
    }

    pub const fn distance_squared(self, other: ChunkPos) -> i32 {
        let dx = self.x - other.x;
        let dz = self.z - other.z;
        dx * dx + dz * dz
    }

    pub const fn get_region_x(self) -> i32 {
        self.x >> 5
    }

    pub const fn get_region_z(self) -> i32 {
        self.z >> 5
    }

    pub const fn get_region_local_x(self) -> i32 {
        self.x & Self::REGION_MASK
    }

    pub const fn get_region_local_z(self) -> i32 {
        self.z & Self::REGION_MASK
    }

    pub const fn to_chunk_pos(self) -> ChunkPos {
        self
    }
}

impl std::fmt::Display for ChunkPos {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}, {}]", self.x, self.z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_roundtrip() {
        let pos = ChunkPos::new(-12, 345);
        assert_eq!(ChunkPos::unpack(pos.pack()), pos);
    }

    #[test]
    fn containing_uses_floor() {
        assert_eq!(
            ChunkPos::containing(BlockPos::new(-1, 0, 16)),
            ChunkPos::new(-1, 1)
        );
        assert_eq!(
            ChunkPos::containing(BlockPos::new(15, 0, 0)),
            ChunkPos::new(0, 0)
        );
    }
}
