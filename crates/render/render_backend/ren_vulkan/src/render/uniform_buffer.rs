//! Uniform 缓冲：每帧把游戏层算好的 MVP 矩阵写进 GPU 内存。
//!
//! 对应教程「Descriptor set layout and buffer」中的 Uniform buffer 部分。
//! 每个在途帧各有一个 `HOST_VISIBLE | HOST_COHERENT` 的 UBO，
//! 初始化时映射一次并一直保持映射，`update` 直接往映射地址写新矩阵。
//!
//! 相机、投影和动画属于场景逻辑，由游戏层负责计算；渲染后端只负责
//! 把收到的矩阵写进缓冲，不依赖任何数学库。

use ash::vk;
use std::error::Error;
use std::mem::size_of;

use super::buffer::{Buffer, create_buffer};
use super::sync::MAX_FRAMES_IN_FLIGHT;

/// 游戏层每帧提供的 MVP 矩阵。
///
/// 三个矩阵按**列主序**存放（每列一个 `[f32; 4]`），与着色器里的
/// `float4x4` 及 Slang 的列主序编译选项对应。字段顺序必须与
/// `ConstantBuffer<UniformBufferObject>` 完全一致。
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SceneUniforms {
    pub model: [[f32; 4]; 4],
    pub view: [[f32; 4]; 4],
    pub projection: [[f32; 4]; 4],
}

/// 每帧一个 uniform buffer，附带持久映射的 CPU 地址。
pub(super) struct UniformBuffers {
    buffers: Vec<Buffer>,
    /// `map_memory` 返回的地址；数量与 `buffers` 一一对应。
    mapped: Vec<*mut u8>,
}

impl UniformBuffers {
    /// 创建并映射 `MAX_FRAMES_IN_FLIGHT` 个 uniform buffer
    /// （教程的 `createUniformBuffers`）。
    pub(super) fn create(
        instance: &ash::Instance,
        device: &ash::Device,
        physical_device: vk::PhysicalDevice,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let size = size_of::<SceneUniforms>() as vk::DeviceSize;

        let mut buffers = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);
        let mut mapped = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);

        for _ in 0..MAX_FRAMES_IN_FLIGHT {
            // UBO 由 CPU 每帧写入，必须放在 CPU 可见且一致的内存里。
            // 这里不需要 staging buffer：它就是每帧要被覆盖的数据。
            let buffer = match create_buffer(
                instance,
                device,
                physical_device,
                size,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            ) {
                Ok(buffer) => buffer,
                Err(err) => {
                    destroy_buffers(device, &mut buffers);
                    return Err(err);
                }
            };

            // 持久映射：只 map 一次，之后每帧直接写指针。
            let pointer = match unsafe {
                device.map_memory(buffer.memory, 0, size, vk::MemoryMapFlags::empty())
            } {
                Ok(pointer) => pointer.cast::<u8>(),
                Err(err) => {
                    buffer.destroy(device);
                    destroy_buffers(device, &mut buffers);
                    return Err(err.into());
                }
            };

            buffers.push(buffer);
            mapped.push(pointer);
        }

        Ok(Self { buffers, mapped })
    }

    /// 给描述符集写入 buffer 信息时使用。
    pub(super) fn buffers(&self) -> &[Buffer] {
        &self.buffers
    }

    /// 把游戏层算好的 MVP 写入当前帧的 UBO（教程的 `updateUniformBuffer`）。
    /// `frame` 是在途帧索引，不是交换链图像索引。
    pub(super) fn update(&self, frame: usize, uniforms: &SceneUniforms) {
        // HOST_COHERENT 内存写入后不需要手动 flush；帧栅栏已保证 GPU 不再读它。
        unsafe {
            (self.mapped[frame] as *mut SceneUniforms).write(*uniforms);
        }
    }

    /// 先解除映射再释放缓冲和内存。
    pub(super) fn destroy(&self, device: &ash::Device) {
        for buffer in &self.buffers {
            unsafe { device.unmap_memory(buffer.memory) };
            buffer.destroy(device);
        }
    }
}

/// 创建中途失败时回收已经映射/创建的 buffer。
fn destroy_buffers(device: &ash::Device, buffers: &mut Vec<Buffer>) {
    for buffer in buffers.drain(..) {
        unsafe { device.unmap_memory(buffer.memory) };
        buffer.destroy(device);
    }
}
