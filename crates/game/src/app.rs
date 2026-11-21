//! 游戏 App：把窗口输入、Minecraft 世界/玩家逻辑与 Vulkan 后端串起来。

use crate::assets::Assets;
use crate::scene::Scene;
use minecraft::net::minecraft::client::input::Input;
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
                button: MouseButton::Left,
                state: ElementState::Pressed,
            } => {
                // 焦点丢失会解除捕获，点击窗口时重新进入第一人称操作。
                if !self.cursor_grabbed {
                    self.grab_cursor(ctx, true);
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
