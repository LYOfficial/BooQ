// 日志服务模块 - 记录运行时日志

use once_cell::sync::Lazy;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use chrono::Local;

const MAX_LOG_ENTRIES: usize = 500;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,  // "info", "warn", "error", "debug"
    pub source: String, // "mineru", "paddleocr", "system"
    pub message: String,
}

static LOG_BUFFER: Lazy<RwLock<VecDeque<LogEntry>>> = Lazy::new(|| RwLock::new(VecDeque::new()));

/// 添加日志条目
pub fn log(level: &str, source: &str, message: &str) {
    let entry = LogEntry {
        timestamp: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        level: level.to_string(),
        source: source.to_string(),
        message: message.to_string(),
    };
    
    let mut buffer = LOG_BUFFER.write();
    buffer.push_back(entry);
    
    // 保持日志数量在限制内
    while buffer.len() > MAX_LOG_ENTRIES {
        buffer.pop_front();
    }
}

/// 记录信息日志
pub fn info(source: &str, message: &str) {
    log("info", source, message);
    println!("[INFO][{}] {}", source, message);
}

/// 记录警告日志
pub fn warn(source: &str, message: &str) {
    log("warn", source, message);
    println!("[WARN][{}] {}", source, message);
}

/// 记录错误日志
pub fn error(source: &str, message: &str) {
    log("error", source, message);
    eprintln!("[ERROR][{}] {}", source, message);
}

/// 记录调试日志
pub fn debug(source: &str, message: &str) {
    log("debug", source, message);
    #[cfg(debug_assertions)]
    println!("[DEBUG][{}] {}", source, message);
}

/// 获取所有日志
pub fn get_logs() -> Vec<LogEntry> {
    let buffer = LOG_BUFFER.read();
    buffer.iter().cloned().collect()
}

/// 获取指定来源的日志
#[allow(dead_code)]
pub fn get_logs_by_source(source: &str) -> Vec<LogEntry> {
    let buffer = LOG_BUFFER.read();
    buffer.iter()
        .filter(|e| e.source == source)
        .cloned()
        .collect()
}

/// 清空日志
pub fn clear_logs() {
    let mut buffer = LOG_BUFFER.write();
    buffer.clear();
}
