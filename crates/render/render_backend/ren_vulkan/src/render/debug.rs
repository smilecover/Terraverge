//! 调试与校验层：候选层列表、调试消息回调、调试信使创建信息。

use ash::{Entry, vk};
use std::ffi::CStr;

/*
本机 Vulkan SDK（1.4.363，D:\VulkanSDK）注册的 9 个层：
VK_LAYER_KHRONOS_validation:       综合校验层，开发阶段必开。
VK_LAYER_LUNARG_api_dump:          打印所有 Vulkan API 调用（输出量很大）。
VK_LAYER_LUNARG_crash_diagnostic:  崩溃/挂起诊断，需 VK_CRASH_DIAGNOSTIC_ENABLE=1 才激活。
VK_LAYER_LUNARG_gfxreconstruct:    抓帧录制，默认把所有帧录进 gfxrecon_capture.gfxr。
VK_LAYER_LUNARG_monitor:           在窗口标题显示 FPS。
VK_LAYER_LUNARG_screenshot:        按 VK_SCREENSHOT_FRAMES 截取指定帧。
VK_LAYER_KHRONOS_profiles:         用 profile 文件模拟设备能力。
VK_LAYER_KHRONOS_shader_object:    模拟/校验 VK_EXT_shader_object。
VK_LAYER_KHRONOS_synchronization2: 为旧驱动模拟 VK_KHR_synchronization2。
*/

#[cfg(debug_assertions)]
const CANDIDATE_LAYERS: &[&CStr] = &[
    c"VK_LAYER_KHRONOS_validation",
    // c"VK_LAYER_LUNARG_api_dump",
    c"VK_LAYER_LUNARG_crash_diagnostic",
    c"VK_LAYER_LUNARG_gfxreconstruct",
    c"VK_LAYER_LUNARG_monitor",
    c"VK_LAYER_LUNARG_screenshot",
    c"VK_LAYER_KHRONOS_profiles",
    c"VK_LAYER_KHRONOS_shader_object",
    c"VK_LAYER_KHRONOS_synchronization2",
];

/// 调试信息等级（纯数字，越大越啰嗦）：
/// 0 = 关闭（不创建 messenger），1 = ERROR，2 = +WARNING，3 = +INFO，4 = +VERBOSE（全部）。
/// 传给 `debug_messenger_create_info`；创建期和运行期的 messenger 共用。
#[cfg(debug_assertions)]
pub(super) const DEBUG_LEVEL: u8 = 1;

/// 把 0~4 的数字等级翻译成 Vulkan 的 severity 位标志。
#[cfg(debug_assertions)]
fn debug_severity(level: u8) -> vk::DebugUtilsMessageSeverityFlagsEXT {
    use vk::DebugUtilsMessageSeverityFlagsEXT as Severity;

    let mut severity = Severity::empty();
    if level >= 1 {
        severity |= Severity::ERROR;
    }
    if level >= 2 {
        severity |= Severity::WARNING;
    }
    if level >= 3 {
        severity |= Severity::INFO;
    }
    if level >= 4 {
        severity |= Severity::VERBOSE;
    }
    severity
}

/// 验证层消息回调：把 VVL/其他层的消息统一打印到 stderr。
/// 返回 `vk::FALSE` 表示不中止触发消息的 Vulkan 调用（教程要求始终返回 FALSE）。
#[cfg(debug_assertions)]
unsafe extern "system" fn debug_callback(
    severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    _message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT<'_>,
    _user_data: *mut std::ffi::c_void,
) -> vk::Bool32 {
    // 调用方保证 pCallbackData 和 pMessage 非空。
    let message = unsafe { CStr::from_ptr((*callback_data).p_message) };

    let level = if severity.contains(vk::DebugUtilsMessageSeverityFlagsEXT::ERROR) {
        "ERROR"
    } else if severity.contains(vk::DebugUtilsMessageSeverityFlagsEXT::WARNING) {
        "WARN "
    } else if severity.contains(vk::DebugUtilsMessageSeverityFlagsEXT::INFO) {
        "INFO "
    } else {
        "DEBUG"
    };

    eprintln!("[Vulkan {level}] {}", message.to_string_lossy());
    vk::FALSE
}

/// 同时用于 vkCreateDebugUtilsMessengerEXT 和 VkInstanceCreateInfo::pNext，
/// 后者可以捕获 vkCreateInstance / vkDestroyInstance 期间的消息。
/// `level` 是 0~4 的数字等级（0 关闭，4 最啰嗦），见模块顶部的 `DEBUG_LEVEL`。
#[cfg(debug_assertions)]
pub(super) fn debug_messenger_create_info(
    level: u8,
) -> vk::DebugUtilsMessengerCreateInfoEXT<'static> {
    vk::DebugUtilsMessengerCreateInfoEXT::default()
        .message_severity(debug_severity(level))
        .message_type(
            vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
        )
        .pfn_user_callback(Some(debug_callback))
}

/// 根据实际安装情况过滤候选层；未安装的层会打印提示并跳过。
#[cfg(debug_assertions)]
pub(super) fn validation_layers(entry: &Entry) -> Result<Vec<&'static CStr>, vk::Result> {
    let available = unsafe { entry.enumerate_instance_layer_properties()? };

    let enabled: Vec<&'static CStr> = CANDIDATE_LAYERS
        .iter()
        .copied()
        .filter(|wanted| {
            available.iter().any(|layer| {
                layer
                    .layer_name_as_c_str()
                    .is_ok_and(|name| name == *wanted)
            })
        })
        .collect();

    for wanted in CANDIDATE_LAYERS {
        if !enabled.contains(wanted) {
            eprintln!("未安装 {wanted:?}，已跳过");
        }
    }

    Ok(enabled)
}

/// 发布构建不启用任何校验层。
#[cfg(not(debug_assertions))]
pub(super) fn validation_layers(_entry: &Entry) -> Result<Vec<&'static CStr>, vk::Result> {
    Ok(Vec::new())
}
