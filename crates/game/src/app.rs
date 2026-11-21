//! 游戏 App：把窗口输入、Minecraft 世界/玩家逻辑与 Vulkan 后端串起来。

use crate::assets::Assets;
use crate::scene::Scene;
use minecraft::net::minecraft::client::input::Input;
use minecraft::net::minecraft::core::{BlockPos, Direction};
use minecraft::net::minecraft::world::level::Level;
use minecraft::net::minecraft::world::level::block::Blocks;
use minecraft::net::minecraft::world::phys::{AABB, Vec3};
use render::Render;
use std::collections::HashSet;
use std::time::Instant;
use window::{App, Context, ElementState, KeyCode, MouseButton, WindowConfig, WindowEvent};

/// 每帧最大时间步，避免窗口拖动/断点后玩家瞬移穿过地形。
const MAX_FRAME_DELTA: f32 = 0.05;
/// 鼠标灵敏度（度/像素）。原版默认约 0.15。
const MOUSE_SENSITIVITY: f64 = 0.15;

pub struct Game {
    size: (u32, u32),
    render: Option<Render>,
    assets: Option<Assets>,
    scene: Option<Scene>,
    keys: HashSet<KeyCode>,
    cursor_grabbed: bool,
    last_frame: Instant,
    /// 调试放置槽位：1=石头、2=草方块、3=橡木木板、4=橡木树叶、5=玻璃。
    selected_block_slot: usize,
}

impl Game {
    pub fn new() -> Self {
        Self {
            size: (0, 0),
            render: None,
            assets: None,
            scene: None,
            keys: HashSet::new(),
            cursor_grabbed: false,
            last_frame: Instant::now(),
            selected_block_slot: 0,
        }
    }

    fn selected_block_state(
        &self,
    ) -> minecraft::net::minecraft::world::level::block::state::BlockState {
        match self.selected_block_slot {
            1 => Blocks::grass_block().default_block_state(),
            2 => Blocks::oak_planks().default_block_state(),
            3 => Blocks::oak_leaves().default_block_state(),
            4 => Blocks::glass().default_block_state(),
            _ => Blocks::stone().default_block_state(),
        }
    }

    fn input(&self) -> Input {
        Input {
            forward: self.keys.contains(&KeyCode::KeyW),
            backward: self.keys.contains(&KeyCode::KeyS),
            left: self.keys.contains(&KeyCode::KeyA),
            right: self.keys.contains(&KeyCode::KeyD),
            jumping: self.keys.contains(&KeyCode::Space),
            shift: self.keys.contains(&KeyCode::ShiftLeft)
                || self.keys.contains(&KeyCode::ShiftRight),
            sprinting: self.keys.contains(&KeyCode::ControlLeft)
                || self.keys.contains(&KeyCode::ControlRight),
        }
    }

    fn grab_cursor(&mut self, ctx: &Context<'_>, grab: bool) {
        match ctx.set_cursor_grab(grab) {
            Ok(()) => {
                self.cursor_grabbed = grab;
                ctx.set_cursor_visible(!grab);
            }
            Err(err) => {
                eprintln!("切换鼠标捕获失败: {err}");
                self.cursor_grabbed = false;
            }
        }
    }

    /// 临时调试交互：从眼睛沿视线做体素 DDA，返回命中方块与进入面。
    ///
    /// 对应 26.3 的 `BlockGetter.clip(ClipContext)`/`MultiPlayerGameMode`，
    /// 但当前只服务于网格热更新验证；M3 会替换为正式移植。
    fn raycast(
        level: &Level,
        origin: Vec3,
        direction: Vec3,
        max_distance: f64,
    ) -> Option<(BlockPos, Direction)> {
        let dir = direction.normalize();
        if dir.length_sqr() < 1.0e-9 {
            return None;
        }

        let mut x = origin.x.floor() as i32;
        let mut y = origin.y.floor() as i32;
        let mut z = origin.z.floor() as i32;

        let step_x = dir.x.signum() as i32;
        let step_y = dir.y.signum() as i32;
        let step_z = dir.z.signum() as i32;

        let delta_x = if step_x != 0 {
            (1.0 / dir.x).abs()
        } else {
            f64::INFINITY
        };
        let delta_y = if step_y != 0 {
            (1.0 / dir.y).abs()
        } else {
            f64::INFINITY
        };
        let delta_z = if step_z != 0 {
            (1.0 / dir.z).abs()
        } else {
            f64::INFINITY
        };

        let mut max_x = if step_x > 0 {
            (x as f64 + 1.0 - origin.x) / dir.x
        } else if step_x < 0 {
            (origin.x - x as f64) / -dir.x
        } else {
            f64::INFINITY
        };
        let mut max_y = if step_y > 0 {
            (y as f64 + 1.0 - origin.y) / dir.y
        } else if step_y < 0 {
            (origin.y - y as f64) / -dir.y
        } else {
            f64::INFINITY
        };
        let mut max_z = if step_z > 0 {
            (z as f64 + 1.0 - origin.z) / dir.z
        } else if step_z < 0 {
            (origin.z - z as f64) / -dir.z
        } else {
            f64::INFINITY
        };

        let mut face = Direction::UP;
        let mut travelled = 0.0f64;
        while travelled <= max_distance {
            let pos = BlockPos::new(x, y, z);
            if level.is_inside_build_height(y) {
                let state = level.get_block_state(pos);
                if !state.is_air() && state.has_collision() {
                    return Some((pos, face));
                }
            }

            if max_x < max_y && max_x < max_z {
                travelled = max_x;
                x += step_x;
                max_x += delta_x;
                face = if step_x > 0 {
                    Direction::WEST
                } else {
                    Direction::EAST
                };
            } else if max_y < max_z {
                travelled = max_y;
                y += step_y;
                max_y += delta_y;
                face = if step_y > 0 {
                    Direction::DOWN
                } else {
                    Direction::UP
                };
            } else {
                travelled = max_z;
                z += step_z;
                max_z += delta_z;
                face = if step_z > 0 {
                    Direction::NORTH
                } else {
                    Direction::SOUTH
                };
            }
        }

        None
    }

    fn intersects_player(player_box: &AABB, pos: BlockPos) -> bool {
        let block_box = AABB::new(
            pos.get_x() as f64,
            pos.get_y() as f64,
            pos.get_z() as f64,
            pos.get_x() as f64 + 1.0,
            pos.get_y() as f64 + 1.0,
            pos.get_z() as f64 + 1.0,
        );
        player_box.intersects(&block_box)
    }

    fn apply_block_edit(&mut self, break_block: bool) {
        let Some(scene) = &self.scene else {
            return;
        };
        let eye = scene.eye_position();
        let look = scene.look_vector();
        let player_box = scene.player().get_bounding_box();
        let placement_state = self.selected_block_state();

        let (Some(assets), Some(render)) = (&mut self.assets, &mut self.render) else {
            return;
        };

        let level = assets.level_mut();
        let Some((hit_pos, face)) = Self::raycast(level, eye, look, 5.0) else {
            return;
        };

        let (target, state) = if break_block {
            (hit_pos, Blocks::air().default_block_state())
        } else {
            let target = hit_pos.relative(face);
            if !level.get_block_state(target).is_air()
                || Self::intersects_player(&player_box, target)
                || !level.is_inside_build_height(target.get_y())
            {
                return;
            }
            (target, placement_state)
        };

        let block_name = state.get_block_name().clone();
        level.set_block_state(target, state);
        assets.rebuild_mesh();
        let quads = assets.quad_count();
        if let Err(err) = render.update_mesh(assets.mesh()) {
            eprintln!("更新世界网格失败: {err}");
        } else {
            println!(
                "{} {} {} -> {} 四边形",
                if break_block { "破坏" } else { "放置" },
                block_name,
                target,
                quads
            );
        }
    }
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

impl App for Game {
    fn window_config(&self) -> WindowConfig {
        WindowConfig::with_size("Terraverge 26.3", self.size)
    }

    fn resumed(&mut self, ctx: &Context<'_>) {
        if self.render.is_some() {
            return;
        }
        self.size = ctx.size();

        let (display_handle, window_handle) = match (ctx.display_handle(), ctx.window_handle()) {
            (Ok(display), Ok(window)) => (display, window),
            (Err(err), _) | (_, Err(err)) => {
                eprintln!("获取窗口句柄失败: {err}");
                ctx.exit();
                return;
            }
        };

        let assets = match Assets::load() {
            Ok(assets) => assets,
            Err(err) => {
                eprintln!("资源加载失败: {err}");
                ctx.exit();
                return;
            }
        };
        let scene = Scene::new(assets.spawn_position());

        match Render::new(
            display_handle,
            window_handle,
            self.size.0,
            self.size.1,
            assets.shaders(),
            assets.mesh(),
            assets.texture(),
        ) {
            Ok(mut render) => {
                // 教程的粒子系统不属于 Minecraft 客户端画面。
                render.set_particles_enabled(false);
                self.render = Some(render);
                self.assets = Some(assets);
                self.scene = Some(scene);
                self.last_frame = Instant::now();
                self.grab_cursor(ctx, true);
            }
            Err(err) => {
                eprintln!("渲染器初始化失败: {err}");
                ctx.exit();
            }
        }

        println!("窗口已创建: {}x{}", self.size.0, self.size.1);
    }

    fn about_to_wait(&mut self, ctx: &Context<'_>) {
        let (width, height) = ctx.size();
        if width > 0 && height > 0 && self.render.is_some() {
            ctx.request_redraw();
        }
    }

    fn exiting(&mut self, _ctx: &Context<'_>) {
        self.render = None;
        self.assets = None;
        self.scene = None;
    }

    fn window_event(&mut self, ctx: &Context<'_>, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => ctx.exit(),
            WindowEvent::Resized { width, height } => {
                self.size = (width, height);
                if let Some(render) = &mut self.render {
                    render.resize(width, height);
                }
            }
            WindowEvent::KeyboardInput { key, state, repeat } => {
                if repeat {
                    return;
                }
                match state {
                    ElementState::Pressed => {
                        if key == KeyCode::Escape {
                            ctx.exit();
                            return;
                        }
                        match key {
                            KeyCode::Digit1 => self.selected_block_slot = 0,
                            KeyCode::Digit2 => self.selected_block_slot = 1,
                            KeyCode::Digit3 => self.selected_block_slot = 2,
                            KeyCode::Digit4 => self.selected_block_slot = 3,
                            KeyCode::Digit5 => self.selected_block_slot = 4,
                            _ => {}
                        }
                        self.keys.insert(key);
                    }
                    ElementState::Released => {
                        self.keys.remove(&key);
                    }
                }
            }
            WindowEvent::MouseMotion { delta_x, delta_y } => {
                if self.cursor_grabbed {
                    if let Some(scene) = &mut self.scene {
                        scene.turn(delta_x, delta_y, MOUSE_SENSITIVITY);
                    }
                }
            }
            WindowEvent::MouseButton {
                button,
                state: ElementState::Pressed,
            } => {
                // 焦点丢失会解除捕获，点击窗口时重新进入第一人称操作。
                if !self.cursor_grabbed {
                    if button == MouseButton::Left {
                        self.grab_cursor(ctx, true);
                    }
                    return;
                }
                match button {
                    MouseButton::Left => self.apply_block_edit(true),
                    MouseButton::Right => self.apply_block_edit(false),
                    _ => {}
                }
            }
            WindowEvent::Focused(false) => {
                self.keys.clear();
                if self.cursor_grabbed {
                    self.cursor_grabbed = false;
                    let _ = ctx.set_cursor_grab(false);
                    ctx.set_cursor_visible(true);
                }
            }
            WindowEvent::Focused(true) => {
                if self.render.is_some() && !self.cursor_grabbed {
                    self.grab_cursor(ctx, true);
                }
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = (now - self.last_frame).as_secs_f32().min(MAX_FRAME_DELTA);
                self.last_frame = now;

                let input = self.input();
                let (width, height) = self.size;
                if width == 0 || height == 0 {
                    return;
                }

                let (Some(assets), Some(scene), Some(render)) =
                    (&self.assets, &mut self.scene, &mut self.render)
                else {
                    return;
                };

                scene.update(assets.level(), &input, dt);
                let uniforms = scene.frame(width as f32 / height as f32);
                if let Err(err) = render.draw_frame(&uniforms, dt * 1000.0) {
                    eprintln!("绘制失败: {err}");
                    ctx.exit();
                }
            }
            _ => {}
        }
    }
}
