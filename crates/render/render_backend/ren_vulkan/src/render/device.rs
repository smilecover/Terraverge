//! 物理设备选择与逻辑设备创建：队列族、扩展支持、设备评分与队列句柄。

use ash::vk;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;

use super::swapchain::query_swapchain_support;

/// 队列族索引：用 `Option` 表示“不存在”，因为 0 本身也是合法的队列族索引。
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct QueueFamilyIndices {
    pub(super) graphics_family: Option<u32>,
    pub(super) present_family: Option<u32>,
}

impl QueueFamilyIndices {
    fn is_complete(&self) -> bool {
        self.graphics_family.is_some() && self.present_family.is_some()
    }
}

/// 查询设备支持的所有队列族，找出图形族和呈现族（教程的 findQueueFamilies）。
/// 两者可能不是同一个队列族，所以分开存。
pub(super) fn find_queue_families(
    instance: &ash::Instance,
    surface_loader: &ash::khr::surface::Instance,
    surface: vk::SurfaceKHR,
    device: vk::PhysicalDevice,
) -> Result<QueueFamilyIndices, vk::Result> {
    let families = unsafe { instance.get_physical_device_queue_family_properties(device) };

    let mut indices = QueueFamilyIndices::default();
    for (index, family) in families.iter().enumerate() {
        let index = index as u32;

        if family.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
            indices.graphics_family = Some(index);
        }

        // 这个队列族能否向我们的窗口表面呈现图像？
        let present_support =
            unsafe { surface_loader.get_physical_device_surface_support(device, index, surface)? };
        if present_support {
            indices.present_family = Some(index);
        }

        if indices.is_complete() {
            break;
        }
    }
    Ok(indices)
}

/// 设备适用性检查：队列族完整、支持交换链扩展、交换链格式/呈现模式非空
/// （教程的 isDeviceSuitable）。
fn is_device_suitable(
    instance: &ash::Instance,
    surface_loader: &ash::khr::surface::Instance,
    surface: vk::SurfaceKHR,
    device: vk::PhysicalDevice,
) -> Result<bool, vk::Result> {
    let indices = find_queue_families(instance, surface_loader, surface, device)?;
    if !indices.is_complete() {
        return Ok(false);
    }

    // 设备必须支持 VK_KHR_swapchain。
    if !check_device_extension_support(instance, device)? {
        return Ok(false);
    }

    // 交换链支持不能为空：至少要有一种格式和一种呈现模式。
    let support = query_swapchain_support(device, surface_loader, surface)?;
    Ok(!support.formats.is_empty() && !support.present_modes.is_empty())
}

/// 逻辑设备及其队列。`graphics_family` 在创建命令池等按队列族归属的对象时还要用到。
pub(super) struct LogicalDevice {
    pub(super) device: ash::Device,
    pub(super) graphics_queue: vk::Queue,
    pub(super) present_queue: vk::Queue,
    pub(super) graphics_family: u32,
}

/// 创建逻辑设备，并取回图形/呈现队列句柄（教程的 createLogicalDevice + vkGetDeviceQueue）。
pub(super) fn create_logical_device(
    instance: &ash::Instance,
    surface_loader: &ash::khr::surface::Instance,
    surface: vk::SurfaceKHR,
    physical_device: vk::PhysicalDevice,
) -> Result<LogicalDevice, Box<dyn Error + Send + Sync>> {
    let indices = find_queue_families(instance, surface_loader, surface, physical_device)?;
    let graphics_family = indices
        .graphics_family
        .ok_or("选中的物理设备没有图形队列族")?;
    let present_family = indices
        .present_family
        .ok_or("选中的物理设备没有呈现队列族")?;

    // 教程用 std::set 去重：图形族和呈现族相同时只创建一个队列。
    let unique_families = BTreeSet::from([graphics_family, present_family]);

    // 队列优先级必须在 0.0~1.0 之间；即使只创建一个队列也要指定。
    let queue_priorities = [1.0f32];
    let queue_create_infos: Vec<_> = unique_families
        .iter()
        .map(|&family| {
            vk::DeviceQueueCreateInfo::default()
                .queue_family_index(family)
                .queue_priorities(&queue_priorities)
        })
        .collect();

    // 目前不需要任何特殊功能，全部保持默认（VK_FALSE）。
    let device_features = vk::PhysicalDeviceFeatures::default();

    // 设备级扩展：交换链是必需的（教程的 deviceExtensions）。
    // 某些移植层/驱动（MoltenVK、部分校验层模拟的设备）还会报告
    // VK_KHR_portability_subset；规范要求只要支持就必须显式启用。
    let mut device_extension_names = vec![ash::khr::swapchain::NAME.as_ptr()];
    let available_extensions =
        unsafe { instance.enumerate_device_extension_properties(physical_device)? };
    if available_extensions.iter().any(|extension| {
        extension
            .extension_name_as_c_str()
            .is_ok_and(|name| name == ash::khr::portability_subset::NAME)
    }) {
        device_extension_names.push(ash::khr::portability_subset::NAME.as_ptr());
    }

    let create_info = vk::DeviceCreateInfo::default()
        .queue_create_infos(&queue_create_infos)
        .enabled_features(&device_features)
        .enabled_extension_names(&device_extension_names);

    let device = unsafe { instance.create_device(physical_device, &create_info, None)? };

    // 队列随逻辑设备自动创建，各取该族的第 0 个。
    let graphics_queue = unsafe { device.get_device_queue(graphics_family, 0) };
    let present_queue = unsafe { device.get_device_queue(present_family, 0) };

    Ok(LogicalDevice {
        device,
        graphics_queue,
        present_queue,
        graphics_family,
    })
}

/// 设备是否支持 VK_KHR_swapchain（教程的 checkDeviceExtensionSupport）。
fn check_device_extension_support(
    instance: &ash::Instance,
    device: vk::PhysicalDevice,
) -> Result<bool, vk::Result> {
    let available = unsafe { instance.enumerate_device_extension_properties(device)? };
    Ok(available.iter().any(|extension| {
        extension
            .extension_name_as_c_str()
            .is_ok_and(|name| name == ash::khr::swapchain::NAME)
    }))
}

/// 设备评分（教程的 rateDeviceSuitability）：分数越高越优先，0 表示不满足硬性条件。
fn rate_device_suitability(instance: &ash::Instance, device: vk::PhysicalDevice) -> u32 {
    let props = unsafe { instance.get_physical_device_properties(device) };
    let features = unsafe { instance.get_physical_device_features(device) };

    // 教程示例：应用没有几何着色器就跑不了。
    if features.geometry_shader == vk::FALSE {
        return 0;
    }

    let mut score = 0;

    // 独立显卡有明显性能优势。
    if props.device_type == vk::PhysicalDeviceType::DISCRETE_GPU {
        score += 1000;
    }

    // 纹理最大尺寸会影响画质。
    score += props.limits.max_image_dimension2_d;

    score
}

/// 选择得分最高的显卡（教程的 pickPhysicalDevice 评分方案）。
pub(super) fn pick_physical_device(
    instance: &ash::Instance,
    surface_loader: &ash::khr::surface::Instance,
    surface: vk::SurfaceKHR,
) -> Result<vk::PhysicalDevice, Box<dyn Error + Send + Sync>> {
    let devices = unsafe { instance.enumerate_physical_devices()? };
    if devices.is_empty() {
        return Err("没有可用的 Vulkan 物理设备（显卡驱动不支持？）".into());
    }

    // 教程用 std::multimap 按分数自动排序后取最高分；BTreeMap 是等价做法。
    let mut candidates = BTreeMap::new();

    for device in devices {
        let props = unsafe { instance.get_physical_device_properties(device) };
        let name = props.device_name_as_c_str()?;
        let score = rate_device_suitability(instance, device);
        eprintln!(
            "发现设备：{name:?}，类型：{:?}，评分：{score}",
            props.device_type
        );

        // 评分大于 0 且队列族满足要求，才进入候选。
        if score > 0 && is_device_suitable(instance, surface_loader, surface, device)? {
            candidates.insert(score, device);
        }
    }

    // 取分数最高的一台；一台都不满足则报错。
    candidates
        .pop_last()
        .map(|(_, device)| device)
        .ok_or_else(|| "没有满足需求的 GPU（需要图形/呈现队列族和几何着色器）".into())
}
