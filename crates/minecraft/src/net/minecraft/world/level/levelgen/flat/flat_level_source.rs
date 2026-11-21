//! 对应 `net/minecraft/world/level/levelgen/flat/FlatLevelSource.java` 的最小实现。
//!
//! 真正的 26.3 平地生成器读取 `FlatLevelGeneratorSettings` 的层配置、
//! 生物群系与结构；本里程碑先固定为“基岩 + 两层泥土 + 草方块”的
//! 演示世界，用于贯通资源→模型→网格→Vulkan 的完整链路。

use super::super::ChunkGenerator;
use crate::net::minecraft::core::BlockPos;
use crate::net::minecraft::world::level::block::Blocks;
use crate::net::minecraft::world::level::block::state::BlockState;
use crate::net::minecraft::world::level::chunk::LevelChunk;
use crate::net::minecraft::world::level::chunk_pos::ChunkPos;

pub struct FlatLevelSource {
    seed: i64,
}

impl FlatLevelSource {
    pub const BEDROCK_Y: i32 = -64;
    pub const GRASS_Y: i32 = -61;

    pub fn new(seed: i64) -> Self {
        Self { seed }
    }

    pub fn get_seed(&self) -> i64 {
        self.seed
    }

    pub fn get_layer_state(&self, y: i32) -> Option<BlockState> {
        match y {
            Self::BEDROCK_Y => Some(Blocks::bedrock().default_block_state()),
            -63 | -62 => Some(Blocks::dirt().default_block_state()),
            Self::GRASS_Y => Some(Blocks::grass_block().default_block_state()),
            _ => None,
        }
    }
}

impl ChunkGenerator for FlatLevelSource {
    fn generate_chunk(
        &self,
        pos: ChunkPos,
        min_section_y: i32,
        section_count: usize,
        default_block_state: BlockState,
    ) -> LevelChunk {
        let mut chunk = LevelChunk::new(pos, min_section_y, section_count, default_block_state);
        let min_x = pos.get_min_block_x();
        let min_z = pos.get_min_block_z();

        // 固定 4 层，直接写入对应区块段，避免对整列 4096 个空气格调用 set。
        for x in 0..16 {
            for z in 0..16 {
                for y in Self::BEDROCK_Y..=Self::GRASS_Y {
                    if let Some(state) = self.get_layer_state(y) {
                        chunk.set_block_state(BlockPos::new(min_x + x, y, min_z + z), state);
                    }
                }
            }
        }

        chunk
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_four_layer_superflat() {
        let source = FlatLevelSource::new(0);
        let air = Blocks::air().default_block_state();
        let chunk = source.generate_chunk(ChunkPos::ZERO, -4, 24, air);
        assert_eq!(chunk.get_non_empty_section_count(), 1);
        assert_eq!(
            chunk.get_block_state(BlockPos::new(0, FlatLevelSource::GRASS_Y, 0)),
            Blocks::grass_block().default_block_state()
        );
        assert_eq!(
            chunk.get_block_state(BlockPos::new(0, FlatLevelSource::BEDROCK_Y, 0)),
            Blocks::bedrock().default_block_state()
        );
        assert!(
            chunk
                .get_block_state(BlockPos::new(0, FlatLevelSource::GRASS_Y + 1, 0))
                .is_air()
        );
    }
}
