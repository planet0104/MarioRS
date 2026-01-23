//! 公共存储后端
//!
//! 基于文件系统的存储实现，适用于所有支持 std::fs 的平台

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;

use crate::platform::StorageBackend;

/// 基于文件系统的存储后端
pub struct FileStorage {
    base_path: PathBuf,
}

impl FileStorage {
    /// 创建存储后端 (使用当前工作目录或可执行文件目录)
    pub fn new() -> Self {
        let base_path = std::env::current_dir()
            .ok()
            .or_else(|| {
                std::env::current_exe()
                    .ok()
                    .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            })
            .unwrap_or_else(|| PathBuf::from("."));
        Self { base_path }
    }

    /// 创建存储后端 (使用指定的基础路径)
    pub fn with_base_path(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    /// 获取完整文件路径
    fn get_path(&self, key: &str) -> PathBuf {
        self.base_path.join(key)
    }
}

impl Default for FileStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl StorageBackend for FileStorage {
    fn load(&self, key: &str) -> Option<Vec<u8>> {
        let path = self.get_path(key);
        let mut file = File::open(&path).ok()?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).ok()?;
        Some(buffer)
    }

    fn save(&mut self, key: &str, data: &[u8]) -> Result<(), String> {
        let path = self.get_path(key);
        // 确保父目录存在
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let mut file = File::create(&path).map_err(|e| e.to_string())?;
        file.write_all(data).map_err(|e| e.to_string())
    }

    fn remove(&mut self, key: &str) -> Result<(), String> {
        let path = self.get_path(key);
        fs::remove_file(&path).map_err(|e| e.to_string())
    }

    fn exists(&self, key: &str) -> bool {
        self.get_path(key).exists()
    }
}
