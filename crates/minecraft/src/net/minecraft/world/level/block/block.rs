//! 对应 `net/minecraft/world/level/block/Block.java` 的基础部分。
//!
//! 当前只保留“可以被注册、能提供默认状态、能报告渲染/碰撞属性”的最小
//! 行为；朝向、随机刻、掉落物等行为随具体方块类逐步移植。

use super::RenderShape;
use super::state::block_behaviour::Properties;
use super::state::{BlockState, StateDefinition};
use crate::net::minecraft::resources::Identifier;

pub struct Block {
    properties: Properties,
    state_definition: StateDefinition,
    render_shape: RenderShape,
}

impl Block {
    /// 对应 `new Block(Properties)`。
    pub fn new(properties: Properties) -> Self {
        Self::new_with_render_shape(properties, RenderShape::MODEL)
    }

    pub(crate) fn new_with_render_shape(properties: Properties, render_shape: RenderShape) -> Self {
        let state_definition = StateDefinition::create(
            properties.block_id,
            properties.get_state_properties().to_vec(),
        );
        Self {
            properties,
            state_definition,
            render_shape,
        }
    }

    pub fn default_block_state(&self) -> BlockState {
        self.state_definition.get_default_state()
    }

    pub fn get_state_definition(&self) -> &StateDefinition {
        &self.state_definition
    }

    /// 对应 `getRenderShape(BlockState)`。当前所有变体共享同一渲染形状；
    /// 需要按状态切换时再引入虚函数（例如箱子开合）。
    pub fn get_render_shape(&self, _state: BlockState) -> RenderShape {
        self.render_shape
    }

    pub fn is_air(&self) -> bool {
        self.properties.is_air()
    }

    pub fn has_collision(&self, _state: BlockState) -> bool {
        self.properties.has_collision()
    }

    pub fn can_be_replaced(&self) -> bool {
        self.properties.can_be_replaced()
    }

    pub fn get_properties(&self) -> &Properties {
        &self.properties
    }

    /// Java 中是 `getDescriptionId()`；注册表里的名字更常用，先以标识符暴露。
    pub fn get_name(&self) -> &Identifier {
        self.properties
            .get_id()
            .expect("方块必须在注册前设置标识符")
    }

    pub fn get_block_id(&self) -> u16 {
        self.properties.block_id
    }
}
