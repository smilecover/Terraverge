//! 对应 `net/minecraft/resources/Identifier.java`（26.3 已将旧版
//! `ResourceLocation` 重命名为 `Identifier`）。
//!
//! 行为逐条对齐 Java：命名空间/路径字符集校验、默认命名空间、
//! 路径越权检查，以及语言键等派生字符串。

use crate::net::minecraft::IdentifierException;
use std::fmt;
use std::path::{Path, PathBuf};

/// `namespace:path` 形式的资源标识符。
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Identifier {
    namespace: String,
    path: String,
}

impl Identifier {
    pub const NAMESPACE_SEPARATOR: char = ':';
    pub const DEFAULT_NAMESPACE: &'static str = "minecraft";
    pub const REALMS_NAMESPACE: &'static str = "realms";
    pub const ALLOWED_NAMESPACE_CHARACTERS: &'static str = "[a-z0-9_.-]";

    fn new(namespace: String, path: String) -> Self {
        // Java 的私有构造带 assert；Rust 侧在公开入口完成同样校验。
        debug_assert!(Self::is_valid_namespace(&namespace));
        debug_assert!(Self::is_valid_path(&path));
        Self { namespace, path }
    }

    fn create_untrusted(
        namespace: impl Into<String>,
        path: impl Into<String>,
    ) -> Result<Self, IdentifierException> {
        let namespace = namespace.into();
        let path = path.into();
        assert_valid_namespace(&namespace, &path)?;
        assert_valid_path(&namespace, &path)?;
        Ok(Self { namespace, path })
    }

    /// 对应 `fromNamespaceAndPath`。非法输入返回错误而不是抛异常。
    pub fn from_namespace_and_path(
        namespace: impl Into<String>,
        path: impl Into<String>,
    ) -> Result<Self, IdentifierException> {
        Self::create_untrusted(namespace, path)
    }

    /// 对应 `parse`。
    pub fn parse(identifier: &str) -> Result<Self, IdentifierException> {
        Self::by_separator(identifier, Self::NAMESPACE_SEPARATOR)
    }

    /// 对应 `withDefaultNamespace`。
    pub fn with_default_namespace(path: impl Into<String>) -> Result<Self, IdentifierException> {
        let path = path.into();
        assert_valid_path(Self::DEFAULT_NAMESPACE, &path)?;
        Ok(Self::new(Self::DEFAULT_NAMESPACE.to_string(), path))
    }

    /// 对应 `tryParse`。
    pub fn try_parse(identifier: &str) -> Option<Self> {
        Self::try_by_separator(identifier, Self::NAMESPACE_SEPARATOR)
    }

    /// 对应 `tryBuild`。
    pub fn try_build(namespace: &str, path: &str) -> Option<Self> {
        if Self::is_valid_namespace(namespace) && Self::is_valid_path(path) {
            Some(Self::new(namespace.to_string(), path.to_string()))
        } else {
            None
        }
    }

    /// 对应 `bySeparator`。
    pub fn by_separator(identifier: &str, separator: char) -> Result<Self, IdentifierException> {
        match identifier.find(separator) {
            Some(index) => {
                let namespace = &identifier[..index];
                let path = &identifier[index + separator.len_utf8()..];
                if !namespace.is_empty() {
                    Self::create_untrusted(namespace, path)
                } else {
                    Self::with_default_namespace(path)
                }
            }
            None => Self::with_default_namespace(identifier),
        }
    }

    /// 对应 `tryBySeparator`。
    pub fn try_by_separator(identifier: &str, separator: char) -> Option<Self> {
        match identifier.find(separator) {
            Some(index) => {
                let namespace = &identifier[..index];
                let path = &identifier[index + separator.len_utf8()..];
                if !Self::is_valid_path(path) {
                    None
                } else if !namespace.is_empty() {
                    if Self::is_valid_namespace(namespace) {
                        Some(Self::new(namespace.to_string(), path.to_string()))
                    } else {
                        None
                    }
                } else {
                    Some(Self::new(
                        Self::DEFAULT_NAMESPACE.to_string(),
                        path.to_string(),
                    ))
                }
            }
            None => {
                if Self::is_valid_path(identifier) {
                    Some(Self::new(
                        Self::DEFAULT_NAMESPACE.to_string(),
                        identifier.to_string(),
                    ))
                } else {
                    None
                }
            }
        }
    }

    pub fn get_path(&self) -> &str {
        &self.path
    }

    pub fn get_namespace(&self) -> &str {
        &self.namespace
    }

    /// 对应 `withPath(String)`。返回值可能包含非法字符，因此用 `Result`。
    pub fn with_path(&self, new_path: impl Into<String>) -> Result<Self, IdentifierException> {
        let new_path = new_path.into();
        assert_valid_path(&self.namespace, &new_path)?;
        Ok(Self::new(self.namespace.clone(), new_path))
    }

    /// 对应 `withPath(UnaryOperator<String>)`。
    pub fn with_path_fn(
        &self,
        modifier: impl FnOnce(&str) -> String,
    ) -> Result<Self, IdentifierException> {
        self.with_path(modifier(&self.path))
    }

    pub fn with_prefix(&self, prefix: &str) -> Result<Self, IdentifierException> {
        self.with_path(format!("{prefix}{}", self.path))
    }

    pub fn with_suffix(&self, suffix: &str) -> Result<Self, IdentifierException> {
        self.with_path(format!("{}{suffix}", self.path))
    }

    /// 把标识符解析到根目录下的 `namespace/path`，并阻止 `..` 越界。
    pub fn resolve_against(&self, root: &Path) -> Result<PathBuf, IdentifierException> {
        let resulting_path = root.join(&self.namespace).join(&self.path);
        let normalized_path = normalize(&resulting_path);
        let normalized_root = normalize(root);
        if !normalized_path.starts_with(&normalized_root) {
            return Err(IdentifierException::new(format!(
                "Identifier \"{self}\" tried to access path \"{}\" from root \"{}\"",
                normalized_path.display(),
                normalized_root.display()
            )));
        }
        Ok(resulting_path)
    }

    pub fn to_debug_file_name(&self) -> String {
        self.to_string().replace('/', "_").replace(':', "_")
    }

    pub fn to_language_key(&self) -> String {
        format!("{}.{}", self.namespace, self.path)
    }

    pub fn to_short_language_key(&self) -> String {
        if self.namespace == Self::DEFAULT_NAMESPACE {
            self.path.clone()
        } else {
            self.to_language_key()
        }
    }

    pub fn to_short_string(&self) -> String {
        if self.namespace == Self::DEFAULT_NAMESPACE {
            self.path.clone()
        } else {
            self.to_string()
        }
    }

    pub fn to_language_key_with_prefix(&self, prefix: &str) -> String {
        format!("{prefix}.{}", self.to_language_key())
    }

    pub fn to_language_key_with_affixes(&self, prefix: &str, suffix: &str) -> String {
        format!("{prefix}.{}.{suffix}", self.to_language_key())
    }

    pub fn is_allowed_in_identifier(c: char) -> bool {
        c.is_ascii_digit()
            || c.is_ascii_lowercase()
            || c == '_'
            || c == ':'
            || c == '/'
            || c == '.'
            || c == '-'
    }

    pub fn is_valid_path(path: &str) -> bool {
        path.chars().all(Self::valid_path_char)
    }

    pub fn is_valid_namespace(namespace: &str) -> bool {
        namespace != ".." && namespace.chars().all(Self::valid_namespace_char)
    }

    pub fn valid_path_char(c: char) -> bool {
        c == '_' || c == '-' || c.is_ascii_lowercase() || c.is_ascii_digit() || c == '/' || c == '.'
    }

    fn valid_namespace_char(c: char) -> bool {
        c == '_' || c == '-' || c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.'
    }
}

fn assert_valid_namespace(namespace: &str, path: &str) -> Result<(), IdentifierException> {
    if Identifier::is_valid_namespace(namespace) {
        Ok(())
    } else {
        Err(IdentifierException::new(format!(
            "Non [a-z0-9_.-] character in namespace of identifier: {namespace}:{path}"
        )))
    }
}

fn assert_valid_path(namespace: &str, path: &str) -> Result<(), IdentifierException> {
    if Identifier::is_valid_path(path) {
        Ok(())
    } else {
        Err(IdentifierException::new(format!(
            "Non [a-z0-9/._-] character in path of location: {namespace}:{path}"
        )))
    }
}

/// `Path::normalize` 的稳定替代：处理 `.` 与 `..`，不访问文件系统。
fn normalize(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                result.pop();
            }
            std::path::Component::CurDir => {}
            other => result.push(other.as_os_str()),
        }
    }
    result
}

impl fmt::Display for Identifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.namespace, self.path)
    }
}

impl fmt::Debug for Identifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl PartialOrd for Identifier {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Identifier {
    /// Java 先比较 path 再比较 namespace。
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.path
            .cmp(&other.path)
            .then_with(|| self.namespace.cmp(&other.namespace))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_with_and_without_namespace() {
        assert_eq!(
            Identifier::parse("minecraft:block/stone")
                .unwrap()
                .to_string(),
            "minecraft:block/stone"
        );
        assert_eq!(
            Identifier::parse("stone").unwrap().to_string(),
            "minecraft:stone"
        );
        assert_eq!(
            Identifier::parse(":stone").unwrap().to_string(),
            "minecraft:stone"
        );
    }

    #[test]
    fn rejects_invalid_characters() {
        assert!(Identifier::parse("Minecraft:Stone").is_err());
        assert!(Identifier::parse("minecraft:st one").is_err());
        assert!(Identifier::try_parse("minecraft:st one").is_none());
    }

    #[test]
    fn resolve_against_blocks_escape() {
        let root = Path::new("assets");
        let id = Identifier::parse("minecraft:../../secret").unwrap();
        assert!(id.resolve_against(root).is_err());
    }

    #[test]
    fn language_keys_match_java() {
        let id = Identifier::parse("minecraft:block.stone").unwrap();
        assert_eq!(id.to_language_key(), "minecraft.block.stone");
        assert_eq!(id.to_short_language_key(), "block.stone");
        assert_eq!(
            Identifier::parse("custom:thing").unwrap().to_short_string(),
            "custom:thing"
        );
    }
}
