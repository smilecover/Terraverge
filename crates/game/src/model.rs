//! 加载 Wavefront OBJ 模型。
//!
//! 对应教程「Loading models」章节。教程使用 tinyobjloader；这里实现一个
//! 满足当前需求的最小加载器：
//! - 支持 `v`、`vt`、`f`（多边形会按扇形三角化），忽略法线/材质/分组；
//! - 按 `(位置索引, UV 索引)` 去重顶点，与教程的 `unordered_map` 做法一致；
//! - OBJ 的 V 轴向上，这里翻转成 Vulkan 纹理坐标的向下方向；
//! - 索引统一使用 32 位，避免大模型的 u16 溢出。
//!
//! `assets/models/cube.obj` 是教程 viking_room.obj 的替代资源。

use render::{MeshData, Vertex};
use std::collections::HashMap;
use std::error::Error;
use std::path::Path;

/// 加载后的模型：去重后的顶点和三角形索引。
pub struct ModelData {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
}

impl ModelData {
    /// 借出渲染后端能直接上传的网格视图。
    pub fn as_mesh(&self) -> MeshData<'_> {
        MeshData {
            vertices: &self.vertices,
            indices: &self.indices,
        }
    }
}

/// 读取并解析 `path` 指向的 OBJ 文件。
pub fn load(path: &Path) -> Result<ModelData, Box<dyn Error + Send + Sync>> {
    let source = std::fs::read_to_string(path)?;

    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut tex_coords: Vec<[f32; 2]> = Vec::new();
    let mut vertices: Vec<Vertex> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    // 键是 (位置索引, UV 索引)：同一位置在接缝处会对应多个 UV，不能误合并。
    let mut unique_vertices: HashMap<(u32, u32), u32> = HashMap::new();

    for (line_number, raw_line) in source.lines().enumerate() {
        let line_number = line_number + 1;
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let mut parts = line.split_whitespace();
        match parts.next() {
            Some("v") => {
                let position = [
                    parse_f32(parts.next(), line_number)?,
                    parse_f32(parts.next(), line_number)?,
                    parse_f32(parts.next(), line_number)?,
                ];
                positions.push(position);
            }
            Some("vt") => {
                let uv = [
                    parse_f32(parts.next(), line_number)?,
                    parse_f32(parts.next(), line_number)?,
                ];
                tex_coords.push(uv);
            }
            Some("f") => {
                // 每个 token 形如 v、v/vt 或 v/vt/vn；这里要求带 UV。
                let mut face = Vec::new();
                for token in parts {
                    let mut components = token.split('/');
                    let position_index =
                        parse_index(components.next(), positions.len(), line_number)?;
                    let tex_coord_index = match components.next() {
                        Some(text) if !text.is_empty() => {
                            parse_index(Some(text), tex_coords.len(), line_number)?
                        }
                        _ => {
                            return Err(format!("第 {line_number} 行的面缺少纹理坐标索引").into());
                        }
                    };
                    face.push((position_index, tex_coord_index));
                }

                if face.len() < 3 {
                    return Err(format!("第 {line_number} 行的面少于 3 个顶点").into());
                }

                // 多边形按扇形三角化：(0, i, i+1)。
                for i in 1..face.len() - 1 {
                    for &key in &[face[0], face[i], face[i + 1]] {
                        let index = match unique_vertices.get(&key) {
                            Some(&index) => index,
                            None => {
                                let position = positions[key.0 as usize];
                                let uv = tex_coords[key.1 as usize];
                                // 模型颜色统一为白色，最终颜色来自纹理。
                                let vertex = Vertex::new(
                                    position,
                                    [1.0, 1.0, 1.0],
                                    // 翻转 V，让 OBJ/图片的“上”对上 Vulkan 纹理坐标。
                                    [uv[0], 1.0 - uv[1]],
                                );
                                let index = vertices.len() as u32;
                                vertices.push(vertex);
                                unique_vertices.insert(key, index);
                                index
                            }
                        };
                        indices.push(index);
                    }
                }
            }
            // 法线、材质、分组等对当前渲染没有影响，直接忽略。
            Some(_) | None => {}
        }
    }

    if vertices.is_empty() || indices.is_empty() {
        return Err("模型没有可用的顶点/三角形".into());
    }

    Ok(ModelData { vertices, indices })
}

fn parse_f32(token: Option<&str>, line_number: usize) -> Result<f32, Box<dyn Error + Send + Sync>> {
    let token = token.ok_or_else(|| format!("第 {line_number} 行缺少数字"))?;
    token
        .parse::<f32>()
        .map_err(|_| format!("第 {line_number} 行的 `{token}` 不是合法数字").into())
}

/// 解析 OBJ 索引：从 1 开始，负数表示相对当前列表末尾。
fn parse_index(
    token: Option<&str>,
    count: usize,
    line_number: usize,
) -> Result<u32, Box<dyn Error + Send + Sync>> {
    let token = token.ok_or_else(|| format!("第 {line_number} 行缺少索引"))?;
    let raw = token
        .parse::<i64>()
        .map_err(|_| format!("第 {line_number} 行的索引 `{token}` 不合法"))?;

    let index = if raw > 0 {
        raw - 1
    } else if raw < 0 {
        count as i64 + raw
    } else {
        return Err(format!("第 {line_number} 行的索引不能为 0").into());
    };

    if index < 0 || index as usize >= count {
        return Err(format!("第 {line_number} 行的索引 {raw} 超出范围（共 {count} 个）").into());
    }

    Ok(index as u32)
}
