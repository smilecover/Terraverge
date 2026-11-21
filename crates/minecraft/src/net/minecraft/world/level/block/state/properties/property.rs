//! 对应 `Property<T>` / `BooleanProperty` / `IntegerProperty`。
//!
//! Java 用泛型区分属性值类型；Rust 侧用 `PropertyValue` 枚举表达。
//! 属性本身是静态定义（`static SNOWY: Property`），值校验在解析时完成。

use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PropertyValue {
    Boolean(bool),
    Integer(i32),
}

impl PropertyValue {
    pub fn as_bool(self) -> Option<bool> {
        match self {
            PropertyValue::Boolean(value) => Some(value),
            PropertyValue::Integer(_) => None,
        }
    }

    pub fn as_i32(self) -> Option<i32> {
        match self {
            PropertyValue::Integer(value) => Some(value),
            PropertyValue::Boolean(_) => None,
        }
    }
}

impl fmt::Display for PropertyValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PropertyValue::Boolean(value) => write!(f, "{value}"),
            PropertyValue::Integer(value) => write!(f, "{value}"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PropertyKind {
    Boolean,
    Integer { min: i32, max: i32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Property {
    name: &'static str,
    kind: PropertyKind,
    default: PropertyValue,
}

impl Property {
    pub const fn boolean(name: &'static str, default: bool) -> Self {
        Self {
            name,
            kind: PropertyKind::Boolean,
            default: PropertyValue::Boolean(default),
        }
    }

    pub const fn integer(name: &'static str, default: i32, min: i32, max: i32) -> Self {
        Self {
            name,
            kind: PropertyKind::Integer { min, max },
            default: PropertyValue::Integer(default),
        }
    }

    pub const fn get_name(self) -> &'static str {
        self.name
    }

    pub const fn get_kind(self) -> PropertyKind {
        self.kind
    }

    pub const fn get_default_value(self) -> PropertyValue {
        self.default
    }

    /// Java 的 `getPossibleValues()`；布尔属性按 `false, true` 排列，
    /// 使默认状态落在下标 0（与 Java 的状态顺序不同，但语义等价）。
    pub fn get_possible_values(self) -> Vec<PropertyValue> {
        match self.kind {
            PropertyKind::Boolean => {
                vec![PropertyValue::Boolean(false), PropertyValue::Boolean(true)]
            }
            PropertyKind::Integer { min, max } => (min..=max).map(PropertyValue::Integer).collect(),
        }
    }

    pub fn parse_value(self, text: &str) -> Option<PropertyValue> {
        let value = match self.kind {
            PropertyKind::Boolean => match text {
                "true" => PropertyValue::Boolean(true),
                "false" => PropertyValue::Boolean(false),
                _ => return None,
            },
            PropertyKind::Integer { min, max } => {
                let value = text.parse::<i32>().ok()?;
                if value < min || value > max {
                    return None;
                }
                PropertyValue::Integer(value)
            }
        };
        if self.is_valid(value) {
            Some(value)
        } else {
            None
        }
    }

    pub fn is_valid(self, value: PropertyValue) -> bool {
        match (self.kind, value) {
            (PropertyKind::Boolean, PropertyValue::Boolean(_)) => true,
            (PropertyKind::Integer { min, max }, PropertyValue::Integer(value)) => {
                value >= min && value <= max
            }
            _ => false,
        }
    }

    pub fn value_to_string(self, value: PropertyValue) -> String {
        value.to_string()
    }
}

impl fmt::Display for Property {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_boolean_values() {
        let snowy = Property::boolean("snowy", false);
        assert_eq!(
            snowy.parse_value("true"),
            Some(PropertyValue::Boolean(true))
        );
        assert_eq!(snowy.parse_value("TRUE"), None);
        assert_eq!(snowy.get_possible_values().len(), 2);
    }

    #[test]
    fn integer_range_is_enforced() {
        let layers = Property::integer("layers", 1, 1, 8);
        assert_eq!(layers.parse_value("8"), Some(PropertyValue::Integer(8)));
        assert_eq!(layers.parse_value("9"), None);
        assert_eq!(layers.parse_value("-1"), None);
    }
}
