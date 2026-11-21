//! 对应 `net/minecraft/world/level/material/MapColor.java` 的子集。

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MapColor {
    #[default]
    NONE,
    GRASS,
    DIRT,
    STONE,
    WOOD,
    SAND,
    WATER,
    PLANT,
    SNOW,
}
