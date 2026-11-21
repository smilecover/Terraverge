//! `net.minecraft.util.random` 包。

mod random_source;
mod weighted;

pub use random_source::RandomSource;
pub use weighted::{Weighted, WeightedList};
