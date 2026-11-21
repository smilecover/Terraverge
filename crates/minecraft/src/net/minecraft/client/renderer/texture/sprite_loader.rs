//! 对应 `net/minecraft/client/renderer/texture/SpriteLoader.java`。
//!
//! 负责按标识符读取 PNG、处理缺失贴图并合成图集。

use super::sprite_contents::SpriteContents;
use super::stitcher::Stitcher;
use super::texture_atlas::TextureAtlas;
use crate::net::minecraft::resources::Identifier;
use crate::net::minecraft::server::packs::resources::ResourceManager;
use std::collections::HashSet;

pub struct SpriteLoader;

impl SpriteLoader {
    /// 加载 `texture_ids` 指向的全部贴图并合成图集。缺失的贴图会用
    /// 紫黑棋盘格替代，不会让整个世界加载失败。
    pub fn load_atlas(
        resource_manager: &ResourceManager,
        texture_ids: &[Identifier],
    ) -> TextureAtlas {
        let mut unique = HashSet::new();
        let mut contents = Vec::new();
        for id in texture_ids {
            if !unique.insert(id.clone()) {
                continue;
            }
            match SpriteContents::load(resource_manager, id) {
                Ok(content) => contents.push(content),
                Err(err) => {
                    eprintln!("贴图加载失败（使用缺失贴图占位）: {err}");
                    contents.push(SpriteContents::missing(id));
                }
            }
        }

        let (layout, contents) = Stitcher::stitch(contents);
        TextureAtlas::compose(layout, contents)
    }
}
