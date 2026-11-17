use crate::shader;
use render::*;
use std::path::Path;
use window::{App, Context, WindowConfig, WindowEvent};

struct Game {
    size: (u32, u32),
    render: Option<Render>,
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

        // 着色器由游戏侧编译：资源和入口点属于游戏，渲染后端只接收 SPIR-V。
        let shader_path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/shaders/simple_shader.slang");
        let compiled = match shader::compile(&shader_path) {
            Ok(compiled) => compiled,
            Err(err) => {
                eprintln!("着色器编译失败: {err}");
                return;
            }
        };

        let shaders = ShaderProgram {
            vertex: ShaderStage {
                code: &compiled.vertex,
                entry_point: shader::VERTEX_ENTRY_POINT,
            },
            fragment: ShaderStage {
                code: &compiled.fragment,
                entry_point: shader::FRAGMENT_ENTRY_POINT,
            },
        };

        match Render::new(
            display_handle,
            window_handle,
            self.size.0,
            self.size.1,
            shaders,
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
                if let Some(render) = &mut self.render
                    && let Err(err) = render.draw_frame()
                {
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

pub fn initgame() {
    initrender();
    println!("游戏初始化");

    let game = Game {
        size: (0, 0),
        render: None,
    };
    if let Err(err) = window::run(game) {
        eprintln!("事件循环异常退出: {err}");
    }
}
