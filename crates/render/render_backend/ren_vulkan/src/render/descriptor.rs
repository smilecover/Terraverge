//! 描述符集布局、描述符池与描述符集。
//!
//! 对应教程「Descriptor set layout and buffer」中的布局部分和
//! 「Descriptor pool and sets」章节。当前只有一个描述符：
//! set 0 / binding 0 的 uniform buffer，只在顶点阶段可见。

use ash::vk;
use std::error::Error;

use super::buffer::Buffer;
use super::sync::MAX_FRAMES_IN_FLIGHT;
use super::texture::Texture;

/// 创建描述符集布局（教程的 `createDescriptorSetLayout`）。
pub(super) fn create_descriptor_set_layout(
    device: &ash::Device,
) -> Result<vk::DescriptorSetLayout, vk::Result> {
    // binding 0：顶点阶段使用的 uniform buffer（MVP 矩阵）。
    let uniform_buffer_binding = vk::DescriptorSetLayoutBinding::default()
        .binding(0)
        .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
        .descriptor_count(1)
        .stage_flags(vk::ShaderStageFlags::VERTEX);

    // binding 1：片元阶段使用的组合图像采样器（纹理 + 采样器）。
    let combined_image_sampler_binding = vk::DescriptorSetLayoutBinding::default()
        .binding(1)
        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
        .descriptor_count(1)
        .stage_flags(vk::ShaderStageFlags::FRAGMENT);

    let bindings = [uniform_buffer_binding, combined_image_sampler_binding];
    let create_info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings);

    unsafe { device.create_descriptor_set_layout(&create_info, None) }
}

/// 创建描述符池（教程的 `createDescriptorPool`）。
/// 池的大小要能容纳每个在途帧一个 uniform buffer 描述符。
pub(super) fn create_descriptor_pool(
    device: &ash::Device,
) -> Result<vk::DescriptorPool, vk::Result> {
    let uniform_buffer_size = vk::DescriptorPoolSize::default()
        .ty(vk::DescriptorType::UNIFORM_BUFFER)
        .descriptor_count(MAX_FRAMES_IN_FLIGHT as u32);
    let combined_image_sampler_size = vk::DescriptorPoolSize::default()
        .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
        .descriptor_count(MAX_FRAMES_IN_FLIGHT as u32);
    let pool_sizes = [uniform_buffer_size, combined_image_sampler_size];

    let create_info = vk::DescriptorPoolCreateInfo::default()
        .pool_sizes(&pool_sizes)
        .max_sets(MAX_FRAMES_IN_FLIGHT as u32);

    unsafe { device.create_descriptor_pool(&create_info, None) }
}

/// 分配每个在途帧一个描述符集，并把该帧的 uniform buffer 写进 binding 0
/// （教程的 `createDescriptorSets`：分配后立即 `vkUpdateDescriptorSets`）。
pub(super) fn create_descriptor_sets(
    device: &ash::Device,
    descriptor_pool: vk::DescriptorPool,
    descriptor_set_layout: vk::DescriptorSetLayout,
    uniform_buffers: &[Buffer],
    texture: &Texture,
) -> Result<Vec<vk::DescriptorSet>, Box<dyn Error + Send + Sync>> {
    debug_assert_eq!(uniform_buffers.len(), MAX_FRAMES_IN_FLIGHT);

    // 每个描述符集都按同一份布局创建，数量与在途帧数量一致。
    let layouts = vec![descriptor_set_layout; MAX_FRAMES_IN_FLIGHT];
    let allocate_info = vk::DescriptorSetAllocateInfo::default()
        .descriptor_pool(descriptor_pool)
        .set_layouts(&layouts);

    let descriptor_sets = unsafe { device.allocate_descriptor_sets(&allocate_info)? };

    for (index, descriptor_set) in descriptor_sets.iter().enumerate() {
        // 描述符指向这一帧专属的 UBO，范围覆盖整个 UniformBufferObject。
        let buffer_info = vk::DescriptorBufferInfo::default()
            .buffer(uniform_buffers[index].handle)
            .offset(0)
            .range(uniform_buffers[index].size);

        // binding 1：组合图像采样器要求图像处于 SHADER_READ_ONLY 布局。
        let image_info = vk::DescriptorImageInfo::default()
            .sampler(texture.sampler())
            .image_view(texture.view())
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);

        let buffer_infos = [buffer_info];
        let image_infos = [image_info];
        let writes = [
            vk::WriteDescriptorSet::default()
                .dst_set(*descriptor_set)
                .dst_binding(0)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(&buffer_infos),
            vk::WriteDescriptorSet::default()
                .dst_set(*descriptor_set)
                .dst_binding(1)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(&image_infos),
        ];

        unsafe { device.update_descriptor_sets(&writes, &[]) };
    }

    Ok(descriptor_sets)
}
