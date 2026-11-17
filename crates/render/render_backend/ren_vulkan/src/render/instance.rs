//! Vulkan 实例创建：加载 `Entry`、实例扩展与校验层、`VkInstance`，以及调试信使。

use ash::{Entry, vk};
use std::error::Error;
use std::ffi::c_char;

use super::debug::validation_layers;
#[cfg(debug_assertions)]
use super::debug::{DEBUG_LEVEL, debug_messenger_create_info};

/// 实例创建阶段的产物，成功创建后由 `Render` 接管。
pub(super) struct InstanceContext {
    pub(super) entry: Entry,
    pub(super) instance: ash::Instance,
    #[cfg(debug_assertions)]
    pub(super) debug_utils: ash::ext::debug_utils::Instance,
    #[cfg(debug_assertions)]
    pub(super) debug_messenger: Option<vk::DebugUtilsMessengerEXT>,
}

/// 创建 Vulkan 实例，并在调试构建下创建调试信使。
/// 失败时不会留下半初始化的实例：信使创建失败会先销毁实例再返回错误。
pub(super) fn create_instance() -> Result<InstanceContext, Box<dyn Error + Send + Sync>> {
    let entry =
        unsafe { Entry::load().inspect_err(|err| eprintln!("链接库查找错误：{err}"))? };

    let application_info = vk::ApplicationInfo::default()
        .application_name(c"Terraverge")
        .application_version(vk::make_api_version(0, 1, 0, 0))
        .api_version(vk::API_VERSION_1_3)
        .engine_name(c"Terraverge")
        .engine_version(vk::make_api_version(0, 1, 0, 0));

    let extension_names = [
        ash::khr::surface::NAME,
        ash::khr::win32_surface::NAME,
        #[cfg(debug_assertions)]
        {
            ash::ext::debug_utils::NAME
        },
    ];

    let extension_ptrs = extension_names.map(|name| name.as_ptr());

    let layer_names = validation_layers(&entry)?;
    let layer_ptrs: Vec<*const c_char> = layer_names.iter().map(|name| name.as_ptr()).collect();

    #[cfg(debug_assertions)]
    let mut debug_create_info = debug_messenger_create_info(DEBUG_LEVEL);

    #[allow(unused_mut)]
    let mut create_info = vk::InstanceCreateInfo::default()
        .application_info(&application_info)
        .enabled_extension_names(&extension_ptrs)
        .enabled_layer_names(&layer_ptrs);

    // 等级 0 表示关闭：连 pNext 里的临时 messenger 也不挂，
    // 否则同样会触发 messageSeverity 为 0 的 VUID 错误。
    #[cfg(debug_assertions)]
    if DEBUG_LEVEL > 0 {
        create_info = create_info.push_next(&mut debug_create_info);
    }

    let instance = unsafe {
        entry
            .create_instance(&create_info, None)
            .inspect_err(|err| match *err {
                vk::Result::ERROR_INCOMPATIBLE_DRIVER => {
                    eprintln!("没有匹配的驱动")
                }
                vk::Result::ERROR_EXTENSION_NOT_PRESENT => {
                    eprintln!("不支持当前vulkan扩展")
                }

                _ => {
                    eprintln!("创建vulkan实例错误")
                }
            })?
    };

    #[cfg(debug_assertions)]
    let debug_utils = ash::ext::debug_utils::Instance::new(&entry, &instance);

    // 实例创建成功后创建真正的调试信使；失败时回收实例再返回。
    // 等级 0 = 关闭：完全不创建 messenger（messageSeverity 不允许为 0）。
    #[cfg(debug_assertions)]
    let debug_messenger = if DEBUG_LEVEL == 0 {
        None
    } else {
        let create_info = debug_messenger_create_info(DEBUG_LEVEL);
        match unsafe { debug_utils.create_debug_utils_messenger(&create_info, None) } {
            Ok(messenger) => Some(messenger),
            Err(err) => {
                eprintln!("创建调试信使失败：{err}");
                unsafe { instance.destroy_instance(None) };
                return Err(err.into());
            }
        }
    };

    Ok(InstanceContext {
        entry,
        instance,
        #[cfg(debug_assertions)]
        debug_utils,
        #[cfg(debug_assertions)]
        debug_messenger,
    })
}
