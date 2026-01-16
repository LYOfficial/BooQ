// BooQ - AI驱动的智能题库生成工具
// 主程序入口

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod commands;
mod file_manager;
mod ai_service;
mod ocr_service;
mod mineru_service;
mod rag_service;
mod question_analyzer;
mod config;
mod utils;
mod logger;

fn main() {
    // 加载 .env 文件（开发环境）
    let _ = dotenvy::dotenv();
    
    tauri::Builder::default()
        .setup(|app| {
            // 初始化应用数据目录
            let app_dir = app.path_resolver().app_data_dir().unwrap();
            std::fs::create_dir_all(&app_dir).ok();
            
            // 初始化配置
            config::init_config(&app_dir);
            
            // 记录启动日志
            logger::info("system", "BooQ 应用启动");
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // 文件管理命令
            commands::upload_file,
            commands::get_file_list,
            commands::delete_file,
            commands::rename_file,
            commands::copy_file,
            commands::get_file_content,
            commands::get_file_page,
            commands::get_total_pages,
            
            // OCR 和 Markdown 转换命令
            commands::convert_page_to_markdown,
            commands::get_markdown_content,
            commands::get_markdown_source,
            commands::check_paddle_ocr_configured,
            commands::convert_file_with_paddle_ocr,
            commands::clear_markdown_cache,
            
            // AI 分析命令
            commands::start_analysis,
            commands::stop_analysis,
            commands::get_analysis_progress,
            commands::get_questions,
            commands::get_question_detail,
            
            // 配置命令
            commands::get_config,
            commands::save_config,
            commands::get_models,
            commands::add_model,
            commands::remove_model,
            commands::set_storage_path,
            commands::get_storage_path,
            
            // 系统命令
            commands::get_system_theme,
            commands::test_model,
            
            // MinerU 相关命令
            commands::check_mineru_installed,
            commands::get_mineru_info,
            commands::get_mineru_full_info,
            commands::refresh_mineru_path,
            commands::install_mineru,
            commands::install_modelscope,
            commands::download_mineru_models,
            commands::download_ocr_models,
            commands::update_mineru_config,
            commands::convert_with_mineru,
            
            // 日志命令
            commands::get_logs,
            commands::clear_logs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
