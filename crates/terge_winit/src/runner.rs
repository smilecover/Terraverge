use winit::application::ApplicationHandler;
use winit::error::EventLoopError;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

/// 游戏侧实现的接口。
///
/// 窗口创建、事件循环、关闭流程都由 [`run_game`] 负责，
/// 游戏只需要关心“窗口好了”“来了一个事件”“该推进一帧了”。
pub trait App {
    /// 窗口创建完成（或应用从挂起中恢复）后调用。
    fn resumed(&mut self, window: &Window) {
        let _ = window;
    }

    /// 每个窗口事件。
    ///
    /// 按键事件建议交给输入映射层处理，而不是在这里直接 `match` 物理键。
    fn window_event(&mut self, event: &WindowEvent) {
        let _ = event;
    }

    /// 一轮事件处理结束，适合在这里推进一帧游戏逻辑。
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let _ = event_loop;
    }
}

/// 把 winit 的 [`ApplicationHandler`] 适配到 [`App`]。
struct Runner<A: App> {
    app: A,
    window: Option<Window>,
}

impl<A: App> ApplicationHandler for Runner<A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window = event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("Terraverge")
                        .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 720.0)),
                )
                .expect("创建窗口失败");
            self.window = Some(window);
        }

        if let Some(window) = &self.window {
            self.app.resumed(window);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let close_requested = matches!(event, WindowEvent::CloseRequested);
        self.app.window_event(&event);

        if close_requested {
            println!("The close button was pressed; stopping");
            event_loop.exit();
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.app.about_to_wait(event_loop);
    }
}

/// 创建窗口并运行游戏主循环，直到窗口关闭。
pub fn run_game<A: App>(app: A) -> Result<(), EventLoopError> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut runner = Runner { app, window: None };
    event_loop.run_app(&mut runner)
}
