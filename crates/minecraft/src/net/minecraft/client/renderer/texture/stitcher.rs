//! 对应 `net/minecraft/client/renderer/texture/Stitcher.java`。
//!
//! 26.3 的实现是带 mip 级别与 padding 的二维装箱算法。当前方块贴图
//! 数量很少且均为 16×16，先用固定单元格的行列排布，行为可预测、便于调试。
//! 后续替换为与 Java 相同的装箱算法时，只需改本文件。

use super::sprite_contents::SpriteContents;
use super::texture_atlas::TextureAtlasLayout;
use crate::net::minecraft::resources::Identifier;
use std::collections::HashMap;

pub struct Stitcher;

impl Stitcher {
    /// 返回图集布局与贴图内容；像素拼接由 `TextureAtlas` 完成。
    pub fn stitch(
        mut sprites: Vec<SpriteContents>,
    ) -> (TextureAtlasLayout, HashMap<Identifier, SpriteContents>) {
        sprites.sort_by(|a, b| a.id.cmp(&b.id));
        let count = sprites.len().max(1);
        let cell = sprites
            .iter()
            .map(|sprite| sprite.width.max(sprite.height))
            .max()
            .unwrap_or(16);

        let columns = (count as f64).sqrt().ceil() as u32;
        let columns = columns.max(1);
        let rows = (count as u32).div_ceil(columns);
        let atlas_width = columns * cell;
        let atlas_height = rows * cell;

        let mut placements = Vec::with_capacity(sprites.len());
        let mut contents = HashMap::with_capacity(sprites.len());
        for (index, sprite) in sprites.into_iter().enumerate() {
            let column = index as u32 % columns;
            let row = index as u32 / columns;
            let x = column * cell;
            let y = row * cell;
            placements.push((sprite.id.clone(), x, y, sprite.width, sprite.height));
            contents.insert(sprite.id.clone(), sprite);
        }

        let layout = TextureAtlasLayout::new(atlas_width, atlas_height, placements);
        (layout, contents)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sprite(id: &str, width: u32, height: u32) -> SpriteContents {
        SpriteContents {
            id: Identifier::parse(id).unwrap(),
            width,
            height,
            pixels: vec![255; (width * height * 4) as usize],
        }
    }

    #[test]
    fn stitches_into_square_cells() {
        let (layout, contents) = Stitcher::stitch(vec![
            sprite("minecraft:block/stone", 16, 16),
            sprite("minecraft:block/dirt", 16, 16),
            sprite("minecraft:block/grass_block_side", 16, 16),
        ]);
        assert_eq!(contents.len(), 3);
        assert_eq!(layout.width, 32);
        assert_eq!(layout.height, 32);
        assert_eq!(layout.placements.len(), 3);
    }
}
