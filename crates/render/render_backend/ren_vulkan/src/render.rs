//! Vulkan 渲染后端入口：持有实例、设备、交换链、图形管线与绘制资源，
//! 并保证正确的创建/销毁顺序。
//!
//! 按教程章节拆分为子模块：
//! - `debug`：校验层与调试信使；
//! - `instance`：`VkInstance` 创建；
//! - `surface`：窗口表面 `VkSurfaceKHR`；
//! - `device`：物理设备选择与逻辑设备；
//! - `swapchain`：交换链查询与创建；
//! - `image_view`：交换链图像的图像视图；
//! - `shader_module`：SPIR-V 着色器模块（「Shader modules」章节）；
//! - `render_pass`：渲染通道与附件（「Render passes」章节）；
//! - `pipeline`：固定功能状态与图形管线（「Fixed functions」章节）；
//! - `framebuffer`：把图像视图绑定到渲染通道（「Framebuffers」章节）；
//! - `command_buffer`：命令池、命令缓冲与录制（「Command buffers」章节）；
//! - `sync`：帧信号量/栅栏（「Frames in flight」章节）。

mod command_buffer;
mod debug;
mod device;
mod framebuffer;
mod image_view;
mod instance;
mod pipeline;
mod render_pass;
mod shader_module;
mod surface;
mod swapchain;
mod sync;

use ash::{Entry, vk};
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use std::error::Error;

use self::command_buffer::{allocate_command_buffers, create_command_pool, record_command_buffer};
use self::device::{LogicalDevice, create_logical_device, pick_physical_device};
use self::framebuffer::{create_framebuffers, destroy_framebuffers};
use self::image_view::create_swapchain_image_views;
use self::instance::create_instance;
use self::pipeline::{GraphicsPipeline, create_graphics_pipeline};
use self::render_pass::create_render_pass;
use self::surface::create_surface;
use self::swapchain::{Swapchain, SwapchainOptions, create_swapchain};
use self::sync::{FrameSync, MAX_FRAMES_IN_FLIGHT};

pub use self::shader_module::{ShaderProgram, ShaderStage};

pub struct Render {
    _entry: Entry,
    instance: ash::Instance,
    surface_loader: ash::khr::surface::Instance,
    surface: vk::SurfaceKHR,
    physical_device: vk::PhysicalDevice,
    device: ash::Device,
    graphics_queue: vk::Queue,
    present_queue: vk::Queue,
    swapchain: Swapchain,
    image_views: Vec<vk::ImageView>,
    render_pass: vk::RenderPass,
    graphics_pipeline: GraphicsPipeline,
    framebuffers: Vec<vk::Framebuffer>,
    command_pool: vk::CommandPool,
    command_buffers: Vec<vk::CommandBuffer>,
    sync: FrameSync,
    /// 最近一次窗口尺寸；交换链重建时使用。
    window_width: u32,
    window_height: u32,
    /// 窗口尺寸变化后由绘制循环触发交换链重建。
    framebuffer_resized: bool,
    #[cfg(debug_assertions)]
    debug_utils: Option<ash::ext::debug_utils::Instance>,
    #[cfg(debug_assertions)]
    debug_messenger: Option<vk::DebugUtilsMessengerEXT>,
}

/// 绘制阶段创建的资源，全部依赖渲染通道、交换链和图形队列族。
struct DrawingResources {
    framebuffers: Vec<vk::Framebuffer>,
    command_pool: vk::CommandPool,
    command_buffers: Vec<vk::CommandBuffer>,
    sync: FrameSync,
}

/// 创建帧缓冲、命令池/缓冲和帧同步对象（教程的 createFramebuffers +
/// createCommandPool + createCommandBuffers + createSyncObjects）。
/// 中途失败时回收本函数已创建的对象，外层的交换链等由调用者处理。
fn create_drawing_resources(
    device: &ash::Device,
    render_pass: vk::RenderPass,
    image_views: &[vk::ImageView],
    extent: vk::Extent2D,
    graphics_family: u32,
) -> Result<DrawingResources, Box<dyn Error + Send + Sync>> {
    let mut framebuffers = create_framebuffers(device, render_pass, image_views, extent)?;

    let command_pool = match create_command_pool(device, graphics_family) {
        Ok(command_pool) => command_pool,
        Err(err) => {
            destroy_framebuffers(device, &mut framebuffers);
            return Err(err.into());
        }
    };

    let command_buffers = match allocate_command_buffers(device, command_pool, MAX_FRAMES_IN_FLIGHT)
    {
        Ok(command_buffers) => command_buffers,
        Err(err) => {
            unsafe { device.destroy_command_pool(command_pool, None) };
            destroy_framebuffers(device, &mut framebuffers);
            return Err(err.into());
        }
    };

    let sync = match FrameSync::create(device, image_views.len()) {
        Ok(sync) => sync,
        Err(err) => {
            unsafe { device.destroy_command_pool(command_pool, None) };
            destroy_framebuffers(device, &mut framebuffers);
            return Err(err);
        }
    };

    Ok(DrawingResources {
        framebuffers,
        command_pool,
        command_buffers,
        sync,
    })
}

impl Render {
    pub fn new(
        display_handle: RawDisplayHandle,
        window_handle: RawWindowHandle,
        width: u32,
        height: u32,
        shaders: ShaderProgram<'_>,
    ) -> Result<Render, Box<dyn Error + Send + Sync>> {
        // 实例创建阶段（教程的 createInstance）：Entry、VkInstance，调试构建下还有调试信使。
        let instance_context = create_instance()?;
        let entry = instance_context.entry;
        let instance = instance_context.instance;
        #[cfg(debug_assertions)]
        let debug_utils = instance_context.debug_utils;
        #[cfg(debug_assertions)]
        let debug_messenger = instance_context.debug_messenger;

        // 创建窗口表面（教程的 createSurface）。ash-window 相当于 glfwCreateWindowSurface：
        // 根据原始窗口句柄自动选择平台相关扩展（Windows 下就是 VK_KHR_win32_surface）。
        let surface_loader = ash::khr::surface::Instance::new(&entry, &instance);

        // Render 尚未构造，Drop 不会执行；任何一步失败都按“逆创建顺序”手动回收。
        let cleanup = |device: Option<&ash::Device>,
                       swapchain_loader: Option<&ash::khr::swapchain::Device>,
                       swapchain: Option<vk::SwapchainKHR>,
                       image_views: &[vk::ImageView],
                       render_pass: Option<vk::RenderPass>,
                       pipeline: Option<&GraphicsPipeline>,
                       surface: Option<vk::SurfaceKHR>| {
            if let Some(device) = device {
                // 管线引用管线布局与渲染通道，先于二者销毁。
                if let Some(pipeline) = pipeline {
                    pipeline.destroy(device);
                }
                // 渲染通道先于交换链图像（以及它们的图像视图）销毁。
                if let Some(render_pass) = render_pass {
                    unsafe { device.destroy_render_pass(render_pass, None) };
                }
                for &image_view in image_views {
                    unsafe { device.destroy_image_view(image_view, None) };
                }
            }
            if let (Some(loader), Some(swapchain)) = (swapchain_loader, swapchain) {
                unsafe { loader.destroy_swapchain(swapchain, None) };
            }
            if let Some(device) = device {
                unsafe { device.destroy_device(None) };
            }
            #[cfg(debug_assertions)]
            if let Some(messenger) = debug_messenger {
                unsafe { debug_utils.destroy_debug_utils_messenger(messenger, None) };
            }
            if let Some(surface) = surface {
                unsafe { surface_loader.destroy_surface(surface, None) };
            }
            unsafe { instance.destroy_instance(None) };
        };

        let surface = match create_surface(&entry, &instance, display_handle, window_handle) {
            Ok(surface) => surface,
            Err(err) => {
                cleanup(None, None, None, &[], None, None, None);
                return Err(err);
            }
        };

        // 选择物理设备（教程的 pickPhysicalDevice）。
        let physical_device = match pick_physical_device(&instance, &surface_loader, surface) {
            Ok(device) => device,
            Err(err) => {
                cleanup(None, None, None, &[], None, None, Some(surface));
                return Err(err);
            }
        };

        // 创建逻辑设备并取回图形/呈现队列（教程的 createLogicalDevice）。
        let logical_device =
            match create_logical_device(&instance, &surface_loader, surface, physical_device) {
                Ok(logical_device) => logical_device,
                Err(err) => {
                    cleanup(None, None, None, &[], None, None, Some(surface));
                    return Err(err);
                }
            };
        let LogicalDevice {
            device,
            graphics_queue,
            present_queue,
            graphics_family,
        } = logical_device;

        // 创建交换链（教程的 createSwapChain）。
        let swapchain = match create_swapchain(
            &instance,
            &device,
            &surface_loader,
            surface,
            physical_device,
            SwapchainOptions {
                width,
                height,
                old_swapchain: vk::SwapchainKHR::null(),
            },
        ) {
            Ok(swapchain) => swapchain,
            Err(err) => {
                cleanup(Some(&device), None, None, &[], None, None, Some(surface));
                return Err(err);
            }
        };

        // 为交换链的每张图像创建图像视图（教程的 createImageViews）。
        let image_views =
            match create_swapchain_image_views(&device, &swapchain.images, swapchain.format) {
                Ok(image_views) => image_views,
                Err(err) => {
                    cleanup(
                        Some(&device),
                        Some(&swapchain.loader),
                        Some(swapchain.handle),
                        &[],
                        None,
                        None,
                        Some(surface),
                    );
                    return Err(err);
                }
            };

        // 创建渲染通道（教程的 createRenderPass）。附件格式必须与交换链一致。
        let render_pass = match create_render_pass(&device, swapchain.format) {
            Ok(render_pass) => render_pass,
            Err(err) => {
                cleanup(
                    Some(&device),
                    Some(&swapchain.loader),
                    Some(swapchain.handle),
                    &image_views,
                    None,
                    None,
                    Some(surface),
                );
                return Err(err.into());
            }
        };

        // 创建图形管线（教程的 createGraphicsPipeline）：着色器模块 + 固定功能状态。
        let graphics_pipeline = match create_graphics_pipeline(&device, render_pass, &shaders) {
            Ok(pipeline) => pipeline,
            Err(err) => {
                cleanup(
                    Some(&device),
                    Some(&swapchain.loader),
                    Some(swapchain.handle),
                    &image_views,
                    Some(render_pass),
                    None,
                    Some(surface),
                );
                return Err(err);
            }
        };

        // 创建帧缓冲、命令池/缓冲与帧同步对象。
        let drawing = match create_drawing_resources(
            &device,
            render_pass,
            &image_views,
            swapchain.extent,
            graphics_family,
        ) {
            Ok(drawing) => drawing,
            Err(err) => {
                cleanup(
                    Some(&device),
                    Some(&swapchain.loader),
                    Some(swapchain.handle),
                    &image_views,
                    Some(render_pass),
                    Some(&graphics_pipeline),
                    Some(surface),
                );
                return Err(err);
            }
        };

        let render = Self {
            _entry: entry,
            instance,
            surface_loader,
            surface,
            physical_device,
            device,
            graphics_queue,
            present_queue,
            swapchain,
            image_views,
            render_pass,
            graphics_pipeline,
            framebuffers: drawing.framebuffers,
            command_pool: drawing.command_pool,
            command_buffers: drawing.command_buffers,
            sync: drawing.sync,
            window_width: width,
            window_height: height,
            framebuffer_resized: false,
            #[cfg(debug_assertions)]
            debug_utils: Some(debug_utils),
            #[cfg(debug_assertions)]
            debug_messenger,
        };
        Ok(render)
    }

    /// 窗口尺寸变化时记录新尺寸，实际重建放在下一次绘制，避免连续 resize 时反复重建。
    pub fn resize(&mut self, width: u32, height: u32) {
        self.window_width = width;
        self.window_height = height;
        self.framebuffer_resized = true;
    }

    /// 绘制并呈现一帧（教程的 `drawFrame`）：
    /// 等栅栏 → 获取图像 → 录制 → 提交 → 呈现，必要时重建交换链。
    pub fn draw_frame(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        // 最小化时宽高为 0，交换链无法创建/呈现；等窗口恢复后再继续。
        if self.window_width == 0 || self.window_height == 0 {
            return Ok(());
        }

        // 窗口尺寸变化后，先重建交换链再开始这一帧。
        if self.framebuffer_resized {
            self.framebuffer_resized = false;
            self.recreate_swapchain()?;
        }

        let frame = self.sync.current_frame;
        let in_flight_fence = self.sync.in_flight_fence(frame);

        // 等这一帧上一次提交完成，才能安全重置命令缓冲和栅栏。
        unsafe {
            self.device
                .wait_for_fences(&[in_flight_fence], true, u64::MAX)?
        };

        // 获取下一张可用的交换链图像；成功后 image_available 信号量会被发出。
        let (image_index, acquire_suboptimal) = match unsafe {
            self.swapchain.loader.acquire_next_image(
                self.swapchain.handle,
                u64::MAX,
                self.sync.image_available(frame),
                vk::Fence::null(),
            )
        } {
            Ok(result) => result,
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                self.recreate_swapchain()?;
                return Ok(());
            }
            Err(err) => return Err(err.into()),
        };
        let image_index = image_index as usize;

        // 呈现顺序不一定与获取顺序一致：这张图像可能还被更早的帧占用。
        let image_fence = self.sync.images_in_flight(image_index);
        if image_fence != vk::Fence::null() {
            unsafe {
                self.device
                    .wait_for_fences(&[image_fence], true, u64::MAX)?
            };
        }
        self.sync.set_image_in_flight(image_index, in_flight_fence);

        // 录制这一帧的命令缓冲。
        let command_buffer = self.command_buffers[frame];
        unsafe {
            self.device
                .reset_command_buffer(command_buffer, vk::CommandBufferResetFlags::empty())?
        };
        record_command_buffer(
            &self.device,
            command_buffer,
            self.render_pass,
            self.framebuffers[image_index],
            self.graphics_pipeline.handle,
            self.swapchain.extent,
        )?;

        // 录制成功后才重置栅栏：若前面的步骤失败，栅栏保持已发信号，下一帧不会死等。
        unsafe { self.device.reset_fences(&[in_flight_fence])? };

        let wait_semaphores = [self.sync.image_available(frame)];
        let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let signal_semaphores = [self.sync.render_finished(image_index)];
        let command_buffers = [command_buffer];
        let submit_info = vk::SubmitInfo::default()
            .wait_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&wait_stages)
            .command_buffers(&command_buffers)
            .signal_semaphores(&signal_semaphores);
        unsafe {
            self.device
                .queue_submit(self.graphics_queue, &[submit_info], in_flight_fence)?
        };

        let swapchains = [self.swapchain.handle];
        let image_indices = [image_index as u32];
        let present_info = vk::PresentInfoKHR::default()
            .wait_semaphores(&signal_semaphores)
            .swapchains(&swapchains)
            .image_indices(&image_indices);
        let present_result = unsafe {
            self.swapchain
                .loader
                .queue_present(self.present_queue, &present_info)
        };

        let recreate = match present_result {
            Ok(suboptimal) => suboptimal || acquire_suboptimal || self.framebuffer_resized,
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => true,
            Err(err) => return Err(err.into()),
        };

        self.sync.advance_frame();

        if recreate {
            self.framebuffer_resized = false;
            self.recreate_swapchain()?;
        }

        Ok(())
    }

    /// 重建交换链及其图像视图/帧缓冲（教程的 `recreateSwapChain`）。
    ///
    /// 先创建新资源、成功后再销毁旧资源，这样即使重建失败，旧交换链仍然可用。
    /// 渲染通道与图形管线按教程保持不变，因此要求交换链格式稳定。
    fn recreate_swapchain(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        if self.window_width == 0 || self.window_height == 0 {
            return Ok(());
        }

        // 旧交换链的图像可能仍在使用，先等 GPU 空闲。
        unsafe { self.device.device_wait_idle()? };

        let new_swapchain = create_swapchain(
            &self.instance,
            &self.device,
            &self.surface_loader,
            self.surface,
            self.physical_device,
            SwapchainOptions {
                width: self.window_width,
                height: self.window_height,
                old_swapchain: self.swapchain.handle,
            },
        )?;

        // 渲染通道和管线是按旧格式创建的，格式变化时无法直接复用。
        if new_swapchain.format != self.swapchain.format {
            unsafe {
                new_swapchain
                    .loader
                    .destroy_swapchain(new_swapchain.handle, None)
            };
            return Err("交换链格式变化，无法复用现有渲染通道和图形管线".into());
        }

        let new_image_views = match create_swapchain_image_views(
            &self.device,
            &new_swapchain.images,
            new_swapchain.format,
        ) {
            Ok(image_views) => image_views,
            Err(err) => {
                unsafe {
                    new_swapchain
                        .loader
                        .destroy_swapchain(new_swapchain.handle, None)
                };
                return Err(err);
            }
        };

        let new_framebuffers = match create_framebuffers(
            &self.device,
            self.render_pass,
            &new_image_views,
            new_swapchain.extent,
        ) {
            Ok(framebuffers) => framebuffers,
            Err(err) => {
                for image_view in new_image_views {
                    unsafe { self.device.destroy_image_view(image_view, None) };
                }
                unsafe {
                    new_swapchain
                        .loader
                        .destroy_swapchain(new_swapchain.handle, None)
                };
                return Err(err);
            }
        };

        // 新资源全部就绪，销毁旧交换链及其派生对象（教程的 cleanupSwapChain）。
        self.cleanup_swapchain();

        self.swapchain = new_swapchain;
        self.image_views = new_image_views;
        self.framebuffers = new_framebuffers;

        // 旧交换链已销毁，换一批与新图像数量一致的呈现信号量并清空占用表。
        self.sync
            .recreate_image_semaphores(&self.device, self.swapchain.images.len())?;

        Ok(())
    }

    /// 销毁交换链及其图像视图、帧缓冲（教程的 `cleanupSwapChain`）。
    /// 重建交换链与最终析构共用；调用前必须确保 GPU 已经空闲。
    fn cleanup_swapchain(&mut self) {
        destroy_framebuffers(&self.device, &mut self.framebuffers);

        for image_view in self.image_views.drain(..) {
            unsafe { self.device.destroy_image_view(image_view, None) };
        }

        unsafe {
            self.swapchain
                .loader
                .destroy_swapchain(self.swapchain.handle, None)
        };
    }
}

impl Drop for Render {
    fn drop(&mut self) {
        // 销毁任何对象前先确认 GPU 已经用完它们（教程 cleanup 里的 vkDeviceWaitIdle）。
        let _ = unsafe { self.device.device_wait_idle() };

        // 同步对象不再被任何提交使用。
        self.sync.destroy(&self.device);

        // 命令缓冲随命令池一起销毁。
        unsafe { self.device.destroy_command_pool(self.command_pool, None) };

        // 管线引用管线布局与渲染通道，必须先于二者销毁。
        self.graphics_pipeline.destroy(&self.device);
        unsafe { self.device.destroy_render_pass(self.render_pass, None) };

        // 交换链及其图像视图、帧缓冲（与重建共用的 cleanupSwapChain）。
        self.cleanup_swapchain();

        // 逻辑设备先于实例销毁（教程 cleanup 的顺序）。
        unsafe { self.device.destroy_device(None) };

        // 调试信使必须先于实例销毁（教程的测试错误就是漏掉这一步）。
        #[cfg(debug_assertions)]
        if let (Some(debug_utils), Some(messenger)) =
            (self.debug_utils.take(), self.debug_messenger.take())
        {
            unsafe { debug_utils.destroy_debug_utils_messenger(messenger, None) };
        }

        // 表面也必须在实例之前销毁。
        unsafe { self.surface_loader.destroy_surface(self.surface, None) };

        unsafe { self.instance.destroy_instance(None) };
    }
}
