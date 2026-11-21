use crate::context::Context;
use crate::window_config::WindowConfig;
use winit::event::{ElementState, MouseButton};
use winit::keyboard::KeyCode;

/// 窗口/输入事件。
///
/// 与 winit 类型解耦的成本很高（KeyCode 数百个变体），且 `window` crate
/// 本来就是 winit 的薄封装，因此直接复用 winit 的输入枚举。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WindowEvent {
    CloseRequested,
    Resized {
        width: u32,
        height: u32,
    },
    RedrawRequested,
    Focused(bool),
    ScaleFactorChanged {
        scale_factor: f64,
    },
    KeyboardInput {
        key: KeyCode,
        state: ElementState,
        repeat: bool,
    },
    /// 原始鼠标位移（相对上一帧），用于旋转相机。
    MouseMotion {
        delta_x: f64,
        delta_y: f64,
    },
    MouseButton {
        button: MouseButton,
        state: ElementState,
    },
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
