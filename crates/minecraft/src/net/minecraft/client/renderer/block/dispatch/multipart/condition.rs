//! 对应 `multipart/Condition.java`、`KeyValueCondition.java`、
//! `CombinedCondition.java`。
//!
//! 支持两种 JSON 形式：
//! - 字符串：`"facing=north,open=false"`，值内可用 `|` 表示 OR，`!` 取反；
//! - 对象：`{"AND": [...]}` 或 `{"OR": [...]}`，可递归嵌套。
//!
//! 匹配语义与 Java 相同：逗号分隔的条件是 AND；同一属性的多个候选值
//! 是 OR；否定词项表示“不等于该值”。

use crate::net::minecraft::util::json::{JsonError, JsonValue};
use crate::net::minecraft::world::level::block::state::{BlockState, StateDefinition};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
pub enum Condition {
    And(Vec<Condition>),
    Or(Vec<Condition>),
    KeyValue(BTreeMap<String, Vec<Term>>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Term {
    pub value: String,
    pub negated: bool,
}

impl Condition {
    pub fn from_json(value: &JsonValue) -> Result<Self, JsonError> {
        match value {
            JsonValue::String(text) => Self::parse_key_value(text),
            JsonValue::Object(map) => {
                if map.len() != 1 {
                    return Err(JsonError::message(
                        "组合条件必须且只能包含一个 `AND` 或 `OR`",
                    ));
                }
                let (operation, terms) = map.iter().next().expect("长度已校验为 1");
                let array = terms
                    .as_array()
                    .ok_or_else(|| JsonError::message(format!("`{operation}` 的值必须是数组")))?;
                let mut parsed = Vec::with_capacity(array.len());
                for term in array {
                    parsed.push(Self::from_json(term)?);
                }
                match operation.as_str() {
                    "AND" => Ok(Condition::And(parsed)),
                    "OR" => Ok(Condition::Or(parsed)),
                    other => Err(JsonError::message(format!(
                        "未知的组合条件 `{other}`，只允许 AND/OR"
                    ))),
                }
            }
            other => Err(JsonError::message(format!(
                "`when` 必须是字符串或对象，实际是 {}",
                other.type_name()
            ))),
        }
    }

    /// 解析 Java 的 `KeyValueCondition.Terms.CODEC` 字符串形式。
    pub fn parse_key_value(text: &str) -> Result<Self, JsonError> {
        let mut tests = BTreeMap::new();
        for entry in text.split(',') {
            let Some((key, raw_values)) = entry.split_once('=') else {
                return Err(JsonError::message(format!("条件项 `{entry}` 缺少 `=`")));
            };
            if key.is_empty() {
                return Err(JsonError::message("条件属性名不能为空"));
            }
            let mut terms = Vec::new();
            for raw in raw_values.split('|') {
                let (negated, value) = match raw.strip_prefix('!') {
                    Some(value) => (true, value),
                    None => (false, raw),
                };
                if value.is_empty() {
                    return Err(JsonError::message(format!("条件 `{text}` 含空值")));
                }
                terms.push(Term {
                    value: value.to_string(),
                    negated,
                });
            }
            tests.insert(key.to_string(), terms);
        }
        if tests.is_empty() {
            return Err(JsonError::message("条件不能为空"));
        }
        Ok(Condition::KeyValue(tests))
    }

    pub fn test(&self, definition: &StateDefinition, state: BlockState) -> Result<bool, String> {
        match self {
            Condition::And(terms) => {
                for term in terms {
                    if !term.test(definition, state)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            Condition::Or(terms) => {
                for term in terms {
                    if term.test(definition, state)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            Condition::KeyValue(tests) => {
                for (property_name, terms) in tests {
                    let property = definition
                        .get_property(property_name)
                        .ok_or_else(|| format!("条件引用了未知属性 `{property_name}`"))?;
                    let actual = definition
                        .get_value(state.get_state_index(), property_name)
                        .ok_or_else(|| format!("状态缺少属性 `{property_name}`"))?;

                    let mut matched = false;
                    for term in terms {
                        let expected = property.parse_value(&term.value).ok_or_else(|| {
                            format!("属性 `{property_name}` 不支持值 `{}`", term.value)
                        })?;
                        if (actual == expected) != term.negated {
                            matched = true;
                            break;
                        }
                    }
                    if !matched {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::minecraft::util::json::parse;
    use crate::net::minecraft::world::level::block::state::properties::PropertyValue;
    use crate::net::minecraft::world::level::block::{Blocks, SnowyBlock};

    #[test]
    fn parses_key_value_conditions_with_or_and_negation() {
        let condition = Condition::parse_key_value("snowy=true").unwrap();
        let block = Blocks::grass_block();
        let definition = block.get_state_definition();
        let state = block.default_block_state();
        assert!(!condition.test(definition, state).unwrap());

        let snowy = state
            .set_value(&SnowyBlock::SNOWY, PropertyValue::Boolean(true))
            .unwrap();
        assert!(condition.test(definition, snowy).unwrap());

        let negated = Condition::parse_key_value("snowy=!true").unwrap();
        assert!(negated.test(definition, state).unwrap());
        assert!(!negated.test(definition, snowy).unwrap());
    }

    #[test]
    fn parses_combined_conditions_recursively() {
        let json = parse(r#"{"OR": ["snowy=true", {"AND": ["snowy=false"]}]}"#).unwrap();
        let condition = Condition::from_json(&json).unwrap();
        let block = Blocks::grass_block();
        let state = block.default_block_state();
        assert!(condition.test(block.get_state_definition(), state).unwrap());

        // AND 内层为假时，整个 OR 仍可因另一项为真而匹配；
        // 若把外层改成 AND，默认状态（snowy=false）应不匹配。
        let json = parse(r#"{"AND": ["snowy=false", "snowy=true"]}"#).unwrap();
        let condition = Condition::from_json(&json).unwrap();
        assert!(!condition.test(block.get_state_definition(), state).unwrap());
    }

    #[test]
    fn rejects_invalid_shapes() {
        assert!(Condition::parse_key_value("").is_err());
        assert!(Condition::parse_key_value("snowy").is_err());
        assert!(Condition::parse_key_value("snowy=").is_err());
        let json = parse(r#"{"XOR": ["snowy=true"]}"#).unwrap();
        assert!(Condition::from_json(&json).is_err());
    }
}
