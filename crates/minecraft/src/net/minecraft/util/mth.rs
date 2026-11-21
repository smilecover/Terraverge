//! 对应 `net/minecraft/util/Mth.java` 的常用数学函数。
//!
//! 只移植当前已用到的部分；其余函数在 `docs/PROGRESS.md` 中登记为待移植。

pub struct Mth;

impl Mth {
    pub const PI: f32 = std::f32::consts::PI;
    pub const HALF_PI: f32 = std::f32::consts::FRAC_PI_2;
    pub const TWO_PI: f32 = std::f32::consts::TAU;
    pub const DEG_TO_RAD: f32 = Self::PI / 180.0;
    pub const RAD_TO_DEG: f32 = 180.0 / Self::PI;

    pub fn square(x: f64) -> f64 {
        x * x
    }

    /// 对应 `Mth.getSeed(Vec3i)` / `getSeed(x,y,z)`。
    /// 方块模型加权变体与随机朝向使用这个位置种子。
    pub const fn get_seed_xyz(x: i32, y: i32, z: i32) -> i64 {
        let seed = (x as i64) * 3_129_871 ^ (z as i64) * 116_129_781 ^ (y as i64);
        let seed = seed
            .wrapping_mul(seed)
            .wrapping_mul(42_317_861)
            .wrapping_add(seed.wrapping_mul(11));
        seed >> 16
    }

    pub const fn get_seed(pos: crate::net::minecraft::core::Vec3i) -> i64 {
        Self::get_seed_xyz(pos.get_x(), pos.get_y(), pos.get_z())
    }

    pub fn floor(x: f64) -> i32 {
        x.floor() as i32
    }

    pub fn ceil(x: f64) -> i32 {
        x.ceil() as i32
    }

    pub fn abs_max(a: i32, b: i32) -> i32 {
        let x = a.abs();
        let y = b.abs();
        x.max(y)
    }

    pub fn clamp(value: i32, min: i32, max: i32) -> i32 {
        value.max(min).min(max)
    }

    pub fn clamp_f32(value: f32, min: f32, max: f32) -> f32 {
        value.max(min).min(max)
    }

    pub fn clamp_f64(value: f64, min: f64, max: f64) -> f64 {
        value.max(min).min(max)
    }

    pub fn lerp(delta: f32, start: f32, end: f32) -> f32 {
        start + delta * (end - start)
    }

    pub fn lerp_f64(delta: f64, start: f64, end: f64) -> f64 {
        start + delta * (end - start)
    }

    pub fn lerp2(delta: f64, start: f64, end: f64) -> f64 {
        delta + (start - delta) * (1.0 - end)
    }

    /// 向下取整到 2 的幂。对应 `smallestEncompassingPowerOfTwo`。
    pub const fn smallest_encompassing_power_of_two(value: i32) -> i32 {
        let mut result = value - 1;
        result |= result >> 1;
        result |= result >> 2;
        result |= result >> 4;
        result |= result >> 8;
        result |= result >> 16;
        result + 1
    }

    /// 以 2 为底的对数，等价于 Java `Integer.numberOfLeadingZeros` 实现。
    pub const fn log2(value: i32) -> i32 {
        if value == 0 {
            0
        } else {
            31 - value.leading_zeros() as i32
        }
    }

    pub fn sin(x: f32) -> f32 {
        x.sin()
    }

    pub fn cos(x: f32) -> f32 {
        x.cos()
    }

    pub fn sqrt(x: f64) -> f64 {
        x.sqrt()
    }

    /// `b + (a - b) * t`，对应 Java `Mth.lerpDiscrete` 之外的 `lerp2`。
    pub fn inverse_lerp(value: f64, min: f64, max: f64) -> f64 {
        (value - min) / (max - min)
    }

    pub fn map(value: f64, from_min: f64, from_max: f64, to_min: f64, to_max: f64) -> f64 {
        Self::lerp_f64(
            Self::inverse_lerp(value, from_min, from_max),
            to_min,
            to_max,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn power_of_two_matches_java() {
        assert_eq!(
            Mth::smallest_encompassing_power_of_two(30_000_000),
            33_554_432
        );
        assert_eq!(Mth::log2(33_554_432), 25);
    }
}
