//! `net.minecraft.client.renderer.block.dispatch` 包。

mod block_state_model_dispatcher;
pub mod multipart;
mod variant;

pub use block_state_model_dispatcher::{BlockModelSelection, BlockStateModelDispatcher};
pub use variant::{Variant, WeightedVariant};
