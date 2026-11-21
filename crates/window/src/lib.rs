mod app;
mod context;
mod game_winit;
mod window_config;

pub use app::{App, WindowEvent};
pub use context::Context;
pub use window_config::WindowConfig;

// 输入类型直接复用 winit，避免维护一份重复的键码表。
pub use winit::event::{ElementState, MouseButton};
pub use winit::keyboard::KeyCode;

pub fn run<A: App>(app: A) -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = winit::event_loop::EventLoop::new()?;
    let mut runner = game_winit::Runner::new(app);
    event_loop.run_app(&mut runner)?;
    Ok(())
}
