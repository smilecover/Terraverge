//! 着色器模块：把 SPIR-V 字数组包装成 `VkShaderModule`，并生成管线需要的阶段描述。
//!
//! 对应教程的「Shader modules」章节。模块本身只是 SPIR-V 的容器，
//! 图形管线创建完成后就可以销毁，不需要长期保存。

use ash::vk;
use std::error::Error;
use std::ffi::CString;

/// 一段已编译的 SPIR-V 及其入口点名称。
///
/// `code` 是 SPIR-V 的 32 位字数组（小端），可以直接传给 `vkCreateShaderModule`；
/// `entry_point` 必须与 SPIR-V 里的 `OpEntryPoint` 名称一致。
pub struct ShaderStage<'a> {
    pub code: &'a [u32],
    pub entry_point: &'a str,
}

/// 创建图形管线所需的顶点/片元着色器。
pub struct ShaderProgram<'a> {
    pub vertex: ShaderStage<'a>,
    pub fragment: ShaderStage<'a>,
}

/// 顶点/片元着色器模块。入口点名称也一并持有，
/// 因为 `VkPipelineShaderStageCreateInfo` 只借用 `&CStr`，必须保证生命周期足够长。
pub(super) struct ShaderModules {
    vertex: vk::ShaderModule,
    fragment: vk::ShaderModule,
    vertex_entry: CString,
    fragment_entry: CString,
}

impl ShaderModules {
    pub(super) fn create(
        device: &ash::Device,
        program: &ShaderProgram<'_>,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        // 入口点名称在管线创建时需要以 NUL 结尾的字符串传入。
        let vertex_entry = CString::new(program.vertex.entry_point)?;
        let fragment_entry = CString::new(program.fragment.entry_point)?;

        let vertex = create_shader_module(device, program.vertex.code)?;
        let fragment = match create_shader_module(device, program.fragment.code) {
            Ok(module) => module,
            Err(err) => {
                // 片元模块失败时回收已经创建好的顶点模块。
                unsafe { device.destroy_shader_module(vertex, None) };
                return Err(err.into());
            }
        };

        Ok(Self {
            vertex,
            fragment,
            vertex_entry,
            fragment_entry,
        })
    }

    /// 生成顶点/片元两个管线阶段（教程的 `vertShaderStageInfo` / `fragShaderStageInfo`）。
    pub(super) fn stages(&self) -> [vk::PipelineShaderStageCreateInfo<'_>; 2] {
        [
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(self.vertex)
                .name(&self.vertex_entry),
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(self.fragment)
                .name(&self.fragment_entry),
        ]
    }

    /// 管线创建完成后销毁两个模块。
    pub(super) fn destroy(&self, device: &ash::Device) {
        unsafe {
            device.destroy_shader_module(self.vertex, None);
            device.destroy_shader_module(self.fragment, None);
        }
    }
}

/// 创建单个着色器模块（教程的 `createShaderModule`）。
fn create_shader_module(
    device: &ash::Device,
    code: &[u32],
) -> Result<vk::ShaderModule, vk::Result> {
    let create_info = vk::ShaderModuleCreateInfo::default().code(code);
    unsafe { device.create_shader_module(&create_info, None) }
}
