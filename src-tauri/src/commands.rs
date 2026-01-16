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
