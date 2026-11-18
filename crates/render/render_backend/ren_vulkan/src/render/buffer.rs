//! 顶点/索引缓冲与暂存缓冲（staging buffer）。
//!
//! 对应教程的「Vertex buffer creation」「Staging buffer」「Index buffer」章节。
//! GPU 读取最快的显存（`DEVICE_LOCAL`）通常不能直接被 CPU 写入，所以流程是：
//! 1. 创建 `HOST_VISIBLE | HOST_COHERENT` 的暂存缓冲，映射后把数据拷进去；
//! 2. 创建带 `TRANSFER_DST` 的设备本地缓冲；
//! 3. 用一次性的传输命令把暂存缓冲复制到设备本地缓冲，再销毁暂存缓冲。

use ash::vk;
use std::error::Error;
use std::mem::size_of_val;
use std::slice;

use super::command_buffer::submit_one_time;
use super::vertex::{MeshData, Vertex};

/// 一个已经绑定内存的缓冲。销毁顺序：先销毁缓冲，再释放内存。
pub(super) struct Buffer {
    pub(super) handle: vk::Buffer,
    pub(super) memory: vk::DeviceMemory,
    /// 创建时请求的字节数，写描述符时直接用作 buffer range。
    pub(super) size: vk::DeviceSize,
}

impl Buffer {
    pub(super) fn destroy(&self, device: &ash::Device) {
        unsafe {
            device.destroy_buffer(self.handle, None);
            device.free_memory(self.memory, None);
        }
    }
}

/// 顶点/索引缓冲及索引数量，录制命令时一起传给 `record_command_buffer`。
pub(super) struct Geometry {
    pub(super) vertex_buffer: Buffer,
    pub(super) index_buffer: Buffer,
    pub(super) index_count: u32,
}

impl Geometry {
    /// 与创建顺序相反：先销毁索引缓冲，再销毁顶点缓冲。
    pub(super) fn destroy(&self, device: &ash::Device) {
        self.index_buffer.destroy(device);
        self.vertex_buffer.destroy(device);
    }
}

/// 创建顶点缓冲和索引缓冲（教程的 `createVertexBuffer` + `createIndexBuffer`）。
/// 中途失败时回收已经创建好的那一个。
pub(super) fn create_geometry(
    instance: &ash::Instance,
    device: &ash::Device,
    physical_device: vk::PhysicalDevice,
    command_pool: vk::CommandPool,
    graphics_queue: vk::Queue,
    mesh: MeshData<'_>,
) -> Result<Geometry, Box<dyn Error + Send + Sync>> {
    if mesh.vertices.is_empty() || mesh.indices.is_empty() {
        return Err("网格没有顶点或索引".into());
    }

    let vertex_buffer = create_vertex_buffer(
        instance,
        device,
        physical_device,
        command_pool,
        graphics_queue,
        mesh.vertices,
    )?;

    let index_buffer = match create_index_buffer(
        instance,
        device,
        physical_device,
        command_pool,
        graphics_queue,
        mesh.indices,
    ) {
        Ok(index_buffer) => index_buffer,
        Err(err) => {
            vertex_buffer.destroy(device);
            return Err(err);
        }
    };

    Ok(Geometry {
        vertex_buffer,
        index_buffer,
        index_count: mesh.indices.len() as u32,
    })
}

/// 顶点缓冲使用 `VERTEX_BUFFER` 用途。
fn create_vertex_buffer(
    instance: &ash::Instance,
    device: &ash::Device,
    physical_device: vk::PhysicalDevice,
    command_pool: vk::CommandPool,
    graphics_queue: vk::Queue,
    vertices: &[Vertex],
) -> Result<Buffer, Box<dyn Error + Send + Sync>> {
    create_buffer_with_data(
        instance,
        device,
        physical_device,
        command_pool,
        graphics_queue,
        as_bytes(vertices),
        vk::BufferUsageFlags::VERTEX_BUFFER,
    )
}

/// 索引缓冲使用 `INDEX_BUFFER` 用途。
fn create_index_buffer(
    instance: &ash::Instance,
    device: &ash::Device,
    physical_device: vk::PhysicalDevice,
    command_pool: vk::CommandPool,
    graphics_queue: vk::Queue,
    indices: &[u32],
) -> Result<Buffer, Box<dyn Error + Send + Sync>> {
    create_buffer_with_data(
        instance,
        device,
        physical_device,
        command_pool,
        graphics_queue,
        as_bytes(indices),
        vk::BufferUsageFlags::INDEX_BUFFER,
    )
}

/// 通过暂存缓冲把 `data` 上传到设备本地缓冲（教程的 staging buffer 流程）。
pub(super) fn create_buffer_with_data(
    instance: &ash::Instance,
    device: &ash::Device,
    physical_device: vk::PhysicalDevice,
    command_pool: vk::CommandPool,
    graphics_queue: vk::Queue,
    data: &[u8],
    usage: vk::BufferUsageFlags,
) -> Result<Buffer, Box<dyn Error + Send + Sync>> {
    let size = data.len() as vk::DeviceSize;

    // 1-2. 创建暂存缓冲并写入数据。
    let staging = create_staging_buffer(instance, device, physical_device, data)?;

    // 3. 目标缓冲：设备本地内存，GPU 读取最快；同时作为传输目标。
    let buffer = match create_buffer(
        instance,
        device,
        physical_device,
        size,
        usage | vk::BufferUsageFlags::TRANSFER_DST,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    ) {
        Ok(buffer) => buffer,
        Err(err) => {
            staging.destroy(device);
            return Err(err);
        }
    };

    // 4. 一次性复制命令把内容搬过去，之后暂存缓冲就没用了。
    if let Err(err) = copy_buffer(
        device,
        command_pool,
        graphics_queue,
        staging.handle,
        buffer.handle,
        size,
    ) {
        buffer.destroy(device);
        staging.destroy(device);
        return Err(err);
    }
    staging.destroy(device);

    Ok(buffer)
}

/// 创建一个缓冲并分配、绑定满足 `properties` 要求的内存（教程的 `createBuffer`）。
pub(super) fn create_buffer(
    instance: &ash::Instance,
    device: &ash::Device,
    physical_device: vk::PhysicalDevice,
    size: vk::DeviceSize,
    usage: vk::BufferUsageFlags,
    properties: vk::MemoryPropertyFlags,
) -> Result<Buffer, Box<dyn Error + Send + Sync>> {
    // 缓冲只被图形队列使用，独占共享模式即可。
    let create_info = vk::BufferCreateInfo::default()
        .size(size)
        .usage(usage)
        .sharing_mode(vk::SharingMode::EXCLUSIVE);

    let buffer = unsafe { device.create_buffer(&create_info, None)? };

    // 驱动可能对对齐/大小有额外要求，必须按 MemoryRequirements 分配。
    let requirements = unsafe { device.get_buffer_memory_requirements(buffer) };
    let memory_properties =
        unsafe { instance.get_physical_device_memory_properties(physical_device) };

    let memory_type_index = match find_memory_type(
        &memory_properties,
        requirements.memory_type_bits,
        properties,
    ) {
        Some(index) => index,
        None => {
            unsafe { device.destroy_buffer(buffer, None) };
            return Err("找不到满足要求的显存类型".into());
        }
    };

    let allocate_info = vk::MemoryAllocateInfo::default()
        .allocation_size(requirements.size)
        .memory_type_index(memory_type_index);

    let memory = match unsafe { device.allocate_memory(&allocate_info, None) } {
        Ok(memory) => memory,
        Err(err) => {
            unsafe { device.destroy_buffer(buffer, None) };
            return Err(err.into());
        }
    };

    if let Err(err) = unsafe { device.bind_buffer_memory(buffer, memory, 0) } {
        unsafe {
            device.free_memory(memory, None);
            device.destroy_buffer(buffer, None);
        }
        return Err(err.into());
    }

    Ok(Buffer {
        handle: buffer,
        memory,
        size,
    })
}

/// 在允许的内存类型里找第一个同时具备 `required` 属性的类型
/// （教程的 `findMemoryType`）。`type_filter` 是驱动的位掩码。
pub(super) fn find_memory_type(
    memory_properties: &vk::PhysicalDeviceMemoryProperties,
    type_filter: u32,
    required: vk::MemoryPropertyFlags,
) -> Option<u32> {
    (0..memory_properties.memory_type_count).find(|&index| {
        type_filter & (1 << index) != 0
            && memory_properties.memory_types[index as usize]
                .property_flags
                .contains(required)
    })
}

/// 创建 `HOST_VISIBLE | HOST_COHERENT` 的暂存缓冲，映射后把 `data` 拷进去。
/// 顶点/索引缓冲和纹理上传共用（教程的 staging buffer）。
pub(super) fn create_staging_buffer(
    instance: &ash::Instance,
    device: &ash::Device,
    physical_device: vk::PhysicalDevice,
    data: &[u8],
) -> Result<Buffer, Box<dyn Error + Send + Sync>> {
    let size = data.len() as vk::DeviceSize;

    let staging = create_buffer(
        instance,
        device,
        physical_device,
        size,
        vk::BufferUsageFlags::TRANSFER_SRC,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    )?;

    // 映射后用 memcpy 写入数据，然后立即解除映射。
    match unsafe { device.map_memory(staging.memory, 0, size, vk::MemoryMapFlags::empty()) } {
        Ok(mapped) => {
            // SAFETY: 映射区至少有 `size` 字节；data 是有效且不重叠的源切片。
            unsafe {
                std::ptr::copy_nonoverlapping(data.as_ptr(), mapped.cast::<u8>(), data.len());
                device.unmap_memory(staging.memory);
            }
            Ok(staging)
        }
        Err(err) => {
            staging.destroy(device);
            Err(err.into())
        }
    }
}

/// 用一次性命令缓冲把 `source` 的内容复制到 `destination`（教程的 `copyBuffer`）。
fn copy_buffer(
    device: &ash::Device,
    command_pool: vk::CommandPool,
    graphics_queue: vk::Queue,
    source: vk::Buffer,
    destination: vk::Buffer,
    size: vk::DeviceSize,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    submit_one_time(device, command_pool, graphics_queue, |command_buffer| {
        let region = vk::BufferCopy::default().size(size);
        unsafe { device.cmd_copy_buffer(command_buffer, source, destination, &[region]) };
    })
}

/// 把纯数据的切片按字节看待，用于 `map_memory` 拷贝。
///
/// SAFETY（调用方保证）：`T` 必须是 `#[repr(C)]` 的纯数据、不含指针/引用，
/// 也没有 `Drop`；本模块只对 `Vertex` 和 `u16` 调用。
fn as_bytes<T>(data: &[T]) -> &[u8] {
    unsafe { slice::from_raw_parts(data.as_ptr().cast(), size_of_val(data)) }
}
