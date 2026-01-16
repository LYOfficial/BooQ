// 工具函数模块

#![allow(dead_code)]

use sha2::{Sha256, Digest};
use std::time::{SystemTime, UNIX_EPOCH};

/// 生成唯一 ID
pub fn generate_id() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    
    let mut hasher = Sha256::new();
    hasher.update(timestamp.to_le_bytes());
    hasher.update(rand_bytes());
    
    let result = hasher.finalize();
    hex::encode(&result[..5])
}

/// 生成随机字节
fn rand_bytes() -> [u8; 8] {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    
    let bytes = timestamp.to_le_bytes();
    let mut result = [0u8; 8];
    result.copy_from_slice(&bytes[..8]);
    result
}

/// 格式化文件大小
pub fn format_file_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    
    if size >= GB {
        format!("{:.2} GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.2} MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.2} KB", size as f64 / KB as f64)
    } else {
        format!("{} B", size)
    }
}

/// 截断字符串
pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len - 3).collect();
        format!("{}...", truncated)
    }
}

/// 验证文件扩展名
pub fn is_valid_extension(filename: &str) -> bool {
    let valid_extensions = ["pdf", "doc", "docx", "ppt", "pptx", "txt"];
    
    if let Some(ext) = std::path::Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
    {
        valid_extensions.contains(&ext.to_lowercase().as_str())
    } else {
        false
    }
}
