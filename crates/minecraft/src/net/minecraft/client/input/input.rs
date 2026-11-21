//! 对应 `net/minecraft/client/input/Input.java`（客户端输入状态）。

#[derive(Clone, Copy, Debug, Default)]
pub struct Input {
    pub forward: bool,
    pub backward: bool,
    pub left: bool,
    pub right: bool,
    pub jumping: bool,
    pub shift: bool,
    pub sprinting: bool,
}

impl Input {
    /// `zza`：前进为正，后退为负。
    pub fn move_forward(&self) -> f32 {
        (self.forward as i32 - self.backward as i32) as f32
    }

    /// `xxa`：左为正，右为负（与世界坐标中的左向量对应）。
    pub fn move_strafe(&self) -> f32 {
        (self.left as i32 - self.right as i32) as f32
    }

    pub fn is_moving(&self) -> bool {
        self.forward || self.backward || self.left || self.right
    }

    pub fn has_shift_key_down(&self) -> bool {
        self.shift
    }
}
