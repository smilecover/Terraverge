//! 对应 `net/minecraft/client/renderer/chunk/SectionCompiler.java` 的
//! CPU 网格构建部分。
//!
//! 26.3 会把每个段编译成分层（solid/cutout/translucent）的 GPU 网格；
//! 当前世界没有透明方块，因此先合成单一网格：
//! - 只为 `RenderShape::MODEL` 的方块输出面；
//! - 有 `cullface` 的面在邻居为不透明实心方块时跳过；
//! - 顶点色 = 生物群系着色（暂时固定为草色）× 面向明暗。

use super::render_mesh::{RenderMesh, RenderVertex};
use crate::net::minecraft::client::color::block::BlockColors;
use crate::net::minecraft::client::renderer::texture::TextureAtlas;
use crate::net::minecraft::client::resources::model::{BakedPart, BlockModelSet};
use crate::net::minecraft::core::{BlockPos, Direction};
use crate::net::minecraft::util::mth::Mth;
use crate::net::minecraft::util::random::RandomSource;
use crate::net::minecraft::world::level::Level;
use crate::net::minecraft::world::level::block::RenderShape;
use crate::net::minecraft::world::level::levelgen::LegacyRandomSource;

pub struct SectionCompiler;

impl SectionCompiler {
    /// 编译世界中所有已加载区块的可见方块面。
    pub fn compile(level: &Level, models: &BlockModelSet, atlas: &TextureAtlas) -> RenderMesh {
        let mut mesh = RenderMesh::new();
        let block_colors = BlockColors::vanilla();

        for chunk in level.iter_chunks() {
            let chunk_pos = chunk.get_pos();
            let base_x = chunk_pos.get_min_block_x();
            let base_z = chunk_pos.get_min_block_z();

            for (section_y, section) in chunk.sections() {
                let base_y = section_y << 4;
                for y in 0..16u8 {
                    for z in 0..16u8 {
                        for x in 0..16u8 {
                            let state = section.get_block_state(x, y, z);
                            if state.is_air() || state.get_render_shape() != RenderShape::MODEL {
                                continue;
                            }
                            let Some(state_model) = models.get(state) else {
                                continue;
                            };

                            let block_pos = BlockPos::new(
                                base_x + x as i32,
                                base_y + y as i32,
                                base_z + z as i32,
                            );

                            let selected =
                                select_models(state_model.multipart, &state_model.parts, block_pos);
                            for model in selected {
                                for quad in &model.quads {
                                    if let Some(cull_face) = quad.cull_face {
                                        let neighbor =
                                            level.get_block_state(block_pos.relative(cull_face));
                                        if occludes(neighbor)
                                            || state.get_block().skip_rendering(neighbor)
                                        {
                                            continue;
                                        }
                                    }

                                    let Some(sprite) = atlas.get_sprite(&quad.sprite) else {
                                        continue;
                                    };

                                    let mut color = block_colors
                                        .get_color(state, quad.tint_index)
                                        .unwrap_or([1.0, 1.0, 1.0]);
                                    let shade = face_shade(quad.shade_direction);
                                    color = [color[0] * shade, color[1] * shade, color[2] * shade];

                                    let mut vertices = [RenderVertex::default(); 4];
                                    for (index, vertex) in vertices.iter_mut().enumerate() {
                                        let local = quad.positions[index];
                                        vertex.position = [
                                            block_pos.get_x() as f32 + local[0],
                                            block_pos.get_y() as f32 + local[1],
                                            block_pos.get_z() as f32 + local[2],
                                        ];
                                        vertex.color = color;
                                        // 烘焙阶段已经把模型 UV 归一化到 0..1。
                                        vertex.uv = [
                                            sprite.get_u(quad.uvs[index][0]),
                                            sprite.get_v(quad.uvs[index][1]),
                                        ];
                                    }
                                    mesh.push_quad(vertices);
                                }
                            }
                        }
                    }
                }
            }
        }

        mesh
    }
}

/// 按 Java `ModelBlockRenderer.tesselateBlock` 的随机序列选择模型：
/// - 普通 variants：用 `Mth.getSeed(pos)` 直接抽取；
/// - multipart：先抽一个 long，再对每个 part 用它重置随机源后抽取。
fn select_models<'a>(
    multipart: bool,
    parts: &'a [BakedPart],
    pos: BlockPos,
) -> Vec<&'a crate::net::minecraft::client::resources::model::BakedModel> {
    let seed = Mth::get_seed_xyz(pos.get_x(), pos.get_y(), pos.get_z());
    let mut random = LegacyRandomSource::new(seed);
    let mut selected = Vec::new();

    if multipart {
        let part_seed = random.next_long();
        for part in parts {
            let mut part_random = LegacyRandomSource::new(part_seed);
            if let Some(model) = choose_weighted(part, &mut part_random) {
                selected.push(model);
            }
        }
    } else if let Some(part) = parts.first() {
        if let Some(model) = choose_weighted(part, &mut random) {
            selected.push(model);
        }
    }

    selected
}

/// 对应 `WeightedList.getRandomOrThrow` 的累积权重选择。
fn choose_weighted<'a>(
    part: &'a BakedPart,
    random: &mut impl RandomSource,
) -> Option<&'a crate::net::minecraft::client::resources::model::BakedModel> {
    let total: i32 = part.variants.iter().map(|(_, weight)| *weight).sum();
    if total <= 0 {
        return None;
    }
    let mut selection = random.next_int_bounded(total);
    for (model, weight) in &part.variants {
        if selection < *weight {
            return Some(model);
        }
        selection -= *weight;
    }
    None
}

/// 邻居是否应该剔除当前面：只有“完整遮挡形状”的方块才剔除。
/// 树叶/玻璃等 `noOcclusion` 方块保持相邻面可见。
fn occludes(state: crate::net::minecraft::world::level::block::state::BlockState) -> bool {
    !state.is_air() && state.get_block().is_solid_render(state)
}

/// Minecraft 固定的面向亮度：顶面 1.0、底面 0.5、南北 0.8、东西 0.6。
fn face_shade(direction: Direction) -> f32 {
    match direction {
        Direction::UP => 1.0,
        Direction::DOWN => 0.5,
        Direction::NORTH | Direction::SOUTH => 0.8,
        Direction::WEST | Direction::EAST => 0.6,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::minecraft::client::multiplayer::ClientLevel;

    #[test]
    fn compiles_flat_world_into_visible_top_faces() {
        use crate::net::minecraft::server::packs::resources::ResourceManager;
        use std::path::Path;

        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        let root = manifest
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("minecraft-26.3-client-src");
        if !root.exists() {
            eprintln!("跳过：未找到 minecraft-26.3-client-src");
            return;
        }

        let resource_manager = ResourceManager::new(root);
        let models = BlockModelSet::load(&resource_manager).unwrap();
        let atlas = crate::net::minecraft::client::renderer::texture::SpriteLoader::load_atlas(
            &resource_manager,
            models.textures(),
        );
        let level = ClientLevel::new_flat(1, 0);
        let mesh = SectionCompiler::compile(&level, &models, &atlas);
        assert!(!mesh.is_empty());
        // 9 个区块 × 256 个草方块顶面；边缘不参与剔除的侧面另行增加。
        assert!(mesh.quad_count() >= 9 * 256);
        assert_eq!(mesh.indices.len(), mesh.quad_count() * 6);
    }
}
