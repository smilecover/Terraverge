//! 对应 `net/minecraft/client/color/block/BlockColors.java` 的最小实现。
//!
//! 26.3 的着色由 `BlockTintSource` 按 `tintindex` 与生物群系计算；当前
//! 没有生物群系系统，因此使用原版默认草色/树叶色。查找键是方块标识符。

use crate::net::minecraft::world::level::block::state::BlockState;
use std::collections::HashMap;

pub const GRASS_COLOR: [f32; 3] = [0.568_6, 0.741_2, 0.349_0];
pub const FOLIAGE_COLOR: [f32; 3] = [0.282_4, 0.709_8, 0.094_1];

pub struct BlockColors {
    colors: HashMap<&'static str, [f32; 3]>,
}

impl BlockColors {
    /// 原版默认颜色（温带生物群系）。
    pub fn vanilla() -> Self {
        let mut colors = HashMap::new();
        colors.insert("grass_block", GRASS_COLOR);
        colors.insert("oak_leaves", FOLIAGE_COLOR);
        Self { colors }
    }

    /// 对应 `BlockColors.getColor(state, level, pos, tintIndex)`。
    /// `tint_index < 0` 表示该面不着色。
    pub fn get_color(&self, state: BlockState, tint_index: i32) -> Option<[f32; 3]> {
        if tint_index < 0 {
            return None;
        }
        self.colors.get(state.get_block_name().get_path()).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::minecraft::world::level::block::Blocks;

    #[test]
    fn grass_and_leaves_have_default_tints() {
        let colors = BlockColors::vanilla();
        let grass = Blocks::grass_block().default_block_state();
        let leaves = Blocks::oak_leaves().default_block_state();
        assert_eq!(colors.get_color(grass, 0), Some(GRASS_COLOR));
        assert_eq!(colors.get_color(leaves, 0), Some(FOLIAGE_COLOR));
        assert_eq!(colors.get_color(grass, -1), None);
    }
}
