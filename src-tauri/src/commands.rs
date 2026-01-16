// Tauri 命令处理模块

use crate::{config, file_manager, ocr_service, question_analyzer};
use serde::{Deserialize, Serialize};

// ==================== 数据结构定义 ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub file_type: String,
    pub path: String,
    pub size: u64,
    pub created_at: String,
    pub total_pages: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageContent {
    pub page_number: u32,
    pub content_type: String, // "image", "text", "markdown"
    pub content: String,      // base64 for image, text content for others
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Question {
    pub id: String,
    pub file_id: String,
    pub question_type: String, // "example", "exercise"
    pub chapter: String,
    pub section: String,
    pub knowledge_points: Vec<String>,
    pub question_text: String,
    pub answer: String,
    pub analysis: String,
    pub page_number: u32,
    pub has_original_answer: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisProgress {
    pub file_id: String,
    pub status: String, // "idle", "analyzing", "completed", "error"
    pub current_page: u32,
    pub total_pages: u32,
    pub current_step: String,
    pub questions_found: u32,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub api_url: String,
    pub api_key: String,
    pub model_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub storage_path: String,
    pub theme: String, // "light", "dark", "system"
    pub models: Vec<ModelConfig>,
    pub reading_model: String,
    pub analysis_model: String,
    pub solving_model: String,
    // OCR 相关配置
    #[serde(default)]
    pub use_paddle_ocr: bool,
    #[serde(default)]
    pub mineru_installed: bool,
    #[serde(default)]
    pub paddle_ocr_url: String,
    #[serde(default)]
    pub paddle_ocr_token: String,
}

// ==================== 文件管理命令 ====================

#[tauri::command]
pub async fn upload_file(
    app_handle: tauri::AppHandle,
    file_path: String,
    file_name: String,
) -> Result<FileInfo, String> {
    file_manager::upload_file(&app_handle, &file_path, &file_name)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_file_list(app_handle: tauri::AppHandle) -> Result<Vec<FileInfo>, String> {
    file_manager::get_file_list(&app_handle)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_file(app_handle: tauri::AppHandle, file_id: String) -> Result<(), String> {
    file_manager::delete_file(&app_handle, &file_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn rename_file(
    app_handle: tauri::AppHandle,
    file_id: String,
    new_name: String,
) -> Result<(), String> {
    file_manager::rename_file(&app_handle, &file_id, &new_name)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn copy_file(app_handle: tauri::AppHandle, file_id: String) -> Result<FileInfo, String> {
    file_manager::copy_file(&app_handle, &file_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_file_content(
    app_handle: tauri::AppHandle,
    file_id: String,
) -> Result<String, String> {
    file_manager::get_file_content(&app_handle, &file_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_file_page(
    app_handle: tauri::AppHandle,
    file_id: String,
    page_number: u32,
) -> Result<PageContent, String> {
    file_manager::get_file_page(&app_handle, &file_id, page_number)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_total_pages(
    app_handle: tauri::AppHandle,
    file_id: String,
) -> Result<u32, String> {
    file_manager::get_total_pages(&app_handle, &file_id)
        .await
        .map_err(|e| e.to_string())
}

// ==================== OCR 和 Markdown 命令 ====================

#[tauri::command]
pub async fn convert_page_to_markdown(
    app_handle: tauri::AppHandle,
    file_id: String,
    page_number: u32,
) -> Result<String, String> {
    ocr_service::convert_page_to_markdown(&app_handle, &file_id, page_number)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_markdown_content(
    app_handle: tauri::AppHandle,
    file_id: String,
    page_number: u32,
) -> Result<String, String> {
    ocr_service::get_markdown_content(&app_handle, &file_id, page_number)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_markdown_source(
    app_handle: tauri::AppHandle,
    file_id: String,
    page_number: u32,
) -> Result<String, String> {
    ocr_service::get_markdown_source(&app_handle, &file_id, page_number)
        .await
        .map_err(|e| e.to_string())
}

/// 检查 PaddleOCR-VL API 是否已配置
#[tauri::command]
pub fn check_paddle_ocr_configured() -> bool {
    ocr_service::PaddleOCRClient::is_configured()
}

/// 清除指定页面的 Markdown 缓存
#[tauri::command]
pub async fn clear_markdown_cache(
    app_handle: tauri::AppHandle,
    file_id: String,
    page_number: Option<u32>,
) -> Result<(), String> {
    ocr_service::clear_markdown_cache(&app_handle, &file_id, page_number)
        .await
        .map_err(|e| e.to_string())
}

/// 使用 PaddleOCR-VL 转换整个 PDF 文件
#[tauri::command]
pub async fn convert_file_with_paddle_ocr(
    app_handle: tauri::AppHandle,
    file_id: String,
) -> Result<Vec<String>, String> {
    // 获取文件信息
    let file_info = file_manager::get_file_info(&app_handle, &file_id)
        .await
        .map_err(|e| e.to_string())?;
    
    // 创建 PaddleOCR 客户端
    let client = ocr_service::PaddleOCRClient::from_env()
        .map_err(|e| e.to_string())?;
    
    // 获取输出目录
    let config = config::get_config_sync(&app_handle);
    let base_path = if !config.storage_path.is_empty() {
        std::path::PathBuf::from(&config.storage_path)
    } else {
        app_handle
            .path_resolver()
            .app_data_dir()
            .unwrap()
            .join("files")
    };
    let output_dir = base_path.join(&file_id).join("markdown");
    
    // 解析 PDF 并保存
    client.parse_and_save(&file_info.path, &output_dir)
        .await
        .map_err(|e| e.to_string())
}

// ==================== AI 分析命令 ====================

#[tauri::command]
pub async fn start_analysis(
    app_handle: tauri::AppHandle,
    file_id: String,
) -> Result<(), String> {
    question_analyzer::start_analysis(&app_handle, &file_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn stop_analysis(
    app_handle: tauri::AppHandle,
    file_id: String,
) -> Result<(), String> {
    question_analyzer::stop_analysis(&app_handle, &file_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_analysis_progress(
    app_handle: tauri::AppHandle,
    file_id: String,
) -> Result<AnalysisProgress, String> {
    question_analyzer::get_analysis_progress(&app_handle, &file_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_questions(
    app_handle: tauri::AppHandle,
    file_id: String,
) -> Result<Vec<Question>, String> {
    question_analyzer::get_questions(&app_handle, &file_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_question_detail(
    app_handle: tauri::AppHandle,
    file_id: String,
    question_id: String,
) -> Result<Question, String> {
    question_analyzer::get_question_detail(&app_handle, &file_id, &question_id)
        .await
        .map_err(|e| e.to_string())
}

// ==================== 配置命令 ====================

#[tauri::command]
pub async fn get_config(app_handle: tauri::AppHandle) -> Result<AppConfig, String> {
    config::get_config(&app_handle)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_config(
    app_handle: tauri::AppHandle,
    config_data: AppConfig,
) -> Result<(), String> {
    config::save_config(&app_handle, config_data)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_models(app_handle: tauri::AppHandle) -> Result<Vec<ModelConfig>, String> {
    config::get_models(&app_handle)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_model(
    app_handle: tauri::AppHandle,
    model: ModelConfig,
) -> Result<(), String> {
    config::add_model(&app_handle, model)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remove_model(
    app_handle: tauri::AppHandle,
    model_id: String,
) -> Result<(), String> {
    config::remove_model(&app_handle, &model_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_storage_path(
    app_handle: tauri::AppHandle,
    path: String,
) -> Result<(), String> {
    config::set_storage_path(&app_handle, &path)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_storage_path(app_handle: tauri::AppHandle) -> Result<String, String> {
    config::get_storage_path(&app_handle)
        .await
        .map_err(|e| e.to_string())
}

// ==================== 系统命令 ====================

#[tauri::command]
pub async fn test_model(
    api_url: String,
    api_key: String,
    model_name: String,
) -> Result<String, String> {
    use crate::ai_service::AIService;
    
    let service = AIService::new(&api_url, &api_key, &model_name);
    service
        .test_connection()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_system_theme() -> String {
    #[cfg(target_os = "windows")]
    {
        // Windows: 检查注册表获取系统主题
        use std::process::Command;
        let output = Command::new("reg")
            .args([
                "query",
                "HKCU\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize",
                "/v",
                "AppsUseLightTheme",
            ])
            .output();

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if stdout.contains("0x0") {
                    "dark".to_string()
                } else {
                    "light".to_string()
                }
            }
            Err(_) => "light".to_string(),
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        "light".to_string()
    }
}

// ==================== MinerU 相关命令 ====================

/// 检查 MinerU 是否已安装
#[tauri::command]
pub fn check_mineru_installed() -> bool {
    crate::mineru_service::MineruService::check_installed()
}

/// 获取 MinerU 安装详情
#[tauri::command]
pub fn get_mineru_info() -> crate::mineru_service::MineruInstallInfo {
    crate::mineru_service::MineruService::get_install_info()
}

/// 刷新 MinerU 路径检测
#[tauri::command]
pub fn refresh_mineru_path() -> Option<String> {
    crate::mineru_service::MineruService::refresh_magic_pdf_path()
}

/// 安装 MinerU（带实时输出）
#[tauri::command]
pub async fn install_mineru(app_handle: tauri::AppHandle) -> Result<String, String> {
    // 使用 spawn_blocking 在后台线程运行阻塞代码
    let result = tokio::task::spawn_blocking(move || {
        crate::mineru_service::MineruService::install_with_events(&app_handle)
    })
    .await
    .map_err(|e| e.to_string())?;
    
    // 安装完成后刷新路径检测
    crate::mineru_service::MineruService::refresh_magic_pdf_path();
    
    result.map_err(|e| e.to_string())
}

/// 使用 MinerU 转换 PDF
#[tauri::command]
pub async fn convert_with_mineru(
    app_handle: tauri::AppHandle,
    file_id: String,
) -> Result<Vec<String>, String> {
    use crate::mineru_service::{MineruService, get_mineru_output_dir};
    
    // 获取文件信息
    let file_info = file_manager::get_file_info(&app_handle, &file_id)
        .await
        .map_err(|e| e.to_string())?;
    
    let output_dir = get_mineru_output_dir(&app_handle, &file_id);
    
    let service = MineruService::new();
    service
        .convert_pdf_full(&file_info.path, &output_dir)
        .await
        .map_err(|e| e.to_string())
}

/// 获取 MinerU 详细安装信息（包含模型状态）
#[tauri::command]
pub fn get_mineru_full_info(app_handle: tauri::AppHandle) -> crate::mineru_service::MineruInstallInfo {
    let config = config::get_config_sync(&app_handle);
    let storage_path = if config.storage_path.is_empty() {
        None
    } else {
        Some(config.storage_path.as_str())
    };
    crate::mineru_service::MineruService::get_install_info_with_storage(storage_path)
}

/// 安装 modelscope 依赖
#[tauri::command]
pub async fn install_modelscope(app_handle: tauri::AppHandle) -> Result<String, String> {
    let result = tokio::task::spawn_blocking(move || {
        crate::mineru_service::MineruService::install_modelscope_with_events(&app_handle)
    })
    .await
    .map_err(|e| e.to_string())?;
    
    result.map_err(|e| e.to_string())
}

/// 下载 MinerU 主模型
#[tauri::command]
pub async fn download_mineru_models(app_handle: tauri::AppHandle) -> Result<String, String> {
    let config = config::get_config_sync(&app_handle);
    let storage_path = if config.storage_path.is_empty() {
        None
    } else {
        Some(config.storage_path.clone())
    };
    
    let result = tokio::task::spawn_blocking(move || {
        crate::mineru_service::MineruService::download_main_models_with_events(
            &app_handle, 
            storage_path.as_deref()
        )
    })
    .await
    .map_err(|e| e.to_string())?;
    
    result.map_err(|e| e.to_string())
}

/// 下载 OCR 模型
#[tauri::command]
pub async fn download_ocr_models(app_handle: tauri::AppHandle) -> Result<String, String> {
    let config = config::get_config_sync(&app_handle);
    let storage_path = if config.storage_path.is_empty() {
        None
    } else {
        Some(config.storage_path.clone())
    };
    
    let result = tokio::task::spawn_blocking(move || {
        crate::mineru_service::MineruService::download_ocr_models_with_events(
            &app_handle, 
            storage_path.as_deref()
        )
    })
    .await
    .map_err(|e| e.to_string())?;
    
    result.map_err(|e| e.to_string())
}

/// 更新 MinerU 配置文件
#[tauri::command]
pub fn update_mineru_config(app_handle: tauri::AppHandle) -> Result<String, String> {
    let config = config::get_config_sync(&app_handle);
    let storage_path = if config.storage_path.is_empty() {
        None
    } else {
        Some(config.storage_path.as_str())
    };
    
    crate::mineru_service::MineruService::update_config_with_models(storage_path)
        .map(|_| "配置更新成功".to_string())
        .map_err(|e| e.to_string())
}

// ==================== 日志命令 ====================

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub source: String,
    pub message: String,
}

/// 获取运行日志
#[tauri::command]
pub fn get_logs() -> Vec<LogEntry> {
    crate::logger::get_logs()
        .into_iter()
        .map(|e| LogEntry {
            timestamp: e.timestamp,
            level: e.level,
            source: e.source,
            message: e.message,
        })
        .collect()
}

/// 清空日志
#[tauri::command]
pub fn clear_logs() {
    crate::logger::clear_logs();
}
