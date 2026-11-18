//! 顶点数据、顶点输入描述与网格数据。
//!
//! 对应教程「Vertex input description」「Loading models」章节：顶点从二维位置
//! 扩展为三维位置，仍然保留颜色和纹理坐标。模型由游戏层加载后通过
//! `MeshData` 交给渲染后端。

use ash::vk;
use std::mem::{offset_of, size_of};

/// 一个顶点：三维位置 + RGB 颜色 + 纹理坐标。
///
/// `#[repr(C)]` 保证字段按声明顺序紧凑排列，Rust 侧的偏移量才能与
/// Vulkan 顶点属性描述一致；当前布局大小为 3*4 + 3*4 + 2*4 = 32 字节。
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
    tex_coord: [f32; 2],
}

impl Vertex {
    /// 游戏层加载 OBJ 时用它构造顶点。
    /// 教程里的模型顶点颜色统一为白色，最终颜色来自纹理采样。
    pub fn new(position: [f32; 3], color: [f32; 3], tex_coord: [f32; 2]) -> Self {
        Self {
            position,
            color,
            tex_coord,
        }
    }

    /// 绑定描述：所有属性都来自 binding 0，按“每顶点前进一个 `Vertex`”读取。
    pub(super) fn binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::default()
            .binding(0)
            .stride(size_of::<Self>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
    }

    /// 属性描述：location 0 是 vec3 位置，location 1 是 vec3 颜色，
    /// location 2 是 vec2 纹理坐标。
    pub(super) fn attribute_descriptions() -> [vk::VertexInputAttributeDescription; 3] {
        [
            vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(0)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(offset_of!(Self, position) as u32),
            vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(1)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(offset_of!(Self, color) as u32),
            vk::VertexInputAttributeDescription::default()
                .binding(0)
                .location(2)
                .format(vk::Format::R32G32_SFLOAT)
                .offset(offset_of!(Self, tex_coord) as u32),
        ]
    }
}

/// 一段可上传的网格数据。游戏层加载模型后把顶点/索引切片交给渲染后端，
/// 渲染后端不关心模型文件格式。索引统一使用 32 位，避免大模型溢出。
pub struct MeshData<'a> {
    pub vertices: &'a [Vertex],
    pub indices: &'a [u32],
}
