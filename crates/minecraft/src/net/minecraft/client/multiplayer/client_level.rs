//! 对应 `net/minecraft/client/multiplayer/ClientLevel.java` 的世界入口。
//!
//! 完整的 `ClientLevel` 负责实体、方块实体、刻调度、光照与区块上传；
//! 本里程碑以 `Level` 为存储、`FlatLevelSource` 为生成器，先打通渲染。

use crate::net::minecraft::core::BlockPos;
use crate::net::minecraft::world::level::Level;
use crate::net::minecraft::world::level::chunk_pos::ChunkPos;
use crate::net::minecraft::world::level::levelgen::ChunkGenerator;
use crate::net::minecraft::world::level::levelgen::flat::FlatLevelSource;
use crate::net::minecraft::world::phys::Vec3;
use std::ops::{Deref, DerefMut};

pub const OVERWORLD_MIN_Y: i32 = -64;
pub const OVERWORLD_HEIGHT: i32 = 384;

pub struct ClientLevel {
    level: Level,
}

impl ClientLevel {
    /// 生成以 `(0,0)` 为中心、半径 `radius` 个区块的平坦演示世界。
    pub fn new_flat(radius: i32, seed: i64) -> Self {
        let mut level = Level::new(OVERWORLD_MIN_Y, OVERWORLD_HEIGHT);
        let generator = FlatLevelSource::new(seed);
        for chunk_x in -radius..=radius {
            for chunk_z in -radius..=radius {
                let pos = ChunkPos::new(chunk_x, chunk_z);
                let chunk = generator.generate_chunk(
                    pos,
                    level.get_min_section_y(),
                    level.get_sections_count(),
                    level.get_default_block_state(),
                );
                level.insert_chunk(chunk);
            }
        }
        Self { level }
    }

    pub fn get_level(&self) -> &Level {
        &self.level
    }

    pub fn get_level_mut(&mut self) -> &mut Level {
        &mut self.level
    }

    /// 玩家出生点：区块 (0,0) 中央的草方块顶面之上。
    pub fn get_spawn_position(&self) -> Vec3 {
        let x = 8;
        let z = 8;
        let top = self.level.get_height_motion_blocking(x, z);
        Vec3::new(x as f64 + 0.5, top as f64, z as f64 + 0.5)
    }

    pub fn get_shared_spawn(&self) -> BlockPos {
        let spawn = self.get_spawn_position();
        BlockPos::containing(spawn.x, spawn.y, spawn.z)
    }

    pub fn get_chunk_count(&self) -> usize {
        self.level.chunk_count()
    }
}

impl Deref for ClientLevel {
    type Target = Level;

    fn deref(&self) -> &Self::Target {
        &self.level
    }
}

impl DerefMut for ClientLevel {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.level
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_world_has_expected_ground_height() {
        let level = ClientLevel::new_flat(1, 0);
        assert_eq!(level.get_chunk_count(), 9);
        let spawn = level.get_spawn_position();
        assert_eq!(spawn.y, -60.0);
        assert!(
            level
                .get_block_state(BlockPos::new(8, -61, 8))
                .is_air()
                .eq(&false)
        );
    }
}
