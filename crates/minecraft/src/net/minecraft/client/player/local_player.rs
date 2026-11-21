//! 对应 `net/minecraft/client/player/LocalPlayer.java` 的玩家实体部分。
//!
//! 物理实现说明：
//! - Java 的实体移动是 20 TPS 固定刻 + 子刻插值；本里程碑使用每帧
//!   `dt` 直接积分，并在轴向上以 0.05 方块步长扫掠避免穿透；
//! - 碰撞形状暂时统一按整方块处理（`Block::has_collision`），
//!   26.3 的 VoxelShape 系统待移植；
//! - 行走/疾跑/潜行速度取自原版数值，跳跃初速度与重力经过等价换算。

use crate::net::minecraft::client::input::Input;
use crate::net::minecraft::core::BlockPos;
use crate::net::minecraft::world::level::Level;
use crate::net::minecraft::world::phys::{AABB, Vec3};

pub struct LocalPlayer {
    position: Vec3,
    velocity: Vec3,
    yaw: f32,
    pitch: f32,
    on_ground: bool,
}

impl LocalPlayer {
    pub const WIDTH: f64 = 0.6;
    pub const HEIGHT: f64 = 1.8;
    pub const EYE_HEIGHT: f64 = 1.62;
    pub const WALK_SPEED: f32 = 4.317;
    pub const SPRINT_SPEED: f32 = 5.612;
    pub const SNEAK_SPEED: f32 = 1.31;
    pub const GRAVITY: f32 = 32.0;
    pub const JUMP_VELOCITY: f32 = 9.0;
    pub const TERMINAL_VELOCITY: f32 = -60.0;

    pub fn new(spawn: Vec3) -> Self {
        Self {
            position: spawn,
            velocity: Vec3::ZERO,
            yaw: 0.0,
            pitch: 10.0,
            on_ground: false,
        }
    }

    pub fn get_position(&self) -> Vec3 {
        self.position
    }

    pub fn set_position(&mut self, position: Vec3) {
        self.position = position;
    }

    pub fn get_velocity(&self) -> Vec3 {
        self.velocity
    }

    pub fn get_eye_position(&self) -> Vec3 {
        self.position.add_xyz(0.0, Self::EYE_HEIGHT, 0.0)
    }

    pub fn get_y_rot(&self) -> f32 {
        self.yaw
    }

    pub fn get_x_rot(&self) -> f32 {
        self.pitch
    }

    pub fn set_rotation(&mut self, yaw: f32, pitch: f32) {
        self.yaw = yaw.rem_euclid(360.0);
        self.pitch = pitch.clamp(-89.9, 89.9);
    }

    /// 鼠标输入：`delta_x/y` 是像素位移，`sensitivity` 是度/像素。
    pub fn turn(&mut self, delta_x: f64, delta_y: f64, sensitivity: f64) {
        let yaw = self.yaw as f64 - delta_x * sensitivity;
        let pitch = self.pitch as f64 + delta_y * sensitivity;
        self.set_rotation(yaw as f32, pitch as f32);
    }

    pub fn is_on_ground(&self) -> bool {
        self.on_ground
    }

    pub fn get_bounding_box(&self) -> AABB {
        let half_width = Self::WIDTH / 2.0;
        AABB::new(
            self.position.x - half_width,
            self.position.y,
            self.position.z - half_width,
            self.position.x + half_width,
            self.position.y + Self::HEIGHT,
            self.position.z + half_width,
        )
    }

    /// 跟随帧调用。`dt` 已在调用方限制到合理范围。
    pub fn tick(&mut self, level: &Level, input: &Input, dt: f32) {
        let forward = input.move_forward();
        let strafe = input.move_strafe();

        let yaw = self.yaw.to_radians();
        // Minecraft 视图：yaw=0 朝南(+Z)，左方向为 +X 旋转后的方向。
        let forward_x = -yaw.sin();
        let forward_z = yaw.cos();
        let left_x = yaw.cos();
        let left_z = yaw.sin();

        let mut move_x = forward_x * forward + left_x * strafe;
        let mut move_z = forward_z * forward + left_z * strafe;
        let length_sqr = move_x * move_x + move_z * move_z;
        if length_sqr > 1.0 {
            let length = length_sqr.sqrt();
            move_x /= length;
            move_z /= length;
        }

        let speed = if input.has_shift_key_down() {
            Self::SNEAK_SPEED
        } else if input.sprinting && input.forward {
            Self::SPRINT_SPEED
        } else {
            Self::WALK_SPEED
        };

        self.velocity.x = move_x as f64 * speed as f64;
        self.velocity.z = move_z as f64 * speed as f64;

        if input.jumping && self.on_ground {
            self.velocity.y = Self::JUMP_VELOCITY as f64;
            self.on_ground = false;
        }
        self.velocity.y = (self.velocity.y - Self::GRAVITY as f64 * dt as f64)
            .max(Self::TERMINAL_VELOCITY as f64);

        self.move_with_collision(level, self.velocity.scale(dt as f64));

        // 静止在地面时，重力每帧都会产生向下的微小位移并立即被碰撞挡住；
        // 这里补一次贴地检测，保证 `on_ground` 稳定。
        if self.velocity.y <= 0.0 {
            self.on_ground = self.collides_at(level, self.position.subtract_xyz(0.0, 0.001, 0.0));
        }

        self.yaw = self.yaw.rem_euclid(360.0);
    }

    fn move_with_collision(&mut self, level: &Level, delta: Vec3) {
        if delta.y != 0.0 {
            let collided = self.try_move_axis(level, delta.y, 1);
            if collided {
                self.velocity.y = 0.0;
                if delta.y < 0.0 {
                    self.on_ground = true;
                }
            } else if delta.y > 0.0 {
                self.on_ground = false;
            }
        }
        if delta.x != 0.0 && self.try_move_axis(level, delta.x, 0) {
            self.velocity.x = 0.0;
        }
        if delta.z != 0.0 && self.try_move_axis(level, delta.z, 2) {
            self.velocity.z = 0.0;
        }
    }

    /// 沿单轴以 0.05 方块步长移动；碰到碰撞体时停在最后合法位置。
    fn try_move_axis(&mut self, level: &Level, amount: f64, axis: usize) -> bool {
        const STEP: f64 = 0.05;
        let mut remaining = amount;
        while remaining.abs() > 1e-9 {
            let step = remaining.clamp(-STEP, STEP);
            let mut candidate = self.position;
            match axis {
                0 => candidate.x += step,
                1 => candidate.y += step,
                _ => candidate.z += step,
            }

            if self.collides_at(level, candidate) {
                return true;
            }
            self.position = candidate;
            remaining -= step;
        }
        false
    }

    fn collides_at(&self, level: &Level, position: Vec3) -> bool {
        let half_width = Self::WIDTH / 2.0;
        let aabb = AABB::new(
            position.x - half_width,
            position.y,
            position.z - half_width,
            position.x + half_width,
            position.y + Self::HEIGHT,
            position.z + half_width,
        );
        self.aabb_collides(level, &aabb)
    }

    fn aabb_collides(&self, level: &Level, aabb: &AABB) -> bool {
        let min_x = aabb.min_x.floor() as i32;
        let max_x = (aabb.max_x - 1.0e-7).floor() as i32;
        let min_y = aabb.min_y.floor() as i32;
        let max_y = (aabb.max_y - 1.0e-7).floor() as i32;
        let min_z = aabb.min_z.floor() as i32;
        let max_z = (aabb.max_z - 1.0e-7).floor() as i32;

        for x in min_x..=max_x {
            for y in min_y..=max_y {
                for z in min_z..=max_z {
                    let state = level.get_block_state(BlockPos::new(x, y, z));
                    if !state.has_collision() {
                        continue;
                    }
                    let block_box = AABB::new(
                        x as f64,
                        y as f64,
                        z as f64,
                        x as f64 + 1.0,
                        y as f64 + 1.0,
                        z as f64 + 1.0,
                    );
                    if aabb.intersects(&block_box) {
                        return true;
                    }
                }
            }
        }
        false
    }
}
