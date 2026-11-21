//! 模型加载错误：资源缺失、JSON 结构问题与解析失败统一走这里。

use crate::net::minecraft::server::packs::resources::ResourceError;
use crate::net::minecraft::util::json::JsonError;
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub struct ModelError {
    message: String,
}

impl ModelError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for ModelError {}

impl From<JsonError> for ModelError {
    fn from(value: JsonError) -> Self {
        Self::new(value.to_string())
    }
}

impl From<ResourceError> for ModelError {
    fn from(value: ResourceError) -> Self {
        Self::new(value.to_string())
    }
}

impl From<String> for ModelError {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for ModelError {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}
