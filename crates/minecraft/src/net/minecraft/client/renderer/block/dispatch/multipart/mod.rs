//! `net.minecraft.client.renderer.block.dispatch.multipart` 包。

mod condition;
mod selector;

pub use condition::{Condition, Term};
pub use selector::MultiPartSelector;
