//! 游戏 App：实现窗口事件循环回调，把场景逻辑与渲染后端串起来。

use crate::{assets::Assets, scene::Scene};
use render::Render;
use window::{App, Context, WindowConfig, WindowEvent};

pub struct Game {
    /// 最近一次窗口尺寸；投影矩阵和交换链重建都依赖它。
    size: (u32, u32),
    /// 相机、动画与每帧 uniform。
    scene: Scene,
    render: Option<Render>,
}

impl Game {
    pub fn new() -> Self {
        Self {
            size: (0, 0),
            scene: Scene::new(),
            render: None,
        }
    }
}

impl App for Game {
    fn window_config(&self) -> WindowConfig {
        WindowConfig::with_size("Terraverge", self.size)
    }

    fn resumed(&mut self, ctx: &Context<'_>) {
        self.size = ctx.size();

        let (display_handle, window_handle) = match (ctx.display_handle(), ctx.window_handle()) {
            (Ok(display), Ok(window)) => (display, window),
            (Err(err), _) | (_, Err(err)) => {
                eprintln!("获取窗口句柄失败: {err}");
                return;
            }
        };

        // 着色器、模型和纹理都由游戏层加载，渲染后端只接收 CPU 数据。
        let assets = match Assets::load() {
            Ok(assets) => assets,
            Err(err) => {
                eprintln!("资源加载失败: {err}");
                return;
            }
        };

        match Render::new(
            display_handle,
            window_handle,
            self.size.0,
            self.size.1,
            assets.shaders(),
            assets.mesh(),
            assets.texture(),
        ) {
            Ok(render) => self.render = Some(render),
            Err(err) => eprintln!("渲染器初始化失败: {err}"),
        }

        println!("窗口已创建: {}x{}", self.size.0, self.size.1);
    }

    fn about_to_wait(&mut self, ctx: &Context<'_>) {
        // 最小化时暂停重绘，避免事件循环空转；窗口恢复后 winit 会再次触发
        // Resized / RedrawRequested，渲染随之继续。
        let (width, height) = ctx.size();
        if width > 0 && height > 0 {
            ctx.request_redraw();
        }
    }

    fn exiting(&mut self, _ctx: &Context<'_>) {
        self.render = None;
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
            WindowEvent::RedrawRequested => {
                let Some(render) = &mut self.render else {
                    return;
                };
                let (width, height) = self.size;
                if width == 0 || height == 0 {
                    return;
                }

                // 场景逻辑属于游戏层：算好 MVP 和粒子 deltaTime 再交给渲染后端。
                let (uniforms, delta_time) = self.scene.frame(width as f32 / height as f32);
                if let Err(err) = render.draw_frame(&uniforms, delta_time) {
                    // 交换链的 out-of-date/suboptimal 已在绘制内部处理；
                    // 能走到这里的是无法在原地恢复的错误（设备丢失、交换链格式变化等）。
                    eprintln!("绘制失败: {err}");
                    ctx.exit();
                }
            }
            _ => {}
        }
    }
}
