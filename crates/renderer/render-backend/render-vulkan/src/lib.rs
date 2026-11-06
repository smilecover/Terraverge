use std::ffi::{CStr, c_char, c_void};

use anyhow::{Context, Result, anyhow, bail};
use ash::vk;
use ash::vk::Handle;
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};

mod shaders {
    include!(concat!(env!("OUT_DIR"), "/triangle_shaders.rs"));
}

struct DebugUtils {
    loader: ash::ext::debug_utils::Instance,
    messenger: vk::DebugUtilsMessengerEXT,
}

pub struct VulkanRenderer {
    _entry: ash::Entry,
    instance: ash::Instance,
    debug_utils: Option<DebugUtils>,
    surface_loader: ash::khr::surface::Instance,
    surface: vk::SurfaceKHR,
    physical_device: vk::PhysicalDevice,
    device: ash::Device,
    queue: vk::Queue,
    queue_family_index: u32,
    swapchain_loader: ash::khr::swapchain::Device,
    window_size: vk::Extent2D,
    swapchain: vk::SwapchainKHR,
    surface_format: vk::SurfaceFormatKHR,
    swapchain_extent: vk::Extent2D,
    swapchain_image_views: Vec<vk::ImageView>,
    render_pass: vk::RenderPass,
    pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
    framebuffers: Vec<vk::Framebuffer>,
    command_pool: vk::CommandPool,
    command_buffers: Vec<vk::CommandBuffer>,
    image_available: vk::Semaphore,
    render_finished: vk::Semaphore,
    in_flight: vk::Fence,
    framebuffer_resized: bool,
}

impl VulkanRenderer {
    pub fn new(
        display_handle: RawDisplayHandle,
        window_handle: RawWindowHandle,
        width: u32,
        height: u32,
    ) -> Result<Self> {
        let entry = unsafe { ash::Entry::load() }
            .map_err(|err| anyhow!("failed to load the Vulkan loader: {err}"))?;

        let layer_properties = unsafe { entry.enumerate_instance_layer_properties() }?;
        let validation_available = layer_properties.iter().any(|layer| {
            let name = unsafe { CStr::from_ptr(layer.layer_name.as_ptr()) };
            name == c"VK_LAYER_KHRONOS_validation"
        });
        let validation_enabled = cfg!(debug_assertions) && validation_available;

        if cfg!(debug_assertions) && !validation_available {
            eprintln!("Vulkan validation layer is not installed");
        }

        let mut extension_names: Vec<*const c_char> =
            ash_window::enumerate_required_extensions(display_handle)
                .map_err(|err| anyhow!("failed to enumerate surface extensions: {err}"))?
                .to_vec();

        let layer_names: Vec<*const c_char> = if validation_enabled {
            extension_names.push(ash::ext::debug_utils::NAME.as_ptr());
            vec![c"VK_LAYER_KHRONOS_validation".as_ptr()]
        } else {
            Vec::new()
        };

        let application_info = vk::ApplicationInfo::default()
            .application_name(c"Terraverge")
            .application_version(vk::make_api_version(0, 0, 1, 0))
            .engine_name(c"Terraverge")
            .engine_version(vk::make_api_version(0, 0, 1, 0))
            .api_version(vk::make_api_version(0, 1, 0, 0));

        let mut debug_create_info = vk::DebugUtilsMessengerCreateInfoEXT::default()
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                    | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | vk::DebugUtilsMessageSeverityFlagsEXT::INFO,
            )
            .message_type(
                vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                    | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
            )
            .pfn_user_callback(Some(vulkan_debug_callback));

        let mut instance_create_info = vk::InstanceCreateInfo::default()
            .application_info(&application_info)
            .enabled_layer_names(&layer_names)
            .enabled_extension_names(&extension_names);

        if validation_enabled {
            instance_create_info = instance_create_info.push_next(&mut debug_create_info);
        }

        let instance = unsafe { entry.create_instance(&instance_create_info, None) }
            .context("failed to create the Vulkan instance")?;

        let debug_utils = if validation_enabled {
            let loader = ash::ext::debug_utils::Instance::new(&entry, &instance);
            let messenger =
                unsafe { loader.create_debug_utils_messenger(&debug_create_info, None) }
                    .context("failed to create the Vulkan debug messenger")?;
            Some(DebugUtils { loader, messenger })
        } else {
            None
        };

        let surface = unsafe {
            ash_window::create_surface(&entry, &instance, display_handle, window_handle, None)
        }
        .context("failed to create the Vulkan surface")?;
        let surface_loader = ash::khr::surface::Instance::new(&entry, &instance);

        let physical_devices = unsafe { instance.enumerate_physical_devices() }?;
        let (physical_device, queue_family_index) = physical_devices
            .iter()
            .find_map(|&candidate| {
                let extensions =
                    unsafe { instance.enumerate_device_extension_properties(candidate) }.ok()?;
                let supports_swapchain = extensions.iter().any(|extension| {
                    let name = unsafe { CStr::from_ptr(extension.extension_name.as_ptr()) };
                    name == ash::khr::swapchain::NAME
                });
                if !supports_swapchain {
                    return None;
                }

                let families =
                    unsafe { instance.get_physical_device_queue_family_properties(candidate) };
                families.iter().enumerate().find_map(|(index, family)| {
                    let graphics = family.queue_flags.contains(vk::QueueFlags::GRAPHICS);
                    let present = unsafe {
                        surface_loader.get_physical_device_surface_support(
                            candidate,
                            index as u32,
                            surface,
                        )
                    }
                    .unwrap_or(false);

                    (graphics && present).then_some((candidate, index as u32))
                })
            })
            .context("no Vulkan device supports both graphics and presentation")?;

        let device_properties = unsafe { instance.get_physical_device_properties(physical_device) };
        let device_name = unsafe { CStr::from_ptr(device_properties.device_name.as_ptr()) }
            .to_string_lossy()
            .into_owned();
        println!("Vulkan device: {device_name}");

        let queue_priorities = [1.0f32];
        let queue_create_info = vk::DeviceQueueCreateInfo::default()
            .queue_family_index(queue_family_index)
            .queue_priorities(&queue_priorities);
        let device_extension_names = [ash::khr::swapchain::NAME.as_ptr()];
        let features = vk::PhysicalDeviceFeatures::default();
        let device_create_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(std::slice::from_ref(&queue_create_info))
            .enabled_extension_names(&device_extension_names)
            .enabled_features(&features);

        let device = unsafe { instance.create_device(physical_device, &device_create_info, None) }
            .context("failed to create the Vulkan device")?;
        let queue = unsafe { device.get_device_queue(queue_family_index, 0) };
        let swapchain_loader = ash::khr::swapchain::Device::new(&instance, &device);
        let surface_format = query_surface_format(&surface_loader, physical_device, surface, None)?;

        let mut renderer = Self {
            _entry: entry,
            instance,
            debug_utils,
            surface_loader,
            surface,
            physical_device,
            device,
            queue,
            queue_family_index,
            swapchain_loader,
            window_size: vk::Extent2D { width, height },
            swapchain: vk::SwapchainKHR::null(),
            surface_format,
            swapchain_extent: vk::Extent2D::default(),
            swapchain_image_views: Vec::new(),
            render_pass: vk::RenderPass::null(),
            pipeline_layout: vk::PipelineLayout::null(),
            pipeline: vk::Pipeline::null(),
            framebuffers: Vec::new(),
            command_pool: vk::CommandPool::null(),
            command_buffers: Vec::new(),
            image_available: vk::Semaphore::null(),
            render_finished: vk::Semaphore::null(),
            in_flight: vk::Fence::null(),
            framebuffer_resized: false,
        };

        renderer.create_swapchain()?;
        renderer.create_render_pass()?;
        renderer.create_framebuffers()?;
        renderer.create_pipeline()?;
        renderer.create_command_pool()?;
        renderer.create_sync_objects()?;

        Ok(renderer)
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.window_size = vk::Extent2D { width, height };
        self.framebuffer_resized = true;
    }

    pub fn render(&mut self) -> Result<()> {
        if self.framebuffer_resized {
            if self.window_size.width == 0 || self.window_size.height == 0 {
                return Ok(());
            }
            self.recreate_swapchain()?;
        }

        unsafe {
            self.device
                .wait_for_fences(&[self.in_flight], true, u64::MAX)?;
        }

        let image_index = match unsafe {
            self.swapchain_loader.acquire_next_image(
                self.swapchain,
                u64::MAX,
                self.image_available,
                vk::Fence::null(),
            )
        } {
            Ok((index, suboptimal)) => {
                if suboptimal {
                    self.framebuffer_resized = true;
                }
                index
            }
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                self.recreate_swapchain()?;
                return Ok(());
            }
            Err(err) => return Err(err).context("failed to acquire a swapchain image"),
        };

        unsafe {
            self.device.reset_fences(&[self.in_flight])?;
        }

        self.record_command_buffer(image_index as usize)?;

        let wait_semaphores = [self.image_available];
        let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let signal_semaphores = [self.render_finished];
        let command_buffers = [self.command_buffers[image_index as usize]];
        let submit_info = vk::SubmitInfo::default()
            .wait_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&wait_stages)
            .command_buffers(&command_buffers)
            .signal_semaphores(&signal_semaphores);

        unsafe {
            self.device
                .queue_submit(self.queue, &[submit_info], self.in_flight)?;
        }

        let swapchains = [self.swapchain];
        let image_indices = [image_index];
        let present_info = vk::PresentInfoKHR::default()
            .wait_semaphores(&signal_semaphores)
            .swapchains(&swapchains)
            .image_indices(&image_indices);

        match unsafe {
            self.swapchain_loader
                .queue_present(self.queue, &present_info)
        } {
            Ok(suboptimal) => {
                if suboptimal || self.framebuffer_resized {
                    self.recreate_swapchain()?;
                }
            }
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => self.recreate_swapchain()?,
            Err(err) => return Err(err).context("failed to present the swapchain image"),
        }

        Ok(())
    }

    fn create_swapchain(&mut self) -> Result<()> {
        let capabilities = unsafe {
            self.surface_loader
                .get_physical_device_surface_capabilities(self.physical_device, self.surface)
        }?;
        let surface_format = query_surface_format(
            &self.surface_loader,
            self.physical_device,
            self.surface,
            Some(self.surface_format),
        )?;
        let present_modes = unsafe {
            self.surface_loader
                .get_physical_device_surface_present_modes(self.physical_device, self.surface)
        }?;
        let present_mode = if present_modes.contains(&vk::PresentModeKHR::MAILBOX) {
            vk::PresentModeKHR::MAILBOX
        } else {
            vk::PresentModeKHR::FIFO
        };

        let size = self.window_size;
        let extent = if capabilities.current_extent.width != u32::MAX {
            vk::Extent2D {
                width: capabilities.current_extent.width.max(1),
                height: capabilities.current_extent.height.max(1),
            }
        } else {
            vk::Extent2D {
                width: size.width.max(1),
                height: size.height.max(1),
            }
        };

        let image_count = {
            let mut count = capabilities.min_image_count + 1;
            if capabilities.max_image_count > 0 {
                count = count.min(capabilities.max_image_count);
            }
            count
        };

        let composite_alpha = if capabilities
            .supported_composite_alpha
            .contains(vk::CompositeAlphaFlagsKHR::OPAQUE)
        {
            vk::CompositeAlphaFlagsKHR::OPAQUE
        } else if capabilities
            .supported_composite_alpha
            .contains(vk::CompositeAlphaFlagsKHR::PRE_MULTIPLIED)
        {
            vk::CompositeAlphaFlagsKHR::PRE_MULTIPLIED
        } else {
            vk::CompositeAlphaFlagsKHR::POST_MULTIPLIED
        };

        let pre_transform = if capabilities
            .supported_transforms
            .contains(vk::SurfaceTransformFlagsKHR::IDENTITY)
        {
            vk::SurfaceTransformFlagsKHR::IDENTITY
        } else {
            capabilities.current_transform
        };

        let create_info = vk::SwapchainCreateInfoKHR::default()
            .surface(self.surface)
            .min_image_count(image_count)
            .image_format(surface_format.format)
            .image_color_space(surface_format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(pre_transform)
            .composite_alpha(composite_alpha)
            .present_mode(present_mode)
            .clipped(true)
            .old_swapchain(vk::SwapchainKHR::null());

        let swapchain = unsafe { self.swapchain_loader.create_swapchain(&create_info, None) }
            .context("failed to create the swapchain")?;
        let images = unsafe { self.swapchain_loader.get_swapchain_images(swapchain) }?;

        let image_views = images
            .iter()
            .map(|&image| {
                let create_info = vk::ImageViewCreateInfo::default()
                    .image(image)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(surface_format.format)
                    .subresource_range(
                        vk::ImageSubresourceRange::default()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .base_mip_level(0)
                            .level_count(1)
                            .base_array_layer(0)
                            .layer_count(1),
                    );

                unsafe { self.device.create_image_view(&create_info, None) }
                    .context("failed to create a swapchain image view")
            })
            .collect::<Result<Vec<_>>>()?;

        self.swapchain = swapchain;
        self.surface_format = surface_format;
        self.swapchain_extent = extent;
        self.swapchain_image_views = image_views;

        Ok(())
    }

    fn create_render_pass(&mut self) -> Result<()> {
        let attachment = vk::AttachmentDescription::default()
            .format(self.surface_format.format)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR);

        let color_attachment = vk::AttachmentReference::default()
            .attachment(0)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

        let color_attachments = [color_attachment];
        let subpass = vk::SubpassDescription::default()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(&color_attachments);

        let dependency = vk::SubpassDependency::default()
            .src_subpass(vk::SUBPASS_EXTERNAL)
            .dst_subpass(0)
            .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .src_access_mask(vk::AccessFlags::empty())
            .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE);

        let attachments = [attachment];
        let subpasses = [subpass];
        let dependencies = [dependency];
        let create_info = vk::RenderPassCreateInfo::default()
            .attachments(&attachments)
            .subpasses(&subpasses)
            .dependencies(&dependencies);

        self.render_pass = unsafe { self.device.create_render_pass(&create_info, None) }
            .context("failed to create the render pass")?;

        Ok(())
    }

    fn create_framebuffers(&mut self) -> Result<()> {
        self.destroy_framebuffers();

        self.framebuffers = self
            .swapchain_image_views
            .iter()
            .map(|&view| {
                let attachments = [view];
                let create_info = vk::FramebufferCreateInfo::default()
                    .render_pass(self.render_pass)
                    .attachments(&attachments)
                    .width(self.swapchain_extent.width)
                    .height(self.swapchain_extent.height)
                    .layers(1);

                unsafe { self.device.create_framebuffer(&create_info, None) }
                    .context("failed to create a framebuffer")
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(())
    }

    fn create_pipeline(&mut self) -> Result<()> {
        let vertex_module = create_shader_module(&self.device, shaders::VERTEX_SPV)?;
        let fragment_module = create_shader_module(&self.device, shaders::FRAGMENT_SPV)?;

        let result = self.create_pipeline_inner(vertex_module, fragment_module);

        unsafe {
            self.device.destroy_shader_module(vertex_module, None);
            self.device.destroy_shader_module(fragment_module, None);
        }

        result
    }

    fn create_pipeline_inner(
        &mut self,
        vertex_module: vk::ShaderModule,
        fragment_module: vk::ShaderModule,
    ) -> Result<()> {
        let stages = [
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(vertex_module)
                .name(c"vs_main"),
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(fragment_module)
                .name(c"fs_main"),
        ];

        let vertex_input = vk::PipelineVertexInputStateCreateInfo::default();
        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);
        let viewport_state = vk::PipelineViewportStateCreateInfo::default()
            .viewport_count(1)
            .scissor_count(1);
        let rasterization = vk::PipelineRasterizationStateCreateInfo::default()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .cull_mode(vk::CullModeFlags::NONE)
            .front_face(vk::FrontFace::CLOCKWISE)
            .depth_bias_enable(false)
            .line_width(1.0);
        let multisampling = vk::PipelineMultisampleStateCreateInfo::default()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);
        let color_blend_attachment = vk::PipelineColorBlendAttachmentState {
            blend_enable: vk::FALSE,
            src_color_blend_factor: vk::BlendFactor::ONE,
            dst_color_blend_factor: vk::BlendFactor::ZERO,
            color_blend_op: vk::BlendOp::ADD,
            src_alpha_blend_factor: vk::BlendFactor::ONE,
            dst_alpha_blend_factor: vk::BlendFactor::ZERO,
            alpha_blend_op: vk::BlendOp::ADD,
            color_write_mask: vk::ColorComponentFlags::RGBA,
        };
        let color_blend_attachments = [color_blend_attachment];
        let color_blending = vk::PipelineColorBlendStateCreateInfo::default()
            .logic_op_enable(false)
            .attachments(&color_blend_attachments);
        let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state =
            vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);

        let layout_info = vk::PipelineLayoutCreateInfo::default();
        let pipeline_layout = unsafe { self.device.create_pipeline_layout(&layout_info, None) }
            .context("failed to create the pipeline layout")?;

        let pipeline_info = vk::GraphicsPipelineCreateInfo::default()
            .stages(&stages)
            .vertex_input_state(&vertex_input)
            .input_assembly_state(&input_assembly)
            .viewport_state(&viewport_state)
            .rasterization_state(&rasterization)
            .multisample_state(&multisampling)
            .color_blend_state(&color_blending)
            .dynamic_state(&dynamic_state)
            .layout(pipeline_layout)
            .render_pass(self.render_pass)
            .subpass(0);

        let pipelines = unsafe {
            self.device
                .create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_info], None)
        }
        .map_err(|(_, err)| anyhow!("failed to create the graphics pipeline: {err}"))?;

        self.pipeline_layout = pipeline_layout;
        self.pipeline = pipelines[0];

        Ok(())
    }

    fn create_command_pool(&mut self) -> Result<()> {
        let create_info = vk::CommandPoolCreateInfo::default()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(self.queue_family_index);

        self.command_pool = unsafe { self.device.create_command_pool(&create_info, None) }
            .context("failed to create the command pool")?;
        self.allocate_command_buffers()
    }

    fn allocate_command_buffers(&mut self) -> Result<()> {
        let create_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(self.command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(self.framebuffers.len() as u32);

        self.command_buffers = unsafe { self.device.allocate_command_buffers(&create_info) }
            .context("failed to allocate command buffers")?;

        Ok(())
    }

    fn create_sync_objects(&mut self) -> Result<()> {
        let semaphore_info = vk::SemaphoreCreateInfo::default();
        let fence_info = vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);

        self.image_available = unsafe { self.device.create_semaphore(&semaphore_info, None) }
            .context("failed to create the image semaphore")?;
        self.render_finished = unsafe { self.device.create_semaphore(&semaphore_info, None) }
            .context("failed to create the render semaphore")?;
        self.in_flight = unsafe { self.device.create_fence(&fence_info, None) }
            .context("failed to create the in-flight fence")?;

        Ok(())
    }

    fn record_command_buffer(&self, image_index: usize) -> Result<()> {
        let command_buffer = self.command_buffers[image_index];

        unsafe {
            self.device
                .reset_command_buffer(command_buffer, vk::CommandBufferResetFlags::empty())?;

            let begin_info = vk::CommandBufferBeginInfo::default();
            self.device
                .begin_command_buffer(command_buffer, &begin_info)?;

            let clear_values = [vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.02, 0.02, 0.05, 1.0],
                },
            }];
            let render_area = vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: self.swapchain_extent,
            };
            let render_pass_info = vk::RenderPassBeginInfo::default()
                .render_pass(self.render_pass)
                .framebuffer(self.framebuffers[image_index])
                .render_area(render_area)
                .clear_values(&clear_values);

            self.device.cmd_begin_render_pass(
                command_buffer,
                &render_pass_info,
                vk::SubpassContents::INLINE,
            );
            self.device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline,
            );

            let viewport = vk::Viewport {
                x: 0.0,
                y: 0.0,
                width: self.swapchain_extent.width as f32,
                height: self.swapchain_extent.height as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            };
            self.device.cmd_set_viewport(command_buffer, 0, &[viewport]);
            self.device
                .cmd_set_scissor(command_buffer, 0, &[render_area]);
            self.device.cmd_draw(command_buffer, 3, 1, 0, 0);
            self.device.cmd_end_render_pass(command_buffer);
            self.device.end_command_buffer(command_buffer)?;
        }

        Ok(())
    }

    fn recreate_swapchain(&mut self) -> Result<()> {
        unsafe {
            self.device.device_wait_idle()?;
            self.device
                .free_command_buffers(self.command_pool, &self.command_buffers);
        }
        self.command_buffers.clear();

        self.destroy_framebuffers();
        self.destroy_image_views();

        if !self.swapchain.is_null() {
            unsafe {
                self.swapchain_loader
                    .destroy_swapchain(self.swapchain, None);
            }
            self.swapchain = vk::SwapchainKHR::null();
        }

        self.create_swapchain()?;
        self.create_framebuffers()?;
        self.allocate_command_buffers()?;
        self.framebuffer_resized = false;

        Ok(())
    }

    fn destroy_framebuffers(&mut self) {
        for framebuffer in self.framebuffers.drain(..) {
            unsafe {
                self.device.destroy_framebuffer(framebuffer, None);
            }
        }
    }

    fn destroy_image_views(&mut self) {
        for view in self.swapchain_image_views.drain(..) {
            unsafe {
                self.device.destroy_image_view(view, None);
            }
        }
    }
}

impl Drop for VulkanRenderer {
    fn drop(&mut self) {
        unsafe {
            let _ = self.device.device_wait_idle();

            if !self.in_flight.is_null() {
                self.device.destroy_fence(self.in_flight, None);
            }
            if !self.render_finished.is_null() {
                self.device.destroy_semaphore(self.render_finished, None);
            }
            if !self.image_available.is_null() {
                self.device.destroy_semaphore(self.image_available, None);
            }
            if !self.command_pool.is_null() {
                self.device.destroy_command_pool(self.command_pool, None);
            }

            self.destroy_framebuffers();
            self.destroy_image_views();

            if !self.swapchain.is_null() {
                self.swapchain_loader
                    .destroy_swapchain(self.swapchain, None);
            }
            if !self.pipeline.is_null() {
                self.device.destroy_pipeline(self.pipeline, None);
            }
            if !self.pipeline_layout.is_null() {
                self.device
                    .destroy_pipeline_layout(self.pipeline_layout, None);
            }
            if !self.render_pass.is_null() {
                self.device.destroy_render_pass(self.render_pass, None);
            }

            self.device.destroy_device(None);

            self.surface_loader.destroy_surface(self.surface, None);

            if let Some(debug_utils) = self.debug_utils.take() {
                debug_utils
                    .loader
                    .destroy_debug_utils_messenger(debug_utils.messenger, None);
            }

            self.instance.destroy_instance(None);
        }
    }
}

fn query_surface_format(
    surface_loader: &ash::khr::surface::Instance,
    physical_device: vk::PhysicalDevice,
    surface: vk::SurfaceKHR,
    preferred: Option<vk::SurfaceFormatKHR>,
) -> Result<vk::SurfaceFormatKHR> {
    let formats =
        unsafe { surface_loader.get_physical_device_surface_formats(physical_device, surface) }?;

    if formats.is_empty() {
        bail!("the window surface does not report any supported formats");
    }

    if let Some(preferred) = preferred {
        return formats
            .iter()
            .copied()
            .find(|format| {
                format.format == preferred.format && format.color_space == preferred.color_space
            })
            .ok_or_else(|| anyhow!("the previous surface format is no longer supported"));
    }

    Ok(formats
        .iter()
        .copied()
        .find(|format| {
            format.format == vk::Format::B8G8R8A8_SRGB
                && format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
        })
        .unwrap_or(formats[0]))
}

fn create_shader_module(device: &ash::Device, code: &[u32]) -> Result<vk::ShaderModule> {
    let create_info = vk::ShaderModuleCreateInfo::default().code(code);
    unsafe { device.create_shader_module(&create_info, None) }
        .context("failed to create a shader module")
}

unsafe extern "system" fn vulkan_debug_callback(
    severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_types: vk::DebugUtilsMessageTypeFlagsEXT,
    callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT<'_>,
    _user_data: *mut c_void,
) -> vk::Bool32 {
    if callback_data.is_null() {
        return vk::FALSE;
    }

    let data = unsafe { &*callback_data };
    if data.p_message.is_null() {
        return vk::FALSE;
    }

    let message = unsafe { CStr::from_ptr(data.p_message) }.to_string_lossy();
    eprintln!("[Vulkan {severity:?} {message_types:?}] {message}");

    vk::FALSE
}
