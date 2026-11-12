use render::Render;
use window::{Context, WindowConfig, WindowEvent};

#[derive(Default)]
pub struct Tever {
    render: Option<Render>,
}

impl window::App for Tever {
    fn window_config(&self) -> WindowConfig {
        WindowConfig::new("Terraverge", 1280, 720)
    }

    fn resumed(&mut self, ctx: &Context) {
        let display_handle = ctx.display_handle().expect("获取显示句柄失败");
        let window_handle = ctx.window_handle().expect("获取窗口句柄失败");
        let (width, height) = ctx.size();
        
    }

    fn window_event(&mut self, ctx: &Context, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => ctx.exit(),
            WindowEvent::Resized(size) => {}
            WindowEvent::RedrawRequested => {}
            _ => {}
        }
    }

    fn about_to_wait(&mut self, ctx: &Context) {
        if self.render.is_some() {
            ctx.request_redraw();
        }
    }

    fn exiting(&mut self, _ctx: &Context) {
        self.render = None;
    }
}

pub fn initgame() -> Result<(), window::EventLoopError> {
    window::run(Tever::default())
}
