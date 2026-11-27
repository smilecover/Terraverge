mod game;
mod game_windows;
mod game_winit;
mod windows_config;

pub use game::{Game, WindowEvent};
pub use game_windows::GameWindows;
pub use windows_config::WindowConfig;

pub fn run<A: Game>(game: A) -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = winit::event_loop::EventLoop::new()?;
    let mut runner = game_winit::Runner::new(game);
    event_loop.run_app(&mut runner)?;
    Ok(())
}
