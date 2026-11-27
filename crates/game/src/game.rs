//! 游戏入口：当前阶段只负责创建并显示窗口、响应基础窗口事件。
use crate::app::RunGame;
use render::initrender;
use window::run;

pub fn initgame() {
    initrender();
    if let Err(err) = run(RunGame::new()) {
        eprintln!("事件循环异常退出: {err}");
    }
}
