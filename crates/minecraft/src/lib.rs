//! Minecraft 26.3 client 的 Rust 移植。
//!
//! 目录结构与 Java 源码一一对应：
//!
//! ```text
//! minecraft-26.3-client-src/net/minecraft/core/BlockPos.java
//!   -> crates/minecraft/src/net/minecraft/core/block_pos.rs
//! ```
//!
//! - 包路径保持原样（`net::minecraft::...`），只把文件名改成 snake_case；
//! - 类型名、字段语义、常量值保持不变；
//! - Java 重载在 Rust 中用后缀区分（详见 `docs/PORTING.md`）；
//! - 暂未实现的方法不会伪造行为：要么明确 `todo!`/返回 `Unsupported`，
//!   要么在 `docs/PROGRESS.md` 中登记。
//!
//! 这个 crate 不依赖渲染后端和窗口系统，保持与 Java 侧 `client` 以下
//! 类库同样的纯净度；真正的 GPU 后端在 `ren_vulkan`，由 `game` 层粘合。

pub mod com;
pub mod net;
