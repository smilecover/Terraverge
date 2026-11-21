use winit::error::ExternalError;
use winit::event_loop::ActiveEventLoop;
use winit::raw_window_handle::{
    HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle, RawWindowHandle,
};
use winit::window::{CursorGrabMode, Window};

pub struct Context<'a> {
    event_loop: &'a ActiveEventLoop,
    window: &'a Window,
}

impl<'a> Context<'a> {
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

    /// 捕获鼠标。优先使用 `Locked`；部分平台不支持时退回 `Confined`。
    pub fn set_cursor_grab(&self, grab: bool) -> Result<(), ExternalError> {
        if grab {
            self.window
                .set_cursor_grab(CursorGrabMode::Locked)
                .or_else(|_| self.window.set_cursor_grab(CursorGrabMode::Confined))
        } else {
            self.window.set_cursor_grab(CursorGrabMode::None)
        }
    }

    pub fn set_cursor_visible(&self, visible: bool) {
        self.window.set_cursor_visible(visible);
    }

    pub fn exit(&self) {
        self.event_loop.exit();
    }
}
