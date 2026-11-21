//! `net.minecraft.client` 包。
//!
//! 26.3 客户端包含窗口、输入、渲染、多人连接、声音等子系统；移植按
//! `docs/PROGRESS.md` 逐包推进。当前已建立资源、模型、图集、网格与玩家
//! 相机的基础链路。

pub mod camera;
pub mod color;
pub mod input;
pub mod multiplayer;
pub mod player;
pub mod renderer;
pub mod resources;
