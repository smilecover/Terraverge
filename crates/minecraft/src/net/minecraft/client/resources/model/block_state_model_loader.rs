//! 对应 `net/minecraft/client/resources/model/BlockStateModelLoader.java`。
//!
//! 遍历内置方块注册表，读取 `blockstates/<路径>.json`，为每个方块状态
//! 选出 `variants` 或 `multipart` 模型集合。26.3 的多资源包栈与异步加载
//! 在此简化为单目录、同步。

use super::model_error::ModelError;
use crate::net::minecraft::client::renderer::block::dispatch::{
    BlockModelSelection, BlockStateModelDispatcher,
};
use crate::net::minecraft::core::registries::builtin_registries;
use crate::net::minecraft::server::packs::resources::ResourceManager;
use crate::net::minecraft::util::json;
use crate::net::minecraft::world::level::block::state::BlockState;
use std::collections::HashMap;

pub struct BlockStateModelLoader;

impl BlockStateModelLoader {
    /// 返回 `方块状态 -> 模型选择` 映射。找不到定义时会明确报错。
    pub fn load(
        resource_manager: &ResourceManager,
    ) -> Result<HashMap<BlockState, BlockModelSelection>, ModelError> {
        let mut result = HashMap::new();

        for block in builtin_registries::block().iter() {
            let block_id = block.get_name().clone();
            let resource = resource_manager
                .get_blockstate_json(&block_id)
                .map_err(|err| ModelError::new(format!("读取方块 {block_id} 的状态失败: {err}")))?;
            let text = resource.read_string().map_err(|err| {
                ModelError::new(format!("方块 {block_id} 的状态不是 UTF-8: {err}"))
            })?;
            let value = json::parse(&text)?;
            let dispatcher = BlockStateModelDispatcher::from_json(&value)
                .map_err(|err| ModelError::new(format!("解析方块 {block_id} 的状态失败: {err}")))?;

            let definition = block.get_state_definition();
            for state in definition.get_possible_states() {
                let selection = dispatcher
                    .select(definition, *state)
                    .map_err(|err| {
                        ModelError::new(format!(
                            "方块 {block_id} 的状态 {} 选择模型失败: {err}",
                            state.property_key()
                        ))
                    })?
                    .ok_or_else(|| {
                        ModelError::new(format!(
                            "方块 {block_id} 的状态 {} 没有匹配的变体",
                            state.property_key()
                        ))
                    })?;
                result.insert(*state, selection);
            }
        }

        Ok(result)
    }
}
