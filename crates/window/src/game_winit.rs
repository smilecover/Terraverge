use crate::game::{Game, WindowEvent};
use crate::game_windows::GameWindows;
use crate::windows_config::WindowConfig;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent as WinitWindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

pub(crate) struct Runner<A> {
    app: A,
    window: Option<Window>,
}

impl<A: Game> Runner<A> {
    pub(crate) fn new(app: A) -> Self {
        Self { app, window: None }
    }
}

impl<A: Game> ApplicationHandler for Runner<A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let config = self.app.window_config();
        let attributes = Window::default_attributes()
            .with_title(&config.title)
            .with_inner_size(LogicalSize::new(config.width, config.height))
            .with_min_inner_size(LogicalSize::new(
                WindowConfig::MIN_WIDTH,
                WindowConfig::MIN_HEIGHT,
            ));

        let window = match event_loop.create_window(attributes) {
            Ok(window) => window,
            Err(err) => {
                eprintln!("创建窗口失败: {err}");
                event_loop.exit();
                return;
            }
        };

        self.app.resumed(&GameWindows::new(event_loop, &window));
        self.window = Some(window);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WinitWindowEvent,
    ) {
        let Some(window) = self.window.as_ref() else {
            return;
        };
        let Some(event) = translate(&event) else {
            return;
        };
        self.app
            .window_event(&GameWindows::new(event_loop, window), event);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(window) = self.window.as_ref() {
            self.app
                .about_to_wait(&GameWindows::new(event_loop, window));
        }
    }

    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(window) = self.window.as_ref() {
            self.app.exiting(&GameWindows::new(event_loop, window));
        }
        self.window = None;
    }
}

fn translate(event: &WinitWindowEvent) -> Option<WindowEvent> {
    Some(match event {
        WinitWindowEvent::CloseRequested => WindowEvent::CloseRequested,
        WinitWindowEvent::Resized(size) => WindowEvent::Resized {
            width: size.width,
            height: size.height,
        },
        WinitWindowEvent::RedrawRequested => WindowEvent::RedrawRequested,
        WinitWindowEvent::Focused(focused) => WindowEvent::Focused(*focused),
        WinitWindowEvent::ScaleFactorChanged { scale_factor, .. } => {
            WindowEvent::ScaleFactorChanged {
                scale_factor: *scale_factor,
            }
        }
        _ => return None,
    })
}
