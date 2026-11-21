//! `net.minecraft.world.level.levelgen` 包。

pub mod flat;

use super::block::state::BlockState;
use super::chunk::LevelChunk;
use super::chunk_pos::ChunkPos;

/// 对应 `ChunkGenerator`。当前只保留生成单个区块的同步接口。
pub trait ChunkGenerator {
    fn generate_chunk(
        &self,
        pos: ChunkPos,
        min_section_y: i32,
        section_count: usize,
        default_block_state: BlockState,
    ) -> LevelChunk;
}
