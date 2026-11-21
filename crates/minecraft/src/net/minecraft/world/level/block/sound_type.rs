//! 对应 `net/minecraft/world/level/block/SoundType.java` 的子集。

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SoundType {
    #[default]
    STONE,
    GRASS,
    GRAVEL,
    WOOD,
    SAND,
    SNOW,
    GLASS,
}
