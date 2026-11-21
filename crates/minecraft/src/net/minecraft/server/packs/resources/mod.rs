//! `net.minecraft.server.packs.resources` 包。
//!
//! Java 的 `ResourceManager` 同时处理多个资源包栈、重载监听与异步加载；
//! 当前只支持一个已经解包的目录资源包（26.3 client jar 解压目录），
//! 足以直接使用 `minecraft-26.3-client-src/assets` 与 `data`。

use crate::net::minecraft::resources::Identifier;
use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum ResourceError {
    NotFound(Identifier),
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
}

impl fmt::Display for ResourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResourceError::NotFound(id) => write!(f, "找不到资源 {id}"),
            ResourceError::Io { path, source } => {
                write!(f, "读取资源 {} 失败: {source}", path.display())
            }
        }
    }
}

impl Error for ResourceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ResourceError::NotFound(_) => None,
            ResourceError::Io { source, .. } => Some(source),
        }
    }
}

/// 一份资源包中的原始资源。
pub struct Resource {
    id: Identifier,
    data: Vec<u8>,
}

impl Resource {
    pub fn get_id(&self) -> &Identifier {
        &self.id
    }

    pub fn open_as_reader(&self) -> &[u8] {
        &self.data
    }

    pub fn read_string(&self) -> Result<String, std::string::FromUtf8Error> {
        String::from_utf8(self.data.clone())
    }
}

/// 目录资源包 + 资源管理器。
///
/// `root` 指向解包后的客户端根目录（其下同时有 `assets/` 与 `data/`）。
pub struct ResourceManager {
    root: PathBuf,
}

impl ResourceManager {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn get_root(&self) -> &Path {
        &self.root
    }

    /// 读取 `assets/<namespace>/<path>`。
    pub fn get_resource(&self, id: &Identifier) -> Result<Resource, ResourceError> {
        let path = self
            .root
            .join("assets")
            .join(id.get_namespace())
            .join(id.get_path());
        self.read_file(id, path)
    }

    /// 读取 `data/<namespace>/<path>`。
    pub fn get_data_resource(&self, id: &Identifier) -> Result<Resource, ResourceError> {
        let path = self
            .root
            .join("data")
            .join(id.get_namespace())
            .join(id.get_path());
        self.read_file(id, path)
    }

    pub fn get_optional_resource(&self, id: &Identifier) -> Option<Resource> {
        self.get_resource(id).ok()
    }

    /// 读取模型 JSON：`assets/<ns>/models/<path>.json`。
    pub fn get_model_json(&self, id: &Identifier) -> Result<Resource, ResourceError> {
        self.get_resource(&self.prefixed(id, "models", ".json")?)
    }

    /// 读取方块状态 JSON：`assets/<ns>/blockstates/<path>.json`。
    pub fn get_blockstate_json(&self, id: &Identifier) -> Result<Resource, ResourceError> {
        self.get_resource(&self.prefixed(id, "blockstates", ".json")?)
    }

    /// 读取贴图 PNG：`assets/<ns>/textures/<path>.png`。
    pub fn get_texture_png(&self, id: &Identifier) -> Result<Resource, ResourceError> {
        self.get_resource(&self.prefixed(id, "textures", ".png")?)
    }

    /// 读取语言文件：`assets/<ns>/lang/<code>.json`。
    pub fn get_language_json(
        &self,
        namespace: &str,
        code: &str,
    ) -> Result<Resource, ResourceError> {
        let path = format!("lang/{code}.json");
        let id = Identifier::from_namespace_and_path(namespace, path.clone()).map_err(|_| {
            ResourceError::Io {
                path: self.root.join("assets").join(namespace).join(&path),
                source: std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("非法的语言资源标识符 {namespace}:{path}"),
                ),
            }
        })?;
        self.get_resource(&id)
    }

    fn prefixed(
        &self,
        id: &Identifier,
        directory: &str,
        extension: &str,
    ) -> Result<Identifier, ResourceError> {
        Identifier::from_namespace_and_path(
            id.get_namespace(),
            format!("{directory}/{}{extension}", id.get_path()),
        )
        .map_err(|_| ResourceError::NotFound(id.clone()))
    }

    fn read_file(&self, id: &Identifier, path: PathBuf) -> Result<Resource, ResourceError> {
        let data = std::fs::read(&path).map_err(|source| {
            if source.kind() == std::io::ErrorKind::NotFound {
                ResourceError::NotFound(id.clone())
            } else {
                ResourceError::Io {
                    path: path.clone(),
                    source,
                }
            }
        })?;
        Ok(Resource {
            id: id.clone(),
            data,
        })
    }
}
