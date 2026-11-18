//! 纹理资源加载：把图片解码成 RGBA8，交给渲染后端上传到 GPU。
//!
//! 对应教程「Images」章节的纹理加载部分。资源和文件路径属于游戏层，
//! 渲染后端只接收原始像素，不关心图片格式。

use render::TextureData;
use std::error::Error;
use std::path::Path;

/// 解码后的纹理数据，`pixels` 是紧凑的 RGBA8（每像素 4 字节）。
pub struct Texture {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl Texture {
    /// 借出渲染后端上传所需的像素视图。
    pub fn as_render_data(&self) -> TextureData<'_> {
        TextureData {
            width: self.width,
            height: self.height,
            pixels: &self.pixels,
        }
    }
}

/// 从文件加载纹理并统一转换成 RGBA8。
/// 教程使用 stb_image，这里用 `image` crate 做等价的事情。
pub fn load(path: &Path) -> Result<Texture, Box<dyn Error + Send + Sync>> {
    let image = image::open(path)?.to_rgba8();
    let (width, height) = image.dimensions();

    if width == 0 || height == 0 {
        return Err("纹理尺寸不能为 0".into());
    }

    Ok(Texture {
        width,
        height,
        pixels: image.into_raw(),
    })
}
