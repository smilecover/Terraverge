//! 游戏场景：第一人称玩家、相机与每帧 MVP。
//!
//! 逻辑与 Java 客户端一致地分成“实体刻更新”和“渲染相机”两步：
//! `update` 用输入驱动 `LocalPlayer`，随后相机贴合眼睛位置与视角；
//! `frame` 只负责把当前相机状态转换为渲染后端需要的矩阵。

use base::glam::camera::rh::view::look_to_mat4;
use base::glam::{Mat4, Vec3 as GlamVec3, camera};
use minecraft::net::minecraft::client::camera::Camera;
use minecraft::net::minecraft::client::input::Input;
use minecraft::net::minecraft::client::player::LocalPlayer;
use minecraft::net::minecraft::world::level::Level;
use minecraft::net::minecraft::world::phys::Vec3;
use render::SceneUniforms;

pub struct Scene {
    player: LocalPlayer,
    camera: Camera,
}

impl Scene {
    pub fn new(spawn: Vec3) -> Self {
        let player = LocalPlayer::new(spawn);
        let mut camera = Camera::new();
        camera.set_position(player.get_eye_position());
        camera.set_rotation(player.get_y_rot(), player.get_x_rot());
        Self { player, camera }
    }

    pub fn update(&mut self, level: &Level, input: &Input, dt: f32) {
        self.player.tick(level, input, dt);
        self.camera.set_position(self.player.get_eye_position());
        self.camera
            .set_rotation(self.player.get_y_rot(), self.player.get_x_rot());
    }

    /// 鼠标旋转视角。灵敏度单位：度/像素（原版默认约 0.15）。
    pub fn turn(&mut self, delta_x: f64, delta_y: f64, sensitivity: f64) {
        self.player.turn(delta_x, delta_y, sensitivity);
    }

    /// 构造本帧的 MVP。`aspect_ratio` = 宽 / 高。
    pub fn frame(&self, aspect_ratio: f32) -> SceneUniforms {
        let eye = self.camera.get_position();
        let look = self.camera.get_look_vector();
        let eye = GlamVec3::new(eye.x as f32, eye.y as f32, eye.z as f32);
        let direction = GlamVec3::new(look.x as f32, look.y as f32, look.z as f32).normalize();
        let view = look_to_mat4(eye, direction, GlamVec3::Y);

        // Minecraft 默认 FOV 70°（垂直）。近平面取小值以贴近方块表面。
        let projection = camera::rh::proj::vulkan::perspective(
            70.0_f32.to_radians(),
            aspect_ratio.max(f32::EPSILON),
            0.05,
            512.0,
        );

        SceneUniforms {
            model: Mat4::IDENTITY.to_cols_array_2d(),
            view: view.to_cols_array_2d(),
            projection: projection.to_cols_array_2d(),
        }
    }
}
