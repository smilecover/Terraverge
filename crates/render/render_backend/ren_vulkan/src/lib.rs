use anyhow::{Context, Result};
use ash::vk;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use std::ffi::c_char;

pub struct Render {
    _entry: ash::Entry, // Entry 必须比 Instance 活得久
    instance: ash::Instance,
}

impl Render {
    pub fn new<W: HasWindowHandle + HasDisplayHandle>(window: &W) -> Result<Self> {
        unsafe {
            let entry = ash::Entry::load().context("未找到 Vulkan 运行库")?;

            let display = window.display_handle()?.as_raw();
            let window_handle = window.window_handle()?.as_raw();

            let mut exts: Vec<*const c_char> =
                ash_window::enumerate_required_extensions(display)?.to_vec();
            if cfg!(debug_assertions) {
                exts.push(ash::ext::debug_utils::NAME.as_ptr());
            }

            let app_info = vk::ApplicationInfo::default()
                .application_name(c"Terraverge")
                .api_version(vk::make_api_version(0, 1, 4, 0))
                .application_version(vk::make_api_version(1, 0, 0, 0))
                .engine_name(c"Terraverge")
                .engine_version(vk::make_api_version(1, 0, 0, 0));

            let create_info = vk::InstanceCreateInfo::default()
                .application_info(&app_info)
                .enabled_extension_names(&exts);

            let instance = entry.create_instance(&create_info, None)?;

            Ok(Self {
                _entry: entry,
                instance,
            })
        }
    }
}
