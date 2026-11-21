//! 对应 `net/minecraft/world/level/block/RenderShape.java`。

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderShape {
    /// `INHERIT`：沿用方块实体的渲染；普通方块不用。
    INHERIT,
    /// `MODEL`：由方块模型渲染。
    MODEL,
    /// `INVISIBLE`：不渲染（空气）。
    INVISIBLE,
}
