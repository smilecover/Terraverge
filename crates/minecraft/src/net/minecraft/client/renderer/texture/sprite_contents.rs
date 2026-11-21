//! 对应 `net/minecraft/client/renderer/texture/SpriteContents.java`。
//!
//! 只保留静态贴图内容；动画帧（`.mcmeta` 的 animation 段）与 mipmap
//! 生成策略尚未移植，见 `docs/PROGRESS.md`。

use crate::net::minecraft::resources::Identifier;
use crate::net::minecraft::server::packs::resources::ResourceManager;
use std::error::Error;

#[derive(Clone)]
pub struct SpriteContents {
    pub id: Identifier,
    pub width: u32,
    pub height: u32,
    /// 紧凑 RGBA8。
    pub pixels: Vec<u8>,
}

impl SpriteContents {
    pub fn load(
        resource_manager: &ResourceManager,
        id: &Identifier,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let resource = resource_manager.get_texture_png(id)?;
        let image = image::load_from_memory(resource.open_as_reader())
            .map_err(|err| format!("解码贴图 {id} 失败: {err}"))?
            .to_rgba8();
        let (width, height) = image.dimensions();
        if width == 0 || height == 0 {
            return Err(format!("贴图 {id} 尺寸为 0").into());
        }
        Ok(Self {
            id: id.clone(),
            width,
            height,
            pixels: image.into_raw(),
        })
    }

    /// 缺失贴图使用 Minecraft 的紫黑棋盘格，便于定位资源问题。
    pub fn missing(id: &Identifier) -> Self {
        const SIZE: u32 = 16;
        let mut pixels = vec![0u8; (SIZE * SIZE * 4) as usize];
        for y in 0..SIZE {
            for x in 0..SIZE {
                let magenta = ((x / 8) + (y / 8)) % 2 == 0;
                let color = if magenta {
                    [255u8, 0, 255, 255]
                } else {
                    [0u8, 0, 0, 255]
                };
                let index = ((y * SIZE + x) * 4) as usize;
                pixels[index..index + 4].copy_from_slice(&color);
            }
        }
        Self {
            id: id.clone(),
            width: SIZE,
            height: SIZE,
            pixels,
        }
    }
}
