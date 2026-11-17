//! 帧同步：信号量/栅栏，以及“交换链图像是否仍在使用”的栅栏表。
//!
//! 对应教程的「Frames in flight」章节。同时允许多帧在途，CPU 不必等 GPU
//! 画完上一帧就能准备下一帧；信号量负责队列内部与呈现之间的顺序，
//! 栅栏负责 CPU 与 GPU 之间的等待。
//!
//! 与教程有一处不同：呈现完成信号量按**交换链图像**分配，而不是按帧分配。
//! 呈现操作没有栅栏可等，按帧复用的信号量可能在上一张图像还没重新获取时
//! 就被拿去呈现另一张图像，触发 VUID-vkQueueSubmit-pSignalSemaphores-00067。
//! 按图像分配后，同一信号量只在重新获取同一张图像时复用，天然安全。

use ash::vk;
use std::error::Error;

/// 同时录制的帧数。2 表示一帧在 GPU 上执行时，CPU 可以准备下一帧。
pub(super) const MAX_FRAMES_IN_FLIGHT: usize = 2;

/// 帧间同步对象；交换链重建时只有按图像分配的部分需要更新。
pub(super) struct FrameSync {
    /// 每帧一个：交换链图像可用时由呈现引擎发出信号，提交时等待它。
    /// 栅栏保证同一帧的信号量在被复用时，上一次等待已经执行完。
    image_available: Vec<vk::Semaphore>,
    /// 每张交换链图像一个：渲染完成时由提交发出信号，呈现时等待它。
    render_finished: Vec<vk::Semaphore>,
    /// 每帧一个栅栏，CPU 通过它知道这一帧的 GPU 工作已经结束。
    in_flight_fences: Vec<vk::Fence>,
    /// 按交换链图像索引记录正在使用该图像的栅栏；`null` 表示当前空闲。
    images_in_flight: Vec<vk::Fence>,
    pub(super) current_frame: usize,
}

impl FrameSync {
    pub(super) fn create(
        device: &ash::Device,
        image_count: usize,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let mut sync = Self {
            image_available: Vec::with_capacity(MAX_FRAMES_IN_FLIGHT),
            render_finished: Vec::with_capacity(image_count),
            in_flight_fences: Vec::with_capacity(MAX_FRAMES_IN_FLIGHT),
            images_in_flight: vec![vk::Fence::null(); image_count],
            current_frame: 0,
        };

        let semaphore_info = vk::SemaphoreCreateInfo::default();
        // 栅栏必须初始为“已发出信号”，否则第一帧会永远等不到。
        let fence_info = vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);

        for _ in 0..MAX_FRAMES_IN_FLIGHT {
            let image_available = match unsafe { device.create_semaphore(&semaphore_info, None) } {
                Ok(semaphore) => semaphore,
                Err(err) => {
                    sync.destroy(device);
                    return Err(err.into());
                }
            };
            sync.image_available.push(image_available);

            let fence = match unsafe { device.create_fence(&fence_info, None) } {
                Ok(fence) => fence,
                Err(err) => {
                    sync.destroy(device);
                    return Err(err.into());
                }
            };
            sync.in_flight_fences.push(fence);
        }

        sync.render_finished = match create_semaphores(device, image_count) {
            Ok(semaphores) => semaphores,
            Err(err) => {
                sync.destroy(device);
                return Err(err.into());
            }
        };

        Ok(sync)
    }

    /// 交换链重建后换一批呈现信号量，并重置图像占用表。
    ///
    /// 调用前必须已经 `device_wait_idle` 且旧交换链已经销毁，
    /// 否则旧的呈现操作可能仍持有这些信号量。
    pub(super) fn recreate_image_semaphores(
        &mut self,
        device: &ash::Device,
        image_count: usize,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        for &semaphore in &self.render_finished {
            unsafe { device.destroy_semaphore(semaphore, None) };
        }
        self.render_finished.clear();

        self.render_finished = create_semaphores(device, image_count)?;
        self.images_in_flight = vec![vk::Fence::null(); image_count];
        Ok(())
    }

    /// 提交时等待的“图像可用”信号量（按帧）。
    pub(super) fn image_available(&self, frame: usize) -> vk::Semaphore {
        self.image_available[frame]
    }

    /// 提交时发出、呈现时等待的“渲染完成”信号量（按图像）。
    pub(super) fn render_finished(&self, image_index: usize) -> vk::Semaphore {
        self.render_finished[image_index]
    }

    pub(super) fn in_flight_fence(&self, frame: usize) -> vk::Fence {
        self.in_flight_fences[frame]
    }

    pub(super) fn images_in_flight(&self, image_index: usize) -> vk::Fence {
        self.images_in_flight[image_index]
    }

    pub(super) fn set_image_in_flight(&mut self, image_index: usize, fence: vk::Fence) {
        self.images_in_flight[image_index] = fence;
    }

    pub(super) fn advance_frame(&mut self) {
        self.current_frame = (self.current_frame + 1) % MAX_FRAMES_IN_FLIGHT;
    }

    pub(super) fn destroy(&self, device: &ash::Device) {
        unsafe {
            for &semaphore in self.image_available.iter().chain(&self.render_finished) {
                device.destroy_semaphore(semaphore, None);
            }
            for &fence in &self.in_flight_fences {
                device.destroy_fence(fence, None);
            }
        }
    }
}

/// 创建 `count` 个二进制信号量；中途失败时回收已创建的部分。
fn create_semaphores(device: &ash::Device, count: usize) -> Result<Vec<vk::Semaphore>, vk::Result> {
    let create_info = vk::SemaphoreCreateInfo::default();
    let mut semaphores = Vec::with_capacity(count);

    for _ in 0..count {
        match unsafe { device.create_semaphore(&create_info, None) } {
            Ok(semaphore) => semaphores.push(semaphore),
            Err(err) => {
                for &semaphore in &semaphores {
                    unsafe { device.destroy_semaphore(semaphore, None) };
                }
                return Err(err);
            }
        }
    }

    Ok(semaphores)
}
