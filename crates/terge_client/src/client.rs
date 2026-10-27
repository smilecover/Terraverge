use terge_winit::{runner, ActiveEventLoop, App, EventLoopError, Window, WindowEvent};

use crate::input::Keybind;

pub struct RunGame;

impl RunGame {
    pub fn new() -> Self {
        Self
    }

    pub fn run(&self) -> Result<(), EventLoopError> {
        runner::run_game(Client::new())
    }
}

/// 客户端主状态：输入映射在这里，之后的游戏逻辑也挂在这里。
struct Client {
    input: Keybind,
}

impl Client {
    fn new() -> Self {
        Self {
            input: Keybind::new(),
        }
    }
}

impl App for Client {
    fn resumed(&mut self, _window: &Window) {}

    fn window_event(&mut self, event: &WindowEvent) {
        // 所有窗口事件先经过输入映射；游戏逻辑不直接接触物理键。
        self.input.handle_event(event);
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // 游戏逻辑在这里查询意图，例如：
        //   if self.input.just_pressed(Action::Jump) { ... }
        //   if self.input.held(Action::MoveForward) { ... }

        // 一帧结束：清空 just_pressed / just_released，held 保留。
        self.input.end_frame();
    }
}
