//! 对应 `net/minecraft/client/resources/model/ModelManager.java` /
//! `ModelBakery.java` 的合并模型加载与烘焙流程。
//!
//! 流程：
//! 1. `BlockStateModelLoader` 选出每个状态的 variants/multipart 集合；
//! 2. 解析模型 JSON，沿 `parent` 链合并元素与 `textures` 槽位；
//! 3. 对每个候选变体烘焙 `BakedQuad`（包含变体/元素旋转与 uvlock）；
//! 4. 汇总所有引用到的贴图，交给 `SpriteLoader` 构建图集。

use super::block_state_model_loader::BlockStateModelLoader;
use super::cuboid::{CuboidModel, CuboidModelElement, FaceBakery};
use super::geometry::BakedModel;
use super::model_error::ModelError;
use super::sprite::TextureSlots;
use crate::net::minecraft::client::renderer::block::dispatch::{
    BlockModelSelection, Variant, WeightedVariant,
};
use crate::net::minecraft::resources::Identifier;
use crate::net::minecraft::server::packs::resources::ResourceManager;
use crate::net::minecraft::util::json;
use crate::net::minecraft::world::level::block::state::BlockState;
use std::collections::{HashMap, HashSet};

#[derive(Clone)]
struct ResolvedCuboidModel {
    elements: Vec<CuboidModelElement>,
    textures: TextureSlots,
    ambient_occlusion: bool,
}

/// 一个 multipart/variants 部件：按权重选择一个已烘焙模型。
#[derive(Clone, Debug)]
pub struct BakedPart {
    pub variants: Vec<(BakedModel, i32)>,
}

/// 一个方块状态的完整烘焙结果。
#[derive(Clone, Debug, Default)]
pub struct BakedStateModel {
    /// 是否来自 `multipart`（影响随机种子消耗顺序）。
    pub multipart: bool,
    /// 普通 variants 只有 1 个 part；multipart 每个命中的 apply 一个 part。
    pub parts: Vec<BakedPart>,
}

impl BakedStateModel {
    pub fn is_empty(&self) -> bool {
        self.parts
            .iter()
            .all(|part| part.variants.iter().all(|(model, _)| model.is_empty()))
    }
}

/// 全部方块状态的烘焙结果与图集依赖。
pub struct BlockModelSet {
    models: HashMap<BlockState, BakedStateModel>,
    textures: Vec<Identifier>,
}

impl BlockModelSet {
    pub fn get(&self, state: BlockState) -> Option<&BakedStateModel> {
        self.models.get(&state)
    }

    pub fn textures(&self) -> &[Identifier] {
        &self.textures
    }

    pub fn len(&self) -> usize {
        self.models.len()
    }

    pub fn is_empty(&self) -> bool {
        self.models.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&BlockState, &BakedStateModel)> {
        self.models.iter()
    }
}

struct ModelBakery<'a> {
    resource_manager: &'a ResourceManager,
    cache: HashMap<Identifier, ResolvedCuboidModel>,
    stack: Vec<Identifier>,
}

impl<'a> ModelBakery<'a> {
    fn new(resource_manager: &'a ResourceManager) -> Self {
        Self {
            resource_manager,
            cache: HashMap::new(),
            stack: Vec::new(),
        }
    }

    fn resolve_model(&mut self, id: &Identifier) -> Result<ResolvedCuboidModel, ModelError> {
        if let Some(cached) = self.cache.get(id) {
            return Ok(cached.clone());
        }
        if self.stack.contains(id) {
            let chain = self
                .stack
                .iter()
                .map(ToString::to_string)
                .chain(std::iter::once(id.to_string()))
                .collect::<Vec<_>>()
                .join(" -> ");
            return Err(ModelError::new(format!("模型父子循环: {chain}")));
        }

        self.stack.push(id.clone());
        let result = self.parse_model(id);
        self.stack.pop();
        let resolved = result?;
        self.cache.insert(id.clone(), resolved.clone());
        Ok(resolved)
    }

    fn parse_model(&mut self, id: &Identifier) -> Result<ResolvedCuboidModel, ModelError> {
        let resource = self
            .resource_manager
            .get_model_json(id)
            .map_err(|err| ModelError::new(format!("读取模型 {id} 失败: {err}")))?;
        let text = resource
            .read_string()
            .map_err(|err| ModelError::new(format!("模型 {id} 不是 UTF-8: {err}")))?;
        let value = json::parse(&text)?;
        let model = CuboidModel::from_json(&value)
            .map_err(|err| ModelError::new(format!("解析模型 {id} 失败: {err}")))?;

        let (mut elements, mut textures, mut ambient_occlusion) =
            if let Some(parent_id) = &model.parent {
                let parent = self.resolve_model(parent_id)?;
                (parent.elements, parent.textures, parent.ambient_occlusion)
            } else {
                (Vec::new(), TextureSlots::empty(), true)
            };

        if let Some(geometry) = &model.geometry {
            elements = geometry.elements.clone();
        }
        textures.override_from(&model.texture_slots);
        if let Some(ambient) = model.ambient_occlusion {
            ambient_occlusion = ambient;
        }

        Ok(ResolvedCuboidModel {
            elements,
            textures,
            ambient_occlusion,
        })
    }

    fn bake_variant(
        &self,
        state: BlockState,
        weighted: &WeightedVariant,
    ) -> Result<BakedModel, ModelError> {
        let variant: &Variant = &weighted.variant;
        let model = self
            .cache
            .get(&variant.model)
            .cloned()
            .ok_or_else(|| ModelError::new("模型尚未解析"))?;

        let mut quads = Vec::new();
        for element in &model.elements {
            let rotation = element.rotation.as_ref();
            for (&facing, face) in &element.faces {
                if !element.should_draw_face(facing) {
                    continue;
                }
                let material = model.textures.resolve(&face.texture).map_err(|err| {
                    ModelError::new(format!(
                        "方块状态 {state} 的 {} 面贴图解析失败: {err}",
                        facing.get_name()
                    ))
                })?;
                quads.push(FaceBakery::bake_quad(
                    element.from,
                    element.to,
                    face,
                    material,
                    facing,
                    variant,
                    rotation,
                    element.shade_direction_override,
                ));
            }
        }

        Ok(BakedModel::new(quads))
    }

    fn bake_selection(
        &self,
        state: BlockState,
        selection: &BlockModelSelection,
    ) -> Result<BakedStateModel, ModelError> {
        let mut parts = Vec::with_capacity(selection.parts.len());
        for part in &selection.parts {
            let mut baked = Vec::with_capacity(part.len());
            for weighted in part {
                baked.push((self.bake_variant(state, weighted)?, weighted.weight));
            }
            parts.push(BakedPart { variants: baked });
        }
        Ok(BakedStateModel {
            multipart: selection.multipart,
            parts,
        })
    }
}

impl BlockModelSet {
    /// 加载全部方块模型。供游戏层在渲染器初始化前调用一次。
    pub fn load(resource_manager: &ResourceManager) -> Result<Self, ModelError> {
        let selections = BlockStateModelLoader::load(resource_manager)?;
        let mut bakery = ModelBakery::new(resource_manager);

        // 先解析所有被引用的模型，填充缓存。
        for selection in selections.values() {
            for part in &selection.parts {
                for weighted in part {
                    bakery.resolve_model(&weighted.variant.model)?;
                }
            }
        }

        let mut models = HashMap::with_capacity(selections.len());
        let mut textures = HashSet::new();
        let mut texture_order = Vec::new();

        for (state, selection) in &selections {
            let baked = bakery.bake_selection(*state, selection)?;
            for part in &baked.parts {
                for (model, _) in &part.variants {
                    for quad in &model.quads {
                        if textures.insert(quad.sprite.clone()) {
                            texture_order.push(quad.sprite.clone());
                        }
                    }
                }
            }
            models.insert(*state, baked);
        }

        texture_order.sort();
        Ok(Self {
            models,
            textures: texture_order,
        })
    }
}

/// 统计用：所有烘焙四边形的总数（含未被随机选中的加权候选）。
pub fn total_quad_count(set: &BlockModelSet) -> usize {
    set.models
        .values()
        .flat_map(|state| state.parts.iter())
        .flat_map(|part| part.variants.iter())
        .map(|(model, _)| model.quads.len())
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn resource_root() -> Option<std::path::PathBuf> {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        let root = manifest
            .parent()?
            .parent()?
            .join("minecraft-26.3-client-src");
        root.exists().then_some(root)
    }

    #[test]
    fn loads_vanilla_models_when_source_tree_is_present() {
        let Some(root) = resource_root() else {
            eprintln!("跳过：未找到 minecraft-26.3-client-src");
            return;
        };
        let manager = ResourceManager::new(root);
        let set = BlockModelSet::load(&manager).unwrap();
        assert!(set.len() >= 7);
        assert!(!set.textures().is_empty());
        assert!(
            set.get(
                crate::net::minecraft::world::level::block::Blocks::stone().default_block_state()
            )
            .is_some()
        );

        // 草方块 variants 是 4 个 y 旋转的加权列表（每个权重 1）。
        let grass = set
            .get(
                crate::net::minecraft::world::level::block::Blocks::grass_block()
                    .default_block_state(),
            )
            .unwrap();
        assert!(!grass.multipart);
        assert_eq!(grass.parts.len(), 1);
        assert_eq!(grass.parts[0].variants.len(), 4);

        // 玻璃的贴图槽是 `{sprite, force_translucent}` 对象，所有面都应标记半透明。
        let glass = set
            .get(crate::net::minecraft::world::level::block::Blocks::glass().default_block_state())
            .unwrap();
        let glass_quads = &glass.parts[0].variants[0].0.quads;
        assert!(!glass_quads.is_empty());
        assert!(glass_quads.iter().all(|quad| quad.force_translucent));

        // 树叶有 28 个状态（distance 1..7 × persistent × waterlogged）。
        let leaves = crate::net::minecraft::world::level::block::Blocks::oak_leaves();
        assert_eq!(
            leaves.get_state_definition().get_possible_states().len(),
            28
        );
    }
}
