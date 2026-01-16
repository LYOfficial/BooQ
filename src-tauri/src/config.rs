// 配置管理模块

use crate::commands::{AppConfig, ModelConfig};
use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

// 全局配置缓存
static CONFIG_CACHE: Lazy<RwLock<Option<AppConfig>>> = Lazy::new(|| RwLock::new(None));

/// 获取配置文件路径
fn get_config_path(app_handle: &AppHandle) -> PathBuf {
    app_handle
        .path_resolver()
        .app_data_dir()
        .unwrap()
        .join("config.json")
}

/// 初始化配置
pub fn init_config(app_dir: &Path) {
    let config_path = app_dir.join("config.json");
    
    if !config_path.exists() {
        let default_config = AppConfig {
            storage_path: String::new(),
            theme: "system".to_string(),
            models: Vec::new(),
            reading_model: String::new(),
            analysis_model: String::new(),
            solving_model: String::new(),
            use_paddle_ocr: false,
            mineru_installed: false,
            paddle_ocr_url: String::new(),
            paddle_ocr_token: String::new(),
        };
        
        if let Ok(content) = serde_json::to_string_pretty(&default_config) {
            fs::write(&config_path, content).ok();
        }
    }
}

/// 获取配置（异步）
pub async fn get_config(app_handle: &AppHandle) -> Result<AppConfig> {
    let config_path = get_config_path(app_handle);
    
    if config_path.exists() {
        let content = fs::read_to_string(&config_path)?;
        let config: AppConfig = serde_json::from_str(&content)?;
        
        // 更新缓存
        let mut cache = CONFIG_CACHE.write();
        *cache = Some(config.clone());
        
        Ok(config)
    } else {
        Ok(AppConfig {
            storage_path: String::new(),
            theme: "system".to_string(),
            models: Vec::new(),
            reading_model: String::new(),
            analysis_model: String::new(),
            solving_model: String::new(),
            use_paddle_ocr: false,
            mineru_installed: false,
            paddle_ocr_url: String::new(),
            paddle_ocr_token: String::new(),
        })
    }
}

/// 获取配置（同步）
pub fn get_config_sync(app_handle: &AppHandle) -> AppConfig {
    // 先检查缓存
    {
        let cache = CONFIG_CACHE.read();
        if let Some(config) = cache.as_ref() {
            return config.clone();
        }
    }
    
    // 从文件读取
    let config_path = get_config_path(app_handle);
    
    if config_path.exists() {
        if let Ok(content) = fs::read_to_string(&config_path) {
            if let Ok(config) = serde_json::from_str::<AppConfig>(&content) {
                // 更新缓存
                let mut cache = CONFIG_CACHE.write();
                *cache = Some(config.clone());
                return config;
            }
        }
    }
    
    AppConfig {
        storage_path: String::new(),
        theme: "system".to_string(),
        models: Vec::new(),
        reading_model: String::new(),
        analysis_model: String::new(),
        solving_model: String::new(),
        use_paddle_ocr: false,
        mineru_installed: false,
        paddle_ocr_url: String::new(),
        paddle_ocr_token: String::new(),
    }
}

/// 保存配置
pub async fn save_config(app_handle: &AppHandle, config: AppConfig) -> Result<()> {
    let config_path = get_config_path(app_handle);
    
    // 确保目录存在
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }
    
    let content = serde_json::to_string_pretty(&config)?;
    fs::write(&config_path, content)?;
    
    // 更新缓存
    let mut cache = CONFIG_CACHE.write();
    *cache = Some(config);
    
    Ok(())
}

/// 获取模型列表
pub async fn get_models(app_handle: &AppHandle) -> Result<Vec<ModelConfig>> {
    let config = get_config(app_handle).await?;
    Ok(config.models)
}

/// 添加模型
pub async fn add_model(app_handle: &AppHandle, model: ModelConfig) -> Result<()> {
    let mut config = get_config(app_handle).await?;
    
    // 检查是否已存在
    if config.models.iter().any(|m| m.id == model.id) {
        return Err(anyhow!("模型 ID 已存在"));
    }
    
    config.models.push(model);
    save_config(app_handle, config).await
}

/// 移除模型
pub async fn remove_model(app_handle: &AppHandle, model_id: &str) -> Result<()> {
    let mut config = get_config(app_handle).await?;
    
    config.models.retain(|m| m.id != model_id);
    
    // 清除引用
    if config.reading_model == model_id {
        config.reading_model = String::new();
    }
    if config.analysis_model == model_id {
        config.analysis_model = String::new();
    }
    if config.solving_model == model_id {
        config.solving_model = String::new();
    }
    
    save_config(app_handle, config).await
}

/// 设置存储路径
pub async fn set_storage_path(app_handle: &AppHandle, path: &str) -> Result<()> {
    let mut config = get_config(app_handle).await?;
    
    // 验证路径是否有效
    let path_obj = Path::new(path);
    if !path_obj.exists() {
        fs::create_dir_all(path_obj)?;
    }
    
    config.storage_path = path.to_string();
    save_config(app_handle, config).await
}

/// 获取存储路径
pub async fn get_storage_path(app_handle: &AppHandle) -> Result<String> {
    let config = get_config(app_handle).await?;
    
    if config.storage_path.is_empty() {
        let default_path = app_handle
            .path_resolver()
            .app_data_dir()
            .unwrap()
            .join("files");
        Ok(default_path.to_string_lossy().to_string())
    } else {
        Ok(config.storage_path)
    }
}
