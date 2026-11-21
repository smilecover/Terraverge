//! Java 侧使用 Gson / DataFixerUpper Codec 读取 JSON。
//!
//! 为了满足“尽量不依赖第三方库”的约束，这里实现一个严格的 JSON 解析器，
//! 覆盖资源包需要的全部语法（对象、数组、字符串转义、Unicode、数字）。
//! 与 Gson 的差异：拒绝注释和尾随逗号（Minecraft 26.3 使用
//! `StrictJsonParser`，资源文件本身也都是标准 JSON）。

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(BTreeMap<String, JsonValue>),
}

impl JsonValue {
    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        match self {
            JsonValue::Object(map) => map.get(key),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&BTreeMap<String, JsonValue>> {
        match self {
            JsonValue::Object(map) => Some(map),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[JsonValue]> {
        match self {
            JsonValue::Array(values) => Some(values),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            JsonValue::String(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            JsonValue::Bool(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            JsonValue::Number(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_f32(&self) -> Option<f32> {
        self.as_f64().map(|value| value as f32)
    }

    pub fn as_i32(&self) -> Option<i32> {
        let value = self.as_f64()?;
        if value.fract() == 0.0 && value >= i32::MIN as f64 && value <= i32::MAX as f64 {
            Some(value as i32)
        } else {
            None
        }
    }

    pub fn as_u8(&self) -> Option<u8> {
        let value = self.as_f64()?;
        if value.fract() == 0.0 && (0.0..=255.0).contains(&value) {
            Some(value as u8)
        } else {
            None
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, JsonValue::Null)
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            JsonValue::Null => "null",
            JsonValue::Bool(_) => "boolean",
            JsonValue::Number(_) => "number",
            JsonValue::String(_) => "string",
            JsonValue::Array(_) => "array",
            JsonValue::Object(_) => "object",
        }
    }

    /// 供模型加载器使用的便捷访问：缺失/类型错误都变成带字段名的错误。
    pub fn require(&self, key: &str) -> Result<&JsonValue, JsonError> {
        self.get(key)
            .ok_or_else(|| JsonError::message(format!("缺少字段 `{key}`")))
    }

    pub fn require_str(&self, key: &str) -> Result<&str, JsonError> {
        self.require(key)?
            .as_str()
            .ok_or_else(|| JsonError::message(format!("字段 `{key}` 不是字符串")))
    }

    pub fn require_object(&self, key: &str) -> Result<&BTreeMap<String, JsonValue>, JsonError> {
        self.require(key)?
            .as_object()
            .ok_or_else(|| JsonError::message(format!("字段 `{key}` 不是对象")))
    }

    pub fn require_array(&self, key: &str) -> Result<&[JsonValue], JsonError> {
        self.require(key)?
            .as_array()
            .ok_or_else(|| JsonError::message(format!("字段 `{key}` 不是数组")))
    }

    pub fn optional_str(&self, key: &str) -> Option<&str> {
        self.get(key).and_then(JsonValue::as_str)
    }

    pub fn optional_bool(&self, key: &str) -> Option<bool> {
        self.get(key).and_then(JsonValue::as_bool)
    }

    pub fn optional_i32(&self, key: &str) -> Option<i32> {
        self.get(key).and_then(JsonValue::as_i32)
    }

    pub fn optional_f32(&self, key: &str) -> Option<f32> {
        self.get(key).and_then(JsonValue::as_f32)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonError {
    message: String,
    offset: Option<usize>,
    line: Option<usize>,
    column: Option<usize>,
}

impl JsonError {
    pub fn message(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            offset: None,
            line: None,
            column: None,
        }
    }

    fn at(message: impl Into<String>, input: &str, offset: usize) -> Self {
        let mut line = 1;
        let mut column = 1;
        for (index, ch) in input.char_indices() {
            if index >= offset {
                break;
            }
            if ch == '\n' {
                line += 1;
                column = 1;
            } else {
                column += 1;
            }
        }
        Self {
            message: message.into(),
            offset: Some(offset),
            line: Some(line),
            column: Some(column),
        }
    }
}

impl fmt::Display for JsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.line, self.column) {
            (Some(line), Some(column)) => {
                write!(
                    f,
                    "JSON 解析错误（第 {line} 行第 {column} 列）: {}",
                    self.message
                )
            }
            _ => f.write_str(&self.message),
        }
    }
}

impl Error for JsonError {}

/// 从字符串解析 JSON。
pub fn parse(input: &str) -> Result<JsonValue, JsonError> {
    let mut parser = Parser {
        bytes: input.as_bytes(),
        position: 0,
        depth: 0,
        input,
    };
    parser.skip_whitespace();
    let value = parser.parse_value()?;
    parser.skip_whitespace();
    if parser.position != input.len() {
        return Err(parser.error("JSON 结束后仍有多余内容"));
    }
    Ok(value)
}

/// 从字节解析 JSON（要求 UTF-8）。
pub fn parse_bytes(input: &[u8]) -> Result<JsonValue, JsonError> {
    let text = std::str::from_utf8(input)
        .map_err(|err| JsonError::message(format!("JSON 不是合法 UTF-8: {err}")))?;
    parse(text)
}

struct Parser<'a> {
    bytes: &'a [u8],
    position: usize,
    depth: usize,
    input: &'a str,
}

impl<'a> Parser<'a> {
    const MAX_DEPTH: usize = 512;

    fn error(&self, message: impl Into<String>) -> JsonError {
        JsonError::at(message, self.input, self.position)
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.position).copied()
    }

    fn bump(&mut self) -> Option<u8> {
        let byte = self.peek()?;
        self.position += 1;
        Some(byte)
    }

    fn skip_whitespace(&mut self) {
        while let Some(byte) = self.peek() {
            if matches!(byte, b' ' | b'\t' | b'\n' | b'\r') {
                self.position += 1;
            } else {
                break;
            }
        }
    }

    fn expect(&mut self, expected: u8) -> Result<(), JsonError> {
        match self.peek() {
            Some(actual) if actual == expected => {
                self.position += 1;
                Ok(())
            }
            Some(actual) => Err(self.error(format!(
                "期望 `{}`，实际是 `{}`",
                expected as char, actual as char
            ))),
            None => Err(self.error(format!("期望 `{}`，但已到结尾", expected as char))),
        }
    }

    fn parse_value(&mut self) -> Result<JsonValue, JsonError> {
        self.skip_whitespace();
        match self.peek() {
            Some(b'{') => self.parse_object(),
            Some(b'[') => self.parse_array(),
            Some(b'"') => self.parse_string().map(JsonValue::String),
            Some(b't') => {
                self.expect_literal(b"true")?;
                Ok(JsonValue::Bool(true))
            }
            Some(b'f') => {
                self.expect_literal(b"false")?;
                Ok(JsonValue::Bool(false))
            }
            Some(b'n') => {
                self.expect_literal(b"null")?;
                Ok(JsonValue::Null)
            }
            Some(byte) if byte == b'-' || byte.is_ascii_digit() => self.parse_number(),
            Some(byte) => Err(self.error(format!("非法的 JSON 值起始字符 `{}`", byte as char))),
            None => Err(self.error("JSON 意外结束")),
        }
    }

    fn expect_literal(&mut self, literal: &[u8]) -> Result<(), JsonError> {
        if self.bytes[self.position..].starts_with(literal) {
            self.position += literal.len();
            Ok(())
        } else {
            Err(self.error("非法字面量"))
        }
    }

    fn parse_object(&mut self) -> Result<JsonValue, JsonError> {
        self.enter()?;
        self.expect(b'{')?;
        let mut map = BTreeMap::new();
        self.skip_whitespace();
        if self.peek() == Some(b'}') {
            self.position += 1;
            self.depth -= 1;
            return Ok(JsonValue::Object(map));
        }

        loop {
            self.skip_whitespace();
            let key = self.parse_string()?;
            self.skip_whitespace();
            self.expect(b':')?;
            let value = self.parse_value()?;
            map.insert(key, value);
            self.skip_whitespace();
            match self.bump() {
                Some(b',') => continue,
                Some(b'}') => break,
                Some(_) => return Err(self.error("对象成员之间缺少 `,`")),
                None => return Err(self.error("对象未闭合")),
            }
        }

        self.depth -= 1;
        Ok(JsonValue::Object(map))
    }

    fn parse_array(&mut self) -> Result<JsonValue, JsonError> {
        self.enter()?;
        self.expect(b'[')?;
        let mut values = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(b']') {
            self.position += 1;
            self.depth -= 1;
            return Ok(JsonValue::Array(values));
        }

        loop {
            let value = self.parse_value()?;
            values.push(value);
            self.skip_whitespace();
            match self.bump() {
                Some(b',') => continue,
                Some(b']') => break,
                Some(_) => return Err(self.error("数组元素之间缺少 `,`")),
                None => return Err(self.error("数组未闭合")),
            }
        }

        self.depth -= 1;
        Ok(JsonValue::Array(values))
    }

    fn enter(&mut self) -> Result<(), JsonError> {
        self.depth += 1;
        if self.depth > Self::MAX_DEPTH {
            Err(self.error("JSON 嵌套层级过深"))
        } else {
            Ok(())
        }
    }

    fn parse_string(&mut self) -> Result<String, JsonError> {
        self.expect(b'"')?;
        let mut result = String::new();
        loop {
            let byte = self.bump().ok_or_else(|| self.error("字符串未闭合"))?;
            match byte {
                b'"' => return Ok(result),
                b'\\' => {
                    let escape = self.bump().ok_or_else(|| self.error("转义序列未完成"))?;
                    match escape {
                        b'"' => result.push('"'),
                        b'\\' => result.push('\\'),
                        b'/' => result.push('/'),
                        b'b' => result.push('\u{0008}'),
                        b'f' => result.push('\u{000c}'),
                        b'n' => result.push('\n'),
                        b'r' => result.push('\r'),
                        b't' => result.push('\t'),
                        b'u' => {
                            let first = self.parse_hex_codepoint()?;
                            let ch = if (0xD800..=0xDBFF).contains(&first) {
                                // UTF-16 代理对。
                                if self.bump() != Some(b'\\') || self.bump() != Some(b'u') {
                                    return Err(self.error("代理对缺少低位转义"));
                                }
                                let second = self.parse_hex_codepoint()?;
                                if !(0xDC00..=0xDFFF).contains(&second) {
                                    return Err(self.error("代理对低位非法"));
                                }
                                let codepoint =
                                    0x10000 + ((first - 0xD800) << 10) + (second - 0xDC00);
                                char::from_u32(codepoint)
                                    .ok_or_else(|| self.error("非法 Unicode 码点"))?
                            } else {
                                char::from_u32(first)
                                    .ok_or_else(|| self.error("非法 Unicode 码点"))?
                            };
                            result.push(ch);
                        }
                        other => {
                            return Err(self.error(format!("未知的转义字符 `\\{}`", other as char)));
                        }
                    }
                }
                byte if byte < 0x20 => {
                    return Err(self.error("字符串中包含未转义的控制字符"));
                }
                byte => {
                    // 非 ASCII 字节按 UTF-8 多字节读取。
                    if byte < 0x80 {
                        result.push(byte as char);
                    } else {
                        let start = self.position - 1;
                        let width =
                            utf8_width(byte).ok_or_else(|| self.error("非法的 UTF-8 起始字节"))?;
                        let end = start + width;
                        if end > self.bytes.len() {
                            return Err(self.error("UTF-8 序列被截断"));
                        }
                        let text = std::str::from_utf8(&self.bytes[start..end])
                            .map_err(|_| self.error("非法的 UTF-8 序列"))?;
                        result.push_str(text);
                        self.position = end;
                    }
                }
            }
        }
    }

    fn parse_hex_codepoint(&mut self) -> Result<u32, JsonError> {
        if self.position + 4 > self.bytes.len() {
            return Err(self.error("Unicode 转义不足 4 位"));
        }
        let text = std::str::from_utf8(&self.bytes[self.position..self.position + 4])
            .map_err(|_| self.error("Unicode 转义不是 ASCII"))?;
        let value = u32::from_str_radix(text, 16)
            .map_err(|_| self.error("Unicode 转义包含非十六进制字符"))?;
        self.position += 4;
        Ok(value)
    }

    fn parse_number(&mut self) -> Result<JsonValue, JsonError> {
        let start = self.position;
        if self.peek() == Some(b'-') {
            self.position += 1;
        }

        match self.bump() {
            Some(b'0') => {
                if self.peek().is_some_and(|b| b.is_ascii_digit()) {
                    return Err(self.error("数字不允许前导零"));
                }
            }
            Some(byte) if byte.is_ascii_digit() => {
                while self.peek().is_some_and(|b| b.is_ascii_digit()) {
                    self.position += 1;
                }
            }
            _ => return Err(self.error("非法数字")),
        }

        if self.peek() == Some(b'.') {
            self.position += 1;
            if !self.peek().is_some_and(|b| b.is_ascii_digit()) {
                return Err(self.error("小数点后缺少数字"));
            }
            while self.peek().is_some_and(|b| b.is_ascii_digit()) {
                self.position += 1;
            }
        }

        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.position += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.position += 1;
            }
            if !self.peek().is_some_and(|b| b.is_ascii_digit()) {
                return Err(self.error("指数部分缺少数字"));
            }
            while self.peek().is_some_and(|b| b.is_ascii_digit()) {
                self.position += 1;
            }
        }

        let text = &self.input[start..self.position];
        let value = text
            .parse::<f64>()
            .map_err(|_| self.error(format!("非法数字 `{text}`")))?;
        if !value.is_finite() {
            return Err(self.error("数字超出 f64 范围"));
        }
        Ok(JsonValue::Number(value))
    }
}

fn utf8_width(first: u8) -> Option<usize> {
    match first {
        0x00..=0x7F => Some(1),
        0xC0..=0xDF => Some(2),
        0xE0..=0xEF => Some(3),
        0xF0..=0xF7 => Some(4),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested_resource_json() {
        let json = r#"{
            "parent": "block/block",
            "textures": {"top": "block/grass_block_top"},
            "elements": [{ "from": [0, 0, 0], "to": [16, 16, 16] }],
            "flag": true,
            "nothing": null
        }"#;
        let value = parse(json).unwrap();
        assert_eq!(value.require_str("parent").unwrap(), "block/block");
        assert_eq!(
            value
                .require_object("textures")
                .unwrap()
                .get("top")
                .unwrap()
                .as_str()
                .unwrap(),
            "block/grass_block_top"
        );
        assert_eq!(value.require_array("elements").unwrap().len(), 1);
        assert_eq!(
            value.require_array("elements").unwrap()[0]
                .require_array("from")
                .unwrap()[1]
                .as_i32()
                .unwrap(),
            0
        );
        assert_eq!(value.optional_bool("flag"), Some(true));
        assert!(value.get("nothing").unwrap().is_null());
    }

    #[test]
    fn parses_unicode_and_escapes() {
        let value = parse(r#""中文 é 😀 \" \\ \n""#).unwrap();
        assert_eq!(value.as_str().unwrap(), "中文 é 😀 \" \\ \n");
    }

    #[test]
    fn rejects_trailing_commas_and_comments() {
        assert!(parse("[1, 2,]").is_err());
        assert!(parse("// comment\n{}").is_err());
    }

    #[test]
    fn reports_line_and_column() {
        let error = parse("{\n  \"a\": ]\n}").unwrap_err();
        assert_eq!(error.line, Some(2));
        assert!(error.to_string().contains("第 2 行"));
    }

    #[test]
    fn numbers_follow_json_grammar() {
        assert_eq!(parse("-12.5e2").unwrap().as_f64(), Some(-1250.0));
        assert_eq!(parse("-12.5e2").unwrap().as_i32(), Some(-1250));
        assert!(parse("01").is_err());
        assert!(parse("1.").is_err());
    }
}
