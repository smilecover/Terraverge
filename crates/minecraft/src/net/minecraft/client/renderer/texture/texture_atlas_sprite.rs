//! 对应 `net/minecraft/client/renderer/texture/TextureAtlasSprite.java`。

use crate::net::minecraft::resources::Identifier;

#[derive(Clone, Debug)]
pub struct TextureAtlasSprite {
    pub id: Identifier,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    u0: f32,
    u1: f32,
    v0: f32,
    v1: f32,
}

impl TextureAtlasSprite {
    pub fn new(
        id: Identifier,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        atlas_width: u32,
        atlas_height: u32,
    ) -> Self {
        Self {
            id,
            x,
            y,
            width,
            height,
            u0: x as f32 / atlas_width as f32,
            u1: (x + width) as f32 / atlas_width as f32,
            v0: y as f32 / atlas_height as f32,
            v1: (y + height) as f32 / atlas_height as f32,
        }
    }

    pub fn get_x(&self) -> u32 {
        self.x
    }

    pub fn get_y(&self) -> u32 {
        self.y
    }

    pub fn get_u0(&self) -> f32 {
        self.u0
    }

    pub fn get_u1(&self) -> f32 {
        self.u1
    }

    pub fn get_v0(&self) -> f32 {
        self.v0
    }

    pub fn get_v1(&self) -> f32 {
        self.v1
    }

    /// 对应 `getU(offset)`：把 0..1 的贴图内偏移映射到图集坐标。
    pub fn get_u(&self, offset: f32) -> f32 {
        self.u0 + (self.u1 - self.u0) * offset
    }

    pub fn get_v(&self, offset: f32) -> f32 {
        self.v0 + (self.v1 - self.v0) * offset
    }
}
