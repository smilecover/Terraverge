//! 对应 `net/minecraft/core/Position.java`：三维坐标的只读视图。

pub trait Position {
    fn x(&self) -> f64;
    fn y(&self) -> f64;
    fn z(&self) -> f64;
}
