//! 对应 `net/minecraft/world/level/Level.java` 的世界存储核心。
//!
//! 26.3 的 `Level` 是抽象类，包含实体、方块实体、刻调度、光照、
//! 方块更新等系统。本移植先提供方块状态读写与高度查询，后续按子系统扩展。

use super::block::Blocks;
use super::block::state::BlockState;
use super::chunk::LevelChunk;
use super::chunk_pos::ChunkPos;
use crate::net::minecraft::core::BlockPos;
use std::collections::HashMap;

pub struct Level {
    chunks: HashMap<ChunkPos, LevelChunk>,
    min_y: i32,
    height: i32,
    default_block_state: BlockState,
}

impl Level {
    pub fn new(min_y: i32, height: i32) -> Self {
        assert!(height > 0 && height % 16 == 0, "世界高度必须是 16 的倍数");
        Self {
            chunks: HashMap::new(),
            min_y,
            height,
            default_block_state: Blocks::air().default_block_state(),
        }
    }

    pub fn get_min_y(&self) -> i32 {
        self.min_y
    }

    pub fn get_height(&self) -> i32 {
        self.height
    }

    pub fn get_max_y(&self) -> i32 {
        self.min_y + self.height
    }

    pub fn get_sections_count(&self) -> usize {
        (self.height / 16) as usize
    }

    pub fn get_min_section_y(&self) -> i32 {
        self.min_y >> 4
    }

    pub fn get_max_section_y(&self) -> i32 {
        self.get_min_section_y() + self.get_sections_count() as i32 - 1
    }

    pub fn is_inside_build_height(&self, y: i32) -> bool {
        y >= self.min_y && y < self.get_max_y()
    }

    pub fn get_default_block_state(&self) -> BlockState {
        self.default_block_state
    }

    pub fn get_chunk(&self, pos: ChunkPos) -> Option<&LevelChunk> {
        self.chunks.get(&pos)
    }

    pub fn get_chunk_mut(&mut self, pos: ChunkPos) -> Option<&mut LevelChunk> {
        self.chunks.get_mut(&pos)
    }

    pub fn has_chunk(&self, pos: ChunkPos) -> bool {
        self.chunks.contains_key(&pos)
    }

    pub fn insert_chunk(&mut self, chunk: LevelChunk) -> Option<LevelChunk> {
        self.chunks.insert(chunk.get_pos(), chunk)
    }

    pub fn iter_chunks(&self) -> impl Iterator<Item = &LevelChunk> {
        self.chunks.values()
    }

    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    /// 对应 `Level.getBlockState`。未加载的区块返回空气。
    pub fn get_block_state(&self, pos: BlockPos) -> BlockState {
        if !self.is_inside_build_height(pos.get_y()) {
            return self.default_block_state;
        }
        let chunk_pos = ChunkPos::containing(pos);
        self.get_chunk(chunk_pos)
            .map_or(self.default_block_state, |chunk| chunk.get_block_state(pos))
    }

    /// 对应 `setBlock(pos, state, flags)` 的基本存储部分；更新传播待移植。
    pub fn set_block_state(&mut self, pos: BlockPos, state: BlockState) -> BlockState {
        if !self.is_inside_build_height(pos.get_y()) {
            return self.default_block_state;
        }
        let chunk_pos = ChunkPos::containing(pos);
        let default_block_state = self.default_block_state;
        self.get_chunk_mut(chunk_pos)
            .map_or(default_block_state, |chunk| {
                chunk.set_block_state(pos, state)
            })
    }

    pub fn is_air(&self, pos: BlockPos) -> bool {
        self.get_block_state(pos).is_air()
    }

    /// 对应 `getHeight(Heightmap.Types.MOTION_BLOCKING, x, z)`。
    /// 返回最高阻挡方块之上的 y 坐标（Java 返回 `height + 1`）。
    pub fn get_height_motion_blocking(&self, x: i32, z: i32) -> i32 {
        let max_y = self.get_max_y() - 1;
        for y in (self.min_y..=max_y).rev() {
            let state = self.get_block_state(BlockPos::new(x, y, z));
            if state.has_collision() {
                return y + 1;
            }
        }
        self.min_y
    }

    pub fn get_height_motion_blocking_pos(&self, pos: BlockPos) -> i32 {
        self.get_height_motion_blocking(pos.get_x(), pos.get_z())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::minecraft::world::level::block::Blocks;

    #[test]
    fn unloaded_blocks_are_air() {
        let level = Level::new(-64, 384);
        assert!(level.get_block_state(BlockPos::new(0, 0, 0)).is_air());
        assert_eq!(level.get_height_motion_blocking(0, 0), -64);
    }

    #[test]
    fn stores_and_reads_block_states() {
        let mut level = Level::new(-64, 384);
        let pos = ChunkPos::ZERO;
        let chunk = LevelChunk::new(
            pos,
            level.get_min_section_y(),
            level.get_sections_count(),
            level.get_default_block_state(),
        );
        level.insert_chunk(chunk);
        let stone = Blocks::stone().default_block_state();
        level.set_block_state(BlockPos::new(3, -60, 5), stone);
        assert_eq!(level.get_block_state(BlockPos::new(3, -60, 5)), stone);
        assert_eq!(level.get_height_motion_blocking(3, 5), -59);
    }
}
