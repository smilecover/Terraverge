//! 对应 `net/minecraft/client/Camera.java` 的核心视图参数。
//!
//! 26.3 的相机还包含视锥、雾、透视切换与 bobbing；当前提供位置、
//! 世界坐标下的朝向与基向量，供游戏层构造视图矩阵。

use crate::net::minecraft::world::phys::Vec3;

#[derive(Clone, Debug)]
pub struct Camera {
    position: Vec3,
    y_rot: f32,
    x_rot: f32,
}

impl Camera {
    pub fn new() -> Self {
        Self {
            position: Vec3::ZERO,
            y_rot: 0.0,
            x_rot: 0.0,
        }
    }

    pub fn set_position(&mut self, position: Vec3) {
        self.position = position;
    }

    pub fn get_position(&self) -> Vec3 {
        self.position
    }

    pub fn set_rotation(&mut self, y_rot: f32, x_rot: f32) {
        self.y_rot = y_rot;
        self.x_rot = x_rot;
    }

    pub fn get_y_rot(&self) -> f32 {
        self.y_rot
    }

    pub fn get_x_rot(&self) -> f32 {
        self.x_rot
    }

    /// Minecraft 坐标约定：yaw=0 朝南(+Z)，pitch>0 向下看。
    pub fn get_look_vector(&self) -> Vec3 {
        let yaw = (self.y_rot as f64).to_radians();
        let pitch = (self.x_rot as f64).to_radians();
        let cos_pitch = pitch.cos();
        Vec3::new(-yaw.sin() * cos_pitch, -pitch.sin(), yaw.cos() * cos_pitch)
    }

    /// 朝向的水平分量，供移动逻辑使用。
    pub fn get_forward_horizontal(&self) -> Vec3 {
        let yaw = (self.y_rot as f64).to_radians();
        Vec3::new(-yaw.sin(), 0.0, yaw.cos())
    }

    pub fn get_left_horizontal(&self) -> Vec3 {
        let yaw = (self.y_rot as f64).to_radians();
        Vec3::new(yaw.cos(), 0.0, yaw.sin())
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self::new()
    }
}
