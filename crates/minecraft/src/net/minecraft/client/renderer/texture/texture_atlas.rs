//! 对应 `net/minecraft/client/renderer/texture/TextureAtlas.java` 的 CPU 侧。
//!
//! GPU 上传由 `ren_vulkan` 负责；本结构只保存合成后的 RGBA8 图集与
//! 每个 sprite 的 UV 范围。

use super::sprite_contents::SpriteContents;
use super::texture_atlas_sprite::TextureAtlasSprite;
use crate::net::minecraft::resources::Identifier;
use std::collections::HashMap;

/// `Stitcher` 产出的布局（对应 26.3 的 `TextureAtlasLayout`）。
pub struct TextureAtlasLayout {
    pub width: u32,
    pub height: u32,
    /// `(id, x, y, width, height)`。
    pub placements: Vec<(Identifier, u32, u32, u32, u32)>,
}

impl TextureAtlasLayout {
    pub fn new(width: u32, height: u32, placements: Vec<(Identifier, u32, u32, u32, u32)>) -> Self {
        Self {
            width,
            height,
            placements,
        }
    }
}

pub struct TextureAtlas {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
    sprites: HashMap<Identifier, TextureAtlasSprite>,
}

impl TextureAtlas {
    /// 按布局把多个 sprite 合成到一张 RGBA8 图集。
    pub fn compose(
        layout: TextureAtlasLayout,
        contents: HashMap<Identifier, SpriteContents>,
    ) -> Self {
        let mut pixels = vec![0u8; (layout.width * layout.height * 4) as usize];
        let mut sprites = HashMap::with_capacity(layout.placements.len());

        for (id, x, y, width, height) in &layout.placements {
            let Some(content) = contents.get(id) else {
                continue;
            };
            debug_assert_eq!(content.width, *width);
            debug_assert_eq!(content.height, *height);

            for row in 0..*height {
                let dest_start = (((y + row) * layout.width + x) * 4) as usize;
                let source_start = (row * content.width * 4) as usize;
                let length = (content.width * 4) as usize;
                pixels[dest_start..dest_start + length]
                    .copy_from_slice(&content.pixels[source_start..source_start + length]);
            }

            sprites.insert(
                id.clone(),
                TextureAtlasSprite::new(
                    id.clone(),
                    *x,
                    *y,
                    *width,
                    *height,
                    layout.width,
                    layout.height,
                ),
            );
        }

        Self {
            width: layout.width,
            height: layout.height,
            pixels,
            sprites,
        }
    }

    pub fn get_sprite(&self, id: &Identifier) -> Option<&TextureAtlasSprite> {
        self.sprites.get(id)
    }

    pub fn get_width(&self) -> u32 {
        self.width
    }

    pub fn get_height(&self) -> u32 {
        self.height
    }

    pub fn get_pixels(&self) -> &[u8] {
        &self.pixels
    }

    pub fn sprite_count(&self) -> usize {
        self.sprites.len()
    }

    pub fn sprites(&self) -> impl Iterator<Item = &TextureAtlasSprite> {
        self.sprites.values()
    }
}
