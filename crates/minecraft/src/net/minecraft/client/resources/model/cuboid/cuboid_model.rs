//! 对应 `net/minecraft/client/resources/model/cuboid/CuboidModel.java`。
//!
//! JSON 字段：`parent`、`textures`、`elements`、`ambientocclusion`。
//! `display`/`gui_light` 属于物品 GUI 渲染，暂未使用。

use super::super::sprite::TextureSlots;
use super::cuboid_model_element::CuboidModelElement;
use crate::net::minecraft::resources::Identifier;
use crate::net::minecraft::util::json::{JsonError, JsonValue};

#[derive(Clone, Debug, Default)]
pub struct UnbakedCuboidGeometry {
    pub elements: Vec<CuboidModelElement>,
}

impl UnbakedCuboidGeometry {
    pub fn new(elements: Vec<CuboidModelElement>) -> Self {
        Self { elements }
    }
}

#[derive(Clone, Debug, Default)]
pub struct CuboidModel {
    pub geometry: Option<UnbakedCuboidGeometry>,
    pub ambient_occlusion: Option<bool>,
    pub texture_slots: TextureSlots,
    pub parent: Option<Identifier>,
}

impl CuboidModel {
    pub fn from_json(value: &JsonValue) -> Result<Self, JsonError> {
        let geometry = match value.get("elements") {
            None => None,
            Some(elements) => {
                let array = elements
                    .as_array()
                    .ok_or_else(|| JsonError::message("`elements` 必须是数组"))?;
                let mut parsed = Vec::with_capacity(array.len());
                for element in array {
                    parsed.push(CuboidModelElement::from_json(element)?);
                }
                Some(UnbakedCuboidGeometry::new(parsed))
            }
        };

        let texture_slots = match value.get("textures") {
            Some(textures) => TextureSlots::from_json(textures)?,
            None => TextureSlots::empty(),
        };

        let parent = match value.optional_str("parent") {
            None | Some("") => None,
            Some(parent) => Some(Identifier::parse(parent).map_err(|err| {
                JsonError::message(format!("非法父模型标识符 `{parent}`: {err}"))
            })?),
        };

        Ok(Self {
            geometry,
            ambient_occlusion: value.optional_bool("ambientocclusion"),
            texture_slots,
            parent,
        })
    }
}
