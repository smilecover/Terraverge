//! 对应 `net/minecraft/world/level/chunk/PalettedContainer.java` 的核心。
//!
//! 26.3 的容器有 single-value / linear / global 三种策略与线程安全包装。
//! 这里先实现语义一致的两态版本：
//! - `data == None`：整个 16³ 段都是 `palette[0]`（对应 single-value）；
//! - `data == Some`：4096 个 u16 调色板索引（对应 linear/global）。
//!
//! 索引顺序与 Java 相同：`y << 8 | z << 4 | x`。

use crate::net::minecraft::world::level::block::state::BlockState;

#[derive(Clone)]
pub struct PalettedContainer {
    palette: Vec<BlockState>,
    data: Option<Box<[u16; 4096]>>,
}

impl PalettedContainer {
    pub fn new(default_state: BlockState) -> Self {
        Self {
            palette: vec![default_state],
            data: None,
        }
    }

    pub fn get(&self, x: u8, y: u8, z: u8) -> BlockState {
        let index = Self::index(x, y, z);
        match &self.data {
            None => self.palette[0],
            Some(data) => self.palette[data[index] as usize],
        }
    }

    pub fn set(&mut self, x: u8, y: u8, z: u8, state: BlockState) {
        let index = Self::index(x, y, z);
        let palette_index = match self
            .palette
            .iter()
            .position(|&candidate| candidate == state)
        {
            Some(index) => index,
            None => {
                assert!(
                    self.palette.len() < u16::MAX as usize,
                    "调色板超过 u16 索引上限"
                );
                self.palette.push(state);
                self.palette.len() - 1
            }
        };

        match &mut self.data {
            None => {
                if palette_index == 0 {
                    return;
                }
                let mut data = Box::new([0u16; 4096]);
                data[index] = palette_index as u16;
                self.data = Some(data);
            }
            Some(data) => {
                data[index] = palette_index as u16;
            }
        }
    }

    pub const fn index(x: u8, y: u8, z: u8) -> usize {
        ((y as usize) << 8) | ((z as usize) << 4) | x as usize
    }

    pub fn maybe_has(&self, state: BlockState) -> bool {
        self.palette.contains(&state)
    }

    pub fn count(&self, state: BlockState) -> usize {
        match &self.data {
            None => {
                if self.palette[0] == state {
                    4096
                } else {
                    0
                }
            }
            Some(data) => data
                .iter()
                .filter(|&&index| self.palette[index as usize] == state)
                .count(),
        }
    }

    /// 是否是单值容器（对应 Java `isSingleValue`）。
    pub fn is_single_value(&self) -> bool {
        self.data.is_none()
    }

    pub fn get_default_state(&self) -> BlockState {
        self.palette[0]
    }

    pub fn palette_len(&self) -> usize {
        self.palette.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::minecraft::world::level::block::Blocks;

    #[test]
    fn single_value_container_matches_java_defaults() {
        let air = Blocks::air().default_block_state();
        let stone = Blocks::stone().default_block_state();
        let mut container = PalettedContainer::new(air);
        assert!(container.is_single_value());
        assert_eq!(container.get(0, 0, 0), air);
        container.set(1, 2, 3, stone);
        assert!(!container.is_single_value());
        assert_eq!(container.get(1, 2, 3), stone);
        assert_eq!(container.get(3, 2, 1), air);
        assert_eq!(container.count(stone), 1);
        assert_eq!(container.count(air), 4095);
    }

    #[test]
    fn index_order_matches_vanilla() {
        assert_eq!(PalettedContainer::index(1, 2, 3), (2 << 8) | (3 << 4) | 1);
    }
}
