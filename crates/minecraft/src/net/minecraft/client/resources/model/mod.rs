//! `net.minecraft.client.resources.model` 包。

mod block_state_model_loader;
mod model_error;
mod model_manager;

pub mod cuboid;
pub mod geometry;
pub mod sprite;

pub use model_error::ModelError;
pub use model_manager::{BlockModelSet, total_quad_count};
