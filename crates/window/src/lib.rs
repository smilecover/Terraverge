mod app;
mod context;
mod game_winit;
mod window_config;

pub use app::{App, WindowEvent};
pub use context::Context;
pub use window_config::WindowConfig;

pub fn run<A: App>(app: A) -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = winit::event_loop::EventLoop::new()?;
    let mut runner = game_winit::Runner::new(app);
    event_loop.run_app(&mut runner)?;
    Ok(())
}
