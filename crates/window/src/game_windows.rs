use winit::event_loop::ActiveEventLoop;
use winit::raw_window_handle::{
    HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle,
};
use winit::window::Window;

pub struct GameWindows<'a> {
    event_loop: &'a ActiveEventLoop,
    window: &'a Window,
}

impl<'a> GameWindows<'a> {
    pub(crate) fn new(event_loop: &'a ActiveEventLoop, window: &'a Window) -> Self {
        Self { event_loop, window }
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
