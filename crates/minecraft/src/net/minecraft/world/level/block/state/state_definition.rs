//! 对应 `net/minecraft/world/level/block/state/StateDefinition.java`。
//!
//! 负责把一组属性展开成全部方块状态，并承担“属性名 -> 值”的查询。
//! 状态 ID 是块内下标，`BlockState` 的身份由 `(block_id, state_index)` 唯一确定。

use super::BlockState;
use super::properties::{Property, PropertyValue};

pub struct StateDefinition {
    properties: Vec<Property>,
    states: Vec<BlockState>,
    /// 与 `states` 等长的属性值向量，顺序与 `properties` 一致。
    state_values: Vec<Vec<PropertyValue>>,
    default_state: BlockState,
}

impl StateDefinition {
    /// 构造一个块的全部状态。`block_id` 由注册表在注册时分配。
    pub fn create(block_id: u16, properties: Vec<Property>) -> Self {
        let state_count: usize = properties
            .iter()
            .map(|property| property.get_possible_values().len())
            .product::<usize>()
            .max(1);

        assert!(
            state_count <= u16::MAX as usize,
            "方块 {} 的状态数量 {} 超过 u16 上限",
            block_id,
            state_count
        );

        // 笛卡尔积。属性顺序固定，保证每次启动状态 ID 一致。
        let mut combinations: Vec<Vec<PropertyValue>> = vec![Vec::new()];
        for property in &properties {
            let values = property.get_possible_values();
            let mut next = Vec::with_capacity(combinations.len() * values.len());
            for combination in &combinations {
                for value in &values {
                    let mut values = combination.clone();
                    values.push(*value);
                    next.push(values);
                }
            }
            combinations = next;
        }

        let default_index = combinations
            .iter()
            .position(|values| {
                values
                    .iter()
                    .zip(properties.iter())
                    .all(|(&value, property)| value == property.get_default_value())
            })
            .unwrap_or(0);

        let states = combinations
            .iter()
            .enumerate()
            .map(|(index, _)| BlockState::new(block_id, index as u16))
            .collect();
        let default_state = BlockState::new(block_id, default_index as u16);

        Self {
            properties,
            states,
            state_values: combinations,
            default_state,
        }
    }

    pub fn get_properties(&self) -> &[Property] {
        &self.properties
    }

    pub fn get_property(&self, name: &str) -> Option<&Property> {
        self.properties
            .iter()
            .find(|property| property.get_name() == name)
    }

    pub fn get_possible_states(&self) -> &[BlockState] {
        &self.states
    }

    pub fn get_default_state(&self) -> BlockState {
        self.default_state
    }

    pub fn state_values(&self, state_index: u16) -> &[PropertyValue] {
        &self.state_values[state_index as usize]
    }

    pub fn get_value(&self, state_index: u16, property_name: &str) -> Option<PropertyValue> {
        let property_index = self
            .properties
            .iter()
            .position(|property| property.get_name() == property_name)?;
        self.state_values[state_index as usize]
            .get(property_index)
            .copied()
    }

    /// 生成 `name=value,name=value` 键。属性按定义顺序排列；
    /// blockstate 匹配按“属性名-值”集合比较，不依赖字符串顺序。
    pub fn state_key(&self, state_index: u16) -> String {
        let values = &self.state_values[state_index as usize];
        self.properties
            .iter()
            .zip(values)
            .map(|(property, value)| format!("{}={}", property.get_name(), value))
            .collect::<Vec<_>>()
            .join(",")
    }

    /// 对应 `VariantSelector.predicate`：selector 是 `a=b,c=d`。
    pub fn matches_selector(&self, state_index: u16, selector: &str) -> bool {
        if selector.is_empty() {
            return true;
        }
        for entry in selector.split(',') {
            let mut parts = entry.splitn(2, '=');
            let Some(name) = parts.next() else {
                return false;
            };
            let Some(raw_value) = parts.next() else {
                return false;
            };
            let Some(property) = self.get_property(name) else {
                return false;
            };
            let Some(expected) = property.parse_value(raw_value) else {
                return false;
            };
            if self.get_value(state_index, name) != Some(expected) {
                return false;
            }
        }
        true
    }

    /// 对应 `setValue`：返回包含新值的状态。
    pub fn get_state_with(
        &self,
        state: BlockState,
        property: &Property,
        value: PropertyValue,
    ) -> Option<BlockState> {
        if !property.is_valid(value) {
            return None;
        }
        let property_index = self
            .properties
            .iter()
            .position(|candidate| candidate.get_name() == property.get_name())?;
        let mut values = self.state_values[state.get_state_index() as usize].clone();
        if values[property_index] == value {
            return Some(state);
        }
        values[property_index] = value;
        self.state_values
            .iter()
            .position(|candidate| candidate == &values)
            .map(|index| self.states[index])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_properties_to_cartesian_states() {
        let definition = StateDefinition::create(
            0,
            vec![
                Property::boolean("snowy", false),
                Property::integer("age", 0, 0, 2),
            ],
        );
        assert_eq!(definition.get_possible_states().len(), 6);
        let default = definition.get_default_state();
        assert_eq!(
            definition.get_value(default.get_state_index(), "snowy"),
            Some(PropertyValue::Boolean(false))
        );
        assert_eq!(
            definition.get_value(default.get_state_index(), "age"),
            Some(PropertyValue::Integer(0))
        );
    }

    #[test]
    fn selector_matching_ignores_order() {
        let definition = StateDefinition::create(0, vec![Property::boolean("snowy", false)]);
        let snowy_true = definition.states[1];
        assert!(definition.matches_selector(snowy_true.get_state_index(), "snowy=true"));
        assert!(!definition.matches_selector(snowy_true.get_state_index(), "snowy=false"));
        assert!(definition.matches_selector(snowy_true.get_state_index(), ""));
    }

    #[test]
    fn setting_value_finds_state() {
        let definition = StateDefinition::create(0, vec![Property::boolean("snowy", false)]);
        let snowy = definition.get_property("snowy").unwrap();
        let default = definition.get_default_state();
        let new_state = definition
            .get_state_with(default, snowy, PropertyValue::Boolean(true))
            .unwrap();
        assert_eq!(
            definition.get_value(new_state.get_state_index(), "snowy"),
            Some(PropertyValue::Boolean(true))
        );
    }
}
