//! 游戏入口：创建 App 并交给窗口事件循环。

use crate::app::Game;
use render::initrender;

pub fn initgame() {
    initrender();
    println!("游戏初始化");

    if let Err(err) = window::run(Game::new()) {
        eprintln!("事件循环异常退出: {err}");
    }
}
