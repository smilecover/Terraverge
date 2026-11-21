//! `net.minecraft` 根包。

pub mod client;
pub mod core;
pub mod resources;
pub mod server;
pub mod util;
pub mod world;

use std::error::Error;
use std::fmt;

/// 对应 `net.minecraft.IdentifierException`。
///
/// Java 侧在解析/构造非法标识符时抛出；Rust 侧统一用 `Result` 表达，
/// 错误类型就是本结构，避免把 panic 当作控制流。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentifierException {
    message: String,
}

impl IdentifierException {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for IdentifierException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for IdentifierException {}
