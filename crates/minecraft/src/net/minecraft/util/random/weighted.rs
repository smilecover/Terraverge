//! 对应 `net/minecraft/util/random/Weighted.java` 与
//! `WeightedList.java` 的选择语义。

use super::random_source::RandomSource;

#[derive(Clone, Debug, PartialEq)]
pub struct Weighted<T> {
    pub value: T,
    pub weight: i32,
}

impl<T> Weighted<T> {
    pub fn new(value: T, weight: i32) -> Self {
        assert!(weight >= 0, "Weight should be >= 0");
        Self { value, weight }
    }
}

/// 总权重小于 64 时 Java 使用扁平数组，否则使用累积权重；
/// 两种实现的抽取结果完全一致，这里统一用累积权重。
#[derive(Clone, Debug)]
pub struct WeightedList<T> {
    items: Vec<Weighted<T>>,
    total_weight: i32,
}

impl<T> WeightedList<T> {
    pub fn of(items: Vec<Weighted<T>>) -> Self {
        let total_weight = items.iter().map(|item| item.weight).sum();
        Self {
            items,
            total_weight,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_weight == 0
    }

    pub fn unwrap(&self) -> &[Weighted<T>] {
        &self.items
    }

    /// 对应 `getRandomOrThrow`。
    pub fn get_random_or_throw<R: RandomSource>(&self, random: &mut R) -> &T {
        assert!(!self.is_empty(), "Weighted list has no elements");
        let mut selection = random.next_int_bounded(self.total_weight);
        for item in &self.items {
            if selection < item.weight {
                return &item.value;
            }
            selection -= item.weight;
        }
        unreachable!("随机数超出总权重")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::minecraft::world::level::levelgen::LegacyRandomSource;

    #[test]
    fn weighted_selection_follows_flat_table() {
        let list = WeightedList::of(vec![
            Weighted::new("a", 1),
            Weighted::new("b", 2),
            Weighted::new("c", 1),
        ]);
        let mut random = LegacyRandomSource::new(0);
        // new Random(0).nextInt(4) = 2，落在第二项（权重 2）上。
        assert_eq!(*list.get_random_or_throw(&mut random), "b");
    }
}
