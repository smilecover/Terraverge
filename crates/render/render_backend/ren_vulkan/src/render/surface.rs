//! 窗口表面（`VkSurfaceKHR`）创建。

use ash::{Entry, vk};
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use std::error::Error;

/// 创建窗口表面（教程的 createSurface；ash-window 相当于 glfwCreateWindowSurface）。
pub(super) fn create_surface(
    entry: &Entry,
    instance: &ash::Instance,
    display_handle: RawDisplayHandle,
    window_handle: RawWindowHandle,
) -> Result<vk::SurfaceKHR, Box<dyn Error + Send + Sync>> {
    let surface = unsafe {
        ash_window::create_surface(entry, instance, display_handle, window_handle, None)?
    };
    Ok(surface)
}
