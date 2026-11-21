//! 对应 `net/minecraft/world/level/levelgen/LegacyRandomSource.java`。
//!
//! Java 的 `java.util.Random` 使用 48 位线性同余发生器；方块模型加权
//! 变体、位置种子等行为依赖它，必须逐位一致。

use crate::net::minecraft::util::random::RandomSource;

#[derive(Clone, Debug)]
pub struct LegacyRandomSource {
    seed: i64,
}

impl LegacyRandomSource {
    const MULTIPLIER: i64 = 25_214_903_917;
    const INCREMENT: i64 = 11;
    const MASK: i64 = 281_474_976_710_655;

    pub fn new(seed: i64) -> Self {
        let mut random = Self { seed: 0 };
        random.set_seed(seed);
        random
    }
}

impl RandomSource for LegacyRandomSource {
    fn set_seed(&mut self, seed: i64) {
        self.seed = (seed ^ Self::MULTIPLIER) & Self::MASK;
    }

    fn next_bits(&mut self, bits: u32) -> i32 {
        self.seed = self
            .seed
            .wrapping_mul(Self::MULTIPLIER)
            .wrapping_add(Self::INCREMENT)
            & Self::MASK;
        (self.seed >> (48 - bits)) as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_random_matches_java_util_random() {
        // 期望值来自 JDK: new Random(42).nextInt()/nextInt(100)/nextLong()
        let mut random = LegacyRandomSource::new(42);
        assert_eq!(random.next_int(), -1_170_105_035);
        assert_eq!(random.next_int_bounded(100), 63);
        assert_eq!(random.next_long(), -5_843_495_416_241_995_736);

        let mut random = LegacyRandomSource::new(0);
        assert_eq!(random.next_int(), -1_155_484_576);
        assert_eq!(random.next_int_bounded(100), 48);
        assert_eq!(random.next_int_bounded(4), 0);
    }

    #[test]
    fn next_int_bounded_wraps_like_java() {
        let mut random = LegacyRandomSource::new(12345);
        for _ in 0..1000 {
            let value = random.next_int_bounded(7);
            assert!((0..7).contains(&value));
        }
    }
}
