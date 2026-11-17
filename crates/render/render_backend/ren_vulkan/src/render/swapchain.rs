//! 交换链：表面能力查询、格式/呈现模式/范围选择、创建与图像获取。

use ash::vk;
use std::error::Error;

use super::device::find_queue_families;

/// 交换链及其派生信息（教程里的 swapchain / swapChainImages / swapChainImageFormat / swapChainExtent）。
pub(super) struct Swapchain {
    pub(super) loader: ash::khr::swapchain::Device,
    pub(super) handle: vk::SwapchainKHR,
    pub(super) images: Vec<vk::Image>,
    pub(super) format: vk::Format,
    pub(super) extent: vk::Extent2D,
}

/// 交换链支持详情（教程的 SwapChainSupportDetails）。
pub(super) struct SwapchainSupportDetails {
    capabilities: vk::SurfaceCapabilitiesKHR,
    pub(super) formats: Vec<vk::SurfaceFormatKHR>,
    pub(super) present_modes: Vec<vk::PresentModeKHR>,
}

/// 查询表面的能力、格式和呈现模式（教程的 querySwapChainSupport）。
pub(super) fn query_swapchain_support(
    physical_device: vk::PhysicalDevice,
    surface_loader: &ash::khr::surface::Instance,
    surface: vk::SurfaceKHR,
) -> Result<SwapchainSupportDetails, vk::Result> {
    let capabilities = unsafe {
        surface_loader.get_physical_device_surface_capabilities(physical_device, surface)?
    };
    let formats =
        unsafe { surface_loader.get_physical_device_surface_formats(physical_device, surface)? };
    let present_modes = unsafe {
        surface_loader.get_physical_device_surface_present_modes(physical_device, surface)?
    };

    Ok(SwapchainSupportDetails {
        capabilities,
        formats,
        present_modes,
    })
}

/// 选择表面格式：优先 B8G8R8A8_SRGB + SRGB_NONLINEAR，否则用第一个可用的
/// （教程的 chooseSwapSurfaceFormat）。
fn choose_surface_format(available: &[vk::SurfaceFormatKHR]) -> vk::SurfaceFormatKHR {
    available
        .iter()
        .copied()
        .find(|format| {
            format.format == vk::Format::B8G8R8A8_SRGB
                && format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
        })
        .unwrap_or(available[0])
}

/// 选择呈现模式：MAILBOX（三重缓冲、不限帧率）优先，否则 FIFO（规范保证支持）
/// （教程的 chooseSwapPresentMode）。
fn choose_present_mode(available: &[vk::PresentModeKHR]) -> vk::PresentModeKHR {
    if available.contains(&vk::PresentModeKHR::MAILBOX) {
        vk::PresentModeKHR::MAILBOX
    } else {
        vk::PresentModeKHR::FIFO
    }
}

/// 选择交换范围：表面决定 currentExtent 时直接用，否则用窗口尺寸夹在允许范围内
/// （教程的 chooseSwapExtent）。
fn choose_swap_extent(
    capabilities: &vk::SurfaceCapabilitiesKHR,
    width: u32,
    height: u32,
) -> vk::Extent2D {
    if capabilities.current_extent.width != u32::MAX {
        capabilities.current_extent
    } else {
        vk::Extent2D {
            width: width.clamp(
                capabilities.min_image_extent.width,
                capabilities.max_image_extent.width,
            ),
            height: height.clamp(
                capabilities.min_image_extent.height,
                capabilities.max_image_extent.height,
            ),
        }
    }
}

/// 交换链的创建参数。`old_swapchain` 在重建时传入旧交换链让驱动复用资源，
/// 首次创建传 `null`。
pub(super) struct SwapchainOptions {
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) old_swapchain: vk::SwapchainKHR,
}

/// 创建交换链并取回交换链图像（教程的 createSwapChain + vkGetSwapchainImagesKHR）。
pub(super) fn create_swapchain(
    instance: &ash::Instance,
    device: &ash::Device,
    surface_loader: &ash::khr::surface::Instance,
    surface: vk::SurfaceKHR,
    physical_device: vk::PhysicalDevice,
    options: SwapchainOptions,
) -> Result<Swapchain, Box<dyn Error + Send + Sync>> {
    let support = query_swapchain_support(physical_device, surface_loader, surface)?;
    let indices = find_queue_families(instance, surface_loader, surface, physical_device)?;

    let surface_format = choose_surface_format(&support.formats);
    let present_mode = choose_present_mode(&support.present_modes);
    let extent = choose_swap_extent(&support.capabilities, options.width, options.height);

    // 至少比 minImageCount 多一张，但不能超过上限（0 表示无上限）。
    let mut image_count = support.capabilities.min_image_count + 1;
    if support.capabilities.max_image_count > 0
        && image_count > support.capabilities.max_image_count
    {
        image_count = support.capabilities.max_image_count;
    }

    let graphics_family = indices.graphics_family.ok_or("缺少图形队列族")?;
    let present_family = indices.present_family.ok_or("缺少呈现队列族")?;

    // 图形族和呈现族不同时用并发模式，让两个族都能访问交换链图像。
    let (sharing_mode, queue_family_indices) = if graphics_family != present_family {
        (
            vk::SharingMode::CONCURRENT,
            vec![graphics_family, present_family],
        )
    } else {
        (vk::SharingMode::EXCLUSIVE, Vec::new())
    };

    let create_info = vk::SwapchainCreateInfoKHR::default()
        .surface(surface)
        .min_image_count(image_count)
        .image_format(surface_format.format)
        .image_color_space(surface_format.color_space)
        .image_extent(extent)
        .image_array_layers(1)
        .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
        .image_sharing_mode(sharing_mode)
        .queue_family_indices(&queue_family_indices)
        .pre_transform(support.capabilities.current_transform)
        .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
        .present_mode(present_mode)
        .clipped(true)
        .old_swapchain(options.old_swapchain);

    let loader = ash::khr::swapchain::Device::new(instance, device);
    let handle = unsafe { loader.create_swapchain(&create_info, None)? };

    let images = match unsafe { loader.get_swapchain_images(handle) } {
        Ok(images) => images,
        Err(err) => {
            unsafe { loader.destroy_swapchain(handle, None) };
            return Err(err.into());
        }
    };

    Ok(Swapchain {
        loader,
        handle,
        images,
        format: surface_format.format,
        extent,
    })
}
