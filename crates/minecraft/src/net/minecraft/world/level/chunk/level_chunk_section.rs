//! 对应 `net/minecraft/world/level/chunk/LevelChunkSection.java`。

use super::paletted_container::PalettedContainer;
use crate::net::minecraft::world::level::block::state::BlockState;

#[derive(Clone)]
pub struct LevelChunkSection {
    states: PalettedContainer,
    non_empty_block_count: u16,
}

impl LevelChunkSection {
    pub fn new(default_state: BlockState) -> Self {
        Self {
            states: PalettedContainer::new(default_state),
            non_empty_block_count: 0,
        }
    }

    pub fn get_block_state(&self, x: u8, y: u8, z: u8) -> BlockState {
        self.states.get(x & 15, y & 15, z & 15)
    }

    pub fn set_block_state(&mut self, x: u8, y: u8, z: u8, state: BlockState) -> BlockState {
        let old_state = self.get_block_state(x, y, z);
        if old_state == state {
            return old_state;
        }
        let old_empty = old_state.is_air();
        let new_empty = state.is_air();
        if old_empty && !new_empty {
            self.non_empty_block_count += 1;
        } else if !old_empty && new_empty {
            self.non_empty_block_count -= 1;
        }
        self.states.set(x & 15, y & 15, z & 15, state);
        old_state
    }

    pub fn has_only_air(&self) -> bool {
        self.non_empty_block_count == 0
    }

    pub fn maybe_has(&self, state: BlockState) -> bool {
        self.states.maybe_has(state)
    }

    pub fn get_states(&self) -> &PalettedContainer {
        &self.states
    }

    pub fn non_empty_block_count(&self) -> u16 {
        self.non_empty_block_count
    }
}
