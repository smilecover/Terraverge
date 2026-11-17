use crate::context::Context;
use crate::window_config::WindowConfig;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WindowEvent {
    CloseRequested,
    Resized { width: u32, height: u32 },
    RedrawRequested,
    Focused(bool),
    ScaleFactorChanged { scale_factor: f64 },
}

pub trait App {
    fn window_config(&self) -> WindowConfig;

    fn resumed(&mut self, _ctx: &Context<'_>) {}

    fn window_event(&mut self, _ctx: &Context<'_>, _event: WindowEvent) {}

    fn about_to_wait(&mut self, ctx: &Context<'_>) {
        ctx.request_redraw();
    }

    fn exiting(&mut self, _ctx: &Context<'_>) {}
}
