//! 场景逻辑：相机、投影、模型动画与每帧 uniform。
//!
//! 这些属于“渲染什么/怎么动”的游戏逻辑，因此放在游戏层。
//! 渲染后端只接收 `SceneUniforms`，不依赖 glam，也不决定相机或动画。

use base::glam::{Mat4, Vec3, camera};
use render::SceneUniforms;
use std::time::Instant;

/// 维护动画时间线，并在每帧生成 MVP 与粒子更新所需的 deltaTime。
pub struct Scene {
    start_time: Instant,
    last_frame_time: Instant,
}

impl Scene {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            start_time: now,
            last_frame_time: now,
        }
    }

    /// 计算这一帧的 MVP 和粒子 deltaTime（毫秒 * 2，与教程一致）。
    /// `aspect_ratio` 来自窗口/交换链尺寸。
    pub fn frame(&mut self, aspect_ratio: f32) -> (SceneUniforms, f32) {
        let now = Instant::now();
        let delta_ms = now.duration_since(self.last_frame_time).as_secs_f32() * 1000.0;
        self.last_frame_time = now;

        let elapsed = self.start_time.elapsed().as_secs_f32();
        let aspect_ratio = aspect_ratio.max(f32::EPSILON);

        // 模型绕 Z 轴每秒旋转 90°。
        let model = Mat4::from_rotation_z(elapsed * std::f32::consts::FRAC_PI_2);

        // 相机位于 (2,2,2)，看向原点，世界坐标 Z 轴向上。
        let view = camera::rh::view::look_at_mat4(Vec3::splat(2.0), Vec3::ZERO, Vec3::Z);

        // Vulkan 投影：深度 [0,1]、NDC Y 向下，与教程的 proj[1][1] *= -1 等价。
        let projection =
            camera::rh::proj::vulkan::perspective(45.0_f32.to_radians(), aspect_ratio, 0.1, 10.0);

        let uniforms = SceneUniforms {
            model: model.to_cols_array_2d(),
            view: view.to_cols_array_2d(),
            projection: projection.to_cols_array_2d(),
        };

        (uniforms, delta_ms * 2.0)
    }
}
