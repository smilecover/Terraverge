//! 对应 `net/minecraft/util/RandomSource.java` 与
//! `world/level/levelgen/BitRandomSource.java` 的默认方法。
//!
//! 具体实现见 `world/level/levelgen/LegacyRandomSource`（与 Java 路径一致）。
//! 目前只需要方块模型选择用到的接口；`nextGaussian`、位置化随机工厂等
//! 待实体/世界生成系统移植时再补。

pub trait RandomSource {
    fn set_seed(&mut self, seed: i64);

    /// 对应 `BitRandomSource.next(int bits)`，返回低 `bits` 位的随机数。
    fn next_bits(&mut self, bits: u32) -> i32;

    fn next_int(&mut self) -> i32 {
        self.next_bits(32)
    }

    /// 与 Java 完全一致的 `nextInt(bound)`（含拒绝采样）。
    fn next_int_bounded(&mut self, bound: i32) -> i32 {
        assert!(bound > 0, "Bound must be positive");
        if bound & (bound - 1) == 0 {
            return ((bound as i64 * self.next_bits(31) as i64) >> 31) as i32;
        }

        loop {
            let sample = self.next_bits(31);
            let modulo = sample % bound;
            if sample - modulo + (bound - 1) >= 0 {
                return modulo;
            }
        }
    }

    fn next_long(&mut self) -> i64 {
        let upper = self.next_bits(32);
        let lower = self.next_bits(32);
        ((upper as i64) << 32).wrapping_add(lower as i64)
    }

    fn next_boolean(&mut self) -> bool {
        self.next_bits(1) != 0
    }

    fn next_float(&mut self) -> f32 {
        self.next_bits(24) as f32 * 5.960_464_5e-8
    }

    fn next_double(&mut self) -> f64 {
        let upper = self.next_bits(26);
        let lower = self.next_bits(27);
        (((upper as i64) << 27) + lower as i64) as f64 * 1.110_223e-16
    }
}
