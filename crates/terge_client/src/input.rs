//! 输入映射：物理键 → 游戏动作 → 每帧查询。
//!
//! 核心是不让游戏逻辑直接接触物理键：
//!
//! ```text
//! WindowEvent::KeyboardInput
//!         │  Keybind::handle_event
//!         ▼
//!   KeyCode ──bindings──▶ Action
//!         │
//!         ▼
//!   InputState（just_pressed / held / just_released）
//!         │
//!         ▼
//!   if input.just_pressed(Action::Jump) { ... }
//! ```
//!
//! 这样改键只动绑定表，游戏逻辑一行不用改。
//!
//! ```ignore
//! fn update(input: &Keybind) {
//!     if input.just_pressed(Action::Jump) { /* 起跳 */ }
//!     if input.held(Action::MoveForward) { /* 前进 */ }
//! }
//! ```

use std::collections::{HashMap, HashSet};

use terge_winit::{ElementState, KeyCode, PhysicalKey, WindowEvent};

/// 游戏动作：玩家“想做什么”，而不是“按了哪个键”。
///
/// 新功能通常只需要加一个变体，并在 [`DEFAULT_BINDINGS`] 里给一个默认键。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Action {
    /// 前进
    MoveForward,
    /// 后退
    MoveBackward,
    /// 左移
    MoveLeft,
    /// 右移
    MoveRight,
    /// 跳跃
    Jump,
    /// 加速
    Sprint,
    /// 交互
    Interact,
    /// 打开背包
    OpenInventory,
    /// 暂停
    Pause,
    /// 调试信息
    ToggleDebug,
}

/// 默认绑定：同一个动作可以配多个物理键（WASD 与方向键同时可用）。
const DEFAULT_BINDINGS: &[(KeyCode, Action)] = &[
    (KeyCode::KeyW, Action::MoveForward),
    (KeyCode::ArrowUp, Action::MoveForward),
    (KeyCode::KeyS, Action::MoveBackward),
    (KeyCode::ArrowDown, Action::MoveBackward),
    (KeyCode::KeyA, Action::MoveLeft),
    (KeyCode::ArrowLeft, Action::MoveLeft),
    (KeyCode::KeyD, Action::MoveRight),
    (KeyCode::ArrowRight, Action::MoveRight),
    (KeyCode::Space, Action::Jump),
    (KeyCode::ShiftLeft, Action::Sprint),
    (KeyCode::KeyE, Action::Interact),
    (KeyCode::Tab, Action::OpenInventory),
    (KeyCode::Escape, Action::Pause),
    (KeyCode::F3, Action::ToggleDebug),
];

/// 按键映射 + 当前输入状态。
///
/// 每帧游戏逻辑结束后调用 [`Keybind::end_frame`]，清空边沿状态。
#[derive(Debug)]
pub struct Keybind {
    /// 物理键 → 动作。一个键只对应一个动作，一个动作可以对应多个键。
    bindings: HashMap<KeyCode, Action>,
    state: InputState,
}

impl Keybind {
    /// 创建一份带默认绑定的映射。
    pub fn new() -> Self {
        Self {
            bindings: DEFAULT_BINDINGS.iter().copied().collect(),
            state: InputState::default(),
        }
    }

    /// 把物理键绑定到动作，返回该键原先绑定的动作。
    ///
    /// 一个键只能绑定一个动作（后绑覆盖先绑）；同一动作可以绑定多个键。
    pub fn bind(&mut self, key: KeyCode, action: Action) -> Option<Action> {
        // 按下过程中改绑定，旧的按住计数已经不可信，直接清空。
        self.state.clear();
        self.bindings.insert(key, action)
    }

    /// 解除某个物理键的绑定。
    pub fn unbind(&mut self, key: KeyCode) -> Option<Action> {
        self.state.clear();
        self.bindings.remove(&key)
    }

    /// 查询物理键当前绑定的动作。
    pub fn action_for(&self, key: KeyCode) -> Option<Action> {
        self.bindings.get(&key).copied()
    }

    /// 查询某个动作绑定的所有物理键（用于设置界面显示）。
    pub fn keys_for(&self, action: Action) -> impl Iterator<Item = KeyCode> + '_ {
        self.bindings
            .iter()
            .filter(move |&(_, &bound)| bound == action)
            .map(|(&key, _)| key)
    }

    /// 处理一个窗口事件。
    pub fn handle_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput { event, .. } => {
                // 系统自动重复不是新的“按下”，按住状态已经记录过。
                if event.repeat {
                    return;
                }
                // 只认物理键：与键盘布局无关，AZERTY / 输入法下 WASD 仍在原位。
                if let PhysicalKey::Code(key) = &event.physical_key {
                    self.handle_key(*key, event.state);
                }
            }
            // 失去焦点后系统不会再发 Released。不清空的话，角色会一直向前走。
            WindowEvent::Focused(false) => self.state.clear(),
            _ => {}
        }
    }

    /// 处理一次物理键的按下 / 松开（不经过 winit 事件，便于测试或脚本驱动）。
    pub fn handle_key(&mut self, key: KeyCode, state: ElementState) {
        let Some(&action) = self.bindings.get(&key) else {
            return;
        };

        match state {
            ElementState::Pressed => self.state.press(action),
            ElementState::Released => self.state.release(action),
        }
    }

    /// 本帧刚按下（边沿触发）：适合跳跃、开背包、暂停。
    #[must_use]
    pub fn just_pressed(&self, action: Action) -> bool {
        self.state.pressed.contains(&action)
    }

    /// 当前按住（电平触发）：适合移动、持续开火。
    #[must_use]
    pub fn held(&self, action: Action) -> bool {
        self.state.held.contains_key(&action)
    }

    /// 本帧刚松开：适合蓄力释放等。
    #[must_use]
    pub fn just_released(&self, action: Action) -> bool {
        self.state.released.contains(&action)
    }

    /// 一帧结束时调用，清空边沿状态。
    ///
    /// `held` 不清空：按住是跨帧的持续状态。
    pub fn end_frame(&mut self) {
        self.state.pressed.clear();
        self.state.released.clear();
    }

    /// 清空全部输入状态（切换场景 / 打开模态 UI 时调用）。
    pub fn clear(&mut self) {
        self.state.clear();
    }
}

impl Default for Keybind {
    fn default() -> Self {
        Self::new()
    }
}

/// 当前输入状态。
///
/// | 查询 | 含义 | 典型用途 |
/// |---|---|---|
/// | `just_pressed` | 本帧刚按下 | 跳跃、开背包 |
/// | `held` | 当前按住 | 移动、持续开火 |
/// | `just_released` | 本帧刚松开 | 蓄力释放 |
#[derive(Debug, Default)]
struct InputState {
    /// 动作 → 当前按住的物理键数量。
    ///
    /// 用计数而不是 bool：W 和 ↑ 同时绑到前进时，松开其中一个不应触发 just_released。
    held: HashMap<Action, u32>,
    pressed: HashSet<Action>,
    released: HashSet<Action>,
}

impl InputState {
    fn press(&mut self, action: Action) {
        let count = self.held.entry(action).or_insert(0);
        if *count == 0 {
            self.pressed.insert(action);
        }
        *count += 1;
    }

    fn release(&mut self, action: Action) {
        let Some(count) = self.held.get_mut(&action) else {
            return;
        };
        *count -= 1;
        if *count == 0 {
            self.held.remove(&action);
            self.released.insert(action);
        }
    }

    fn clear(&mut self) {
        self.held.clear();
        self.pressed.clear();
        self.released.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use terge_winit::ElementState;

    #[test]
    fn press_and_release_edges() {
        let mut input = Keybind::new();

        input.handle_key(KeyCode::Space, ElementState::Pressed);
        assert!(input.just_pressed(Action::Jump));
        assert!(input.held(Action::Jump));

        input.end_frame();
        assert!(!input.just_pressed(Action::Jump));
        assert!(input.held(Action::Jump));

        input.handle_key(KeyCode::Space, ElementState::Released);
        assert!(input.just_released(Action::Jump));
        assert!(!input.held(Action::Jump));
    }

    #[test]
    fn two_keys_bound_to_one_action() {
        let mut input = Keybind::new();

        input.handle_key(KeyCode::KeyW, ElementState::Pressed);
        input.handle_key(KeyCode::ArrowUp, ElementState::Pressed);
        input.end_frame();

        // 松开其中一个键：动作仍按住，也不产生“刚松开”。
        input.handle_key(KeyCode::KeyW, ElementState::Released);
        assert!(input.held(Action::MoveForward));
        assert!(!input.just_released(Action::MoveForward));

        input.handle_key(KeyCode::ArrowUp, ElementState::Released);
        assert!(!input.held(Action::MoveForward));
        assert!(input.just_released(Action::MoveForward));
    }

    #[test]
    fn bind_adds_alternative_key() {
        let mut input = Keybind::new();

        assert_eq!(input.bind(KeyCode::KeyJ, Action::Jump), None);
        assert_eq!(input.action_for(KeyCode::KeyJ), Some(Action::Jump));
        assert!(input.keys_for(Action::Jump).any(|key| key == KeyCode::Space));
    }
}
