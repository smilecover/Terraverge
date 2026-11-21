//! 对应 `LanguageManager.java` 的最小实现：加载一个语言文件的键值表。

use crate::net::minecraft::server::packs::resources::ResourceManager;
use crate::net::minecraft::util::json::{self, JsonValue};
use std::collections::HashMap;
use std::error::Error;

pub struct LanguageManager {
    language_code: String,
    translations: HashMap<String, String>,
}

impl LanguageManager {
    pub fn load(
        resource_manager: &ResourceManager,
        language_code: &str,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let resource = resource_manager.get_language_json("minecraft", language_code)?;
        let text = resource
            .read_string()
            .map_err(|err| format!("语言文件不是 UTF-8: {err}"))?;
        let value = json::parse(&text)?;
        let object = value.as_object().ok_or("语言文件顶层必须是 JSON 对象")?;

        let mut translations = HashMap::with_capacity(object.len());
        for (key, value) in object {
            let Some(translation) = value.as_str() else {
                return Err(format!("语言键 `{key}` 的值不是字符串").into());
            };
            translations.insert(key.clone(), translation.to_string());
        }

        Ok(Self {
            language_code: language_code.to_string(),
            translations,
        })
    }

    pub fn get_language_code(&self) -> &str {
        &self.language_code
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.translations.get(key).map(String::as_str)
    }

    /// 对应 `getOrDefault(key, fallback)`。
    pub fn get_or<'a>(&'a self, key: &str, fallback: &'a str) -> &'a str {
        self.get(key).unwrap_or(fallback)
    }

    /// 缺失时直接返回键本身，便于 UI 调试。
    pub fn get_or_key<'a>(&'a self, key: &'a str) -> &'a str {
        self.get(key).unwrap_or(key)
    }

    pub fn len(&self) -> usize {
        self.translations.len()
    }

    pub fn is_empty(&self) -> bool {
        self.translations.is_empty()
    }

    pub fn parse_json_value(&self, value: &JsonValue) -> Option<&str> {
        value.as_str().and_then(|key| self.get(key))
    }
}
