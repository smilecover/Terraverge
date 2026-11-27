use window::{Game, GameWindows, WindowConfig, WindowEvent};

/// 游戏应用状态；渲染后端接入前只记录窗口尺寸。
pub(crate) struct RunGame {
    size: (u32, u32),
}

impl RunGame {
    pub(crate) fn new() -> Self {
        Self { size: (0, 0) }
    }
}

impl Game for RunGame {
    fn window_config(&self) -> WindowConfig {
        WindowConfig::default()
    }

    fn resumed(&mut self, ctx: &GameWindows<'_>) {
        self.size = ctx.size();
        println!("窗口已创建: {}x{}", self.size.0, self.size.1);
    }

    fn window_event(&mut self, ctx: &GameWindows<'_>, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => ctx.exit(),
            WindowEvent::Resized { width, height } => {
                self.size = (width, height);
            }
            WindowEvent::RedrawRequested => {
                // 预留给渲染后端：Vulkan 交换链绘制尚未接入。
            }
            WindowEvent::Focused(_) | WindowEvent::ScaleFactorChanged { .. } => {}
        }
    }

    fn about_to_wait(&mut self, ctx: &GameWindows<'_>) {
        // 最小化时窗口尺寸为 0，暂停重绘避免事件循环空转。
        if self.size.0 > 0 && self.size.1 > 0 {
            ctx.request_redraw();
        }
    }

    fn exiting(&mut self, _ctx: &GameWindows<'_>) {
        println!("窗口已关闭");
    }
}
