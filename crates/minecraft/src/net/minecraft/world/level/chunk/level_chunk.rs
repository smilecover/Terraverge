//! 对应 `net/minecraft/world/level/chunk/LevelChunk.java` 的基础存储部分。
//!
//! 26.3 的区块包含状态登记、高度图、实体、方块实体、光照等子系统；
//! 当前里程碑只落地方块状态存储与按坐标访问。

use super::super::block::state::BlockState;
use super::super::chunk_pos::ChunkPos;
use super::LevelChunkSection;
use crate::net::minecraft::core::BlockPos;

pub struct LevelChunk {
    pos: ChunkPos,
    min_section_y: i32,
    default_block_state: BlockState,
    sections: Vec<Option<Box<LevelChunkSection>>>,
    /// 非空段数量，供快速判断。
    non_empty_section_count: usize,
}

impl LevelChunk {
    pub fn new(
        pos: ChunkPos,
        min_section_y: i32,
        section_count: usize,
        default_block_state: BlockState,
    ) -> Self {
        let mut sections = Vec::with_capacity(section_count);
        sections.resize_with(section_count, || None);
        Self {
            pos,
            min_section_y,
            default_block_state,
            sections,
            non_empty_section_count: 0,
        }
    }

    pub fn get_pos(&self) -> ChunkPos {
        self.pos
    }

    pub fn get_min_section_y(&self) -> i32 {
        self.min_section_y
    }

    pub fn get_section_count(&self) -> usize {
        self.sections.len()
    }

    pub fn get_max_section_y(&self) -> i32 {
        self.min_section_y + self.sections.len() as i32 - 1
    }

    pub fn get_section(&self, section_y: i32) -> Option<&LevelChunkSection> {
        let index = section_y - self.min_section_y;
        if index < 0 {
            return None;
        }
        self.sections.get(index as usize)?.as_deref()
    }

    pub fn get_section_mut(&mut self, section_y: i32) -> Option<&mut LevelChunkSection> {
        let index = section_y - self.min_section_y;
        if index < 0 {
            return None;
        }
        self.sections.get_mut(index as usize)?.as_deref_mut()
    }

    pub fn get_section_or_create(&mut self, section_y: i32) -> Option<&mut LevelChunkSection> {
        let index = section_y - self.min_section_y;
        if index < 0 || index as usize >= self.sections.len() {
            return None;
        }
        let index = index as usize;
        if self.sections[index].is_none() {
            self.sections[index] = Some(Box::new(LevelChunkSection::new(self.default_block_state)));
        }
        self.sections[index].as_deref_mut()
    }

    /// `pos` 必须位于本区块内；越界时返回空气（对应 Java 未加载段）。
    pub fn get_block_state(&self, pos: BlockPos) -> BlockState {
        let section_y = pos.get_y() >> 4;
        let Some(section) = self.get_section(section_y) else {
            return self.default_block_state;
        };
        section.get_block_state(
            (pos.get_x() & 15) as u8,
            (pos.get_y() & 15) as u8,
            (pos.get_z() & 15) as u8,
        )
    }

    pub fn set_block_state(&mut self, pos: BlockPos, state: BlockState) -> BlockState {
        let section_y = pos.get_y() >> 4;
        let Some(section) = self.get_section_or_create(section_y) else {
            return state;
        };
        let old = section.set_block_state(
            (pos.get_x() & 15) as u8,
            (pos.get_y() & 15) as u8,
            (pos.get_z() & 15) as u8,
            state,
        );
        self.refresh_section_count(section_y);
        old
    }

    fn refresh_section_count(&mut self, _section_y: i32) {
        // 段数量很小（24），全量统计比维护增删计数更不容易出错。
        self.non_empty_section_count = self
            .sections
            .iter()
            .filter(|section| section.as_ref().is_some_and(|s| !s.has_only_air()))
            .count();
    }

    pub fn get_non_empty_section_count(&self) -> usize {
        self.non_empty_section_count
    }

    /// 遍历 (sectionY, 段) 的有效段；用于网格构建与调试。
    pub fn sections(&self) -> impl Iterator<Item = (i32, &LevelChunkSection)> {
        self.sections
            .iter()
            .enumerate()
            .filter_map(|(index, section)| {
                section
                    .as_deref()
                    .filter(|section| !section.has_only_air())
                    .map(|section| (self.min_section_y + index as i32, section))
            })
    }

    /// 遍历所有段（含空段），供需要完整世界高度的算法使用。
    pub fn all_sections(&self) -> impl Iterator<Item = (i32, Option<&LevelChunkSection>)> {
        self.sections
            .iter()
            .enumerate()
            .map(|(index, section)| (self.min_section_y + index as i32, section.as_deref()))
    }

    pub fn is_unsaved(&self) -> bool {
        true
    }
}
