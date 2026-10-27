//! Terraverge 对 winit 的薄封装。
//!
//! 游戏侧只实现 [`App`] trait，不需要直接依赖 winit：
//! 需要命名的事件 / 窗口类型由本 crate 重导出。

pub mod runner;

pub use runner::{run_game, App};
pub use winit;
pub use winit::error::EventLoopError;
pub use winit::event::{ElementState, WindowEvent};
pub use winit::event_loop::ActiveEventLoop;
pub use winit::keyboard::{KeyCode, PhysicalKey};
pub use winit::window::Window;
