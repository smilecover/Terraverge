use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::raw_window_handle::{
    HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle,
};

pub use winit::error::EventLoopError;
pub use winit::event::WindowEvent;
pub use winit::window::Window;

pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
}

impl WindowConfig {
    pub fn new(title: impl Into<String>, width: u32, height: u32) -> Self {
        Self {
            title: title.into(),
            width,
            height,
        }
    }
}

pub struct Context<'a> {
    event_loop: &'a ActiveEventLoop,
    window: &'a Window,
}

impl<'a> Context<'a> {
    pub fn window(&self) -> &'a Window {
        self.window
    }

    pub fn size(&self) -> (u32, u32) {
        let size = self.window.inner_size();
        (size.width, size.height)
    }

    pub fn display_handle(&self) -> Result<RawDisplayHandle, HandleError> {
        self.window.display_handle().map(|handle| handle.as_raw())
    }

    pub fn window_handle(&self) -> Result<RawWindowHandle, HandleError> {
        self.window.window_handle().map(|handle| handle.as_raw())
    }

    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }

    pub fn exit(&self) {
        self.event_loop.exit();
    }
}

pub trait App {
    fn window_config(&self) -> WindowConfig;

    fn resumed(&mut self, _ctx: &Context) {}

    fn window_event(&mut self, ctx: &Context, event: WindowEvent);

    fn about_to_wait(&mut self, _ctx: &Context) {}

    fn exiting(&mut self, _ctx: &Context) {}
}

pub fn run<A: App>(app: A) -> Result<(), EventLoopError> {
    let event_loop = EventLoop::new()?;
    let mut runner = Runner { app, window: None };
    event_loop.run_app(&mut runner)
}

struct Runner<A> {
    app: A,
    window: Option<Window>,
}

impl<A: App> ApplicationHandler for Runner<A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let config = self.app.window_config();
        let attributes = Window::default_attributes()
            .with_title(config.title.as_str())
            .with_inner_size(LogicalSize::new(config.width, config.height));

        match event_loop.create_window(attributes) {
            Ok(window) => {
                self.app.resumed(&Context {
                    event_loop,
                    window: &window,
                });
                self.window = Some(window);
            }
            Err(err) => {
                eprintln!("创建窗口失败: {err}");
                event_loop.exit();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        if let Some(window) = &self.window {
            self.app
                .window_event(&Context { event_loop, window }, event);
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            self.app.about_to_wait(&Context { event_loop, window });
        }
    }

    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            self.app.exiting(&Context { event_loop, window });
        }
        self.window = None;
    }
}
