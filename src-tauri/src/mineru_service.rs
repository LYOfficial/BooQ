// MinerU 服务模块 - 本地 PDF 转 Markdown 工具
// MinerU 是一个开源的文档解析工具，支持 PDF 到 Markdown 的转换

#![allow(dead_code)]

use anyhow::{anyhow, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::{AppHandle, Manager};
use parking_lot::RwLock;
use once_cell::sync::Lazy;
use std::io::Write;

/// 缓存的 magic-pdf 可执行文件路径
static MAGIC_PDF_PATH: Lazy<RwLock<Option<String>>> = Lazy::new(|| RwLock::new(None));

/// MinerU 安装信息
#[derive(Debug, Clone, serde::Serialize)]
pub struct MineruInstallInfo {
    pub is_installed: bool,
    pub command_available: bool,
    pub executable_path: Option<String>,
    pub models_downloaded: bool,
    pub ocr_models_downloaded: bool,
    pub models_dir: Option<String>,
    pub modelscope_installed: bool,
}

/// 模型下载状态
#[derive(Debug, Clone, serde::Serialize)]
pub struct ModelDownloadStatus {
    pub model_type: String,  // "main" 或 "ocr"
    pub status: String,      // "downloading", "completed", "error"
    pub progress: Option<f32>,
    pub message: String,
}

/// MinerU 服务
pub struct MineruService {
    python_path: String,
}

impl MineruService {
    /// 创建新的 MinerU 服务实例
    pub fn new() -> Self {
        Self {
            python_path: "python".to_string(),
        }
    }

    /// 检查 ModelScope 是否已安装
    pub fn check_modelscope_installed() -> bool {
        let pip_check = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(["/C", "pip", "show", "modelscope"])
                .output()
        } else {
            Command::new("pip")
                .args(["show", "modelscope"])
                .output()
        };

        if let Ok(result) = pip_check {
            return result.status.success();
        }
        false
    }

    /// 检查 MinerU 是否已安装（通过 pip）
    pub fn check_installed() -> bool {
        // 方法1: 尝试通过 pip show 检查包是否安装
        let pip_check = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(["/C", "pip", "show", "magic-pdf"])
                .output()
        } else {
            Command::new("pip")
                .args(["show", "magic-pdf"])
                .output()
        };

        if let Ok(result) = pip_check {
            if result.status.success() {
                return true;
            }
        }

        // 方法2: 检查是否有可用路径或命令
        Self::check_command_available()
    }

    /// 检查 magic-pdf 命令是否可用（通过完整路径或 PATH）
    /// 注意：即使命令可用，如果模型未下载，MinerU 仍然无法正常工作
    pub fn check_command_available() -> bool {
        Self::check_command_available_with_storage(None)
    }

    /// 检查 magic-pdf 命令是否可用（带存储路径参数）
    pub fn check_command_available_with_storage(storage_path: Option<&str>) -> bool {
        // 首先检查是否有检测到的完整路径
        if let Some(exe_path) = Self::get_magic_pdf_path() {
            if std::path::Path::new(&exe_path).exists() {
                // 命令可用，但还需要检查模型是否存在
                return Self::check_all_models_exist_with_storage(storage_path);
            }
        }

        // 然后尝试直接调用（依赖 PATH）
        let version_check = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(["/C", "magic-pdf", "--version"])
                .output()
        } else {
            Command::new("magic-pdf")
                .arg("--version")
                .output()
        };

        if let Ok(result) = version_check {
            if result.status.success() {
                // 命令可用，但还需要检查模型是否存在
                return Self::check_all_models_exist_with_storage(storage_path);
            }
        }

        false
    }

    /// 检查所有必需的模型文件是否存在
    /// MinerU 需要主模型才能正常工作
    pub fn check_all_models_exist() -> bool {
        Self::check_all_models_exist_with_storage(None)
    }

    /// 检查所有必需的模型文件是否存在（带存储路径参数）
    pub fn check_all_models_exist_with_storage(storage_path: Option<&str>) -> bool {
        // 使用 get_mineru_models_dir 检测实际的模型目录
        if let Some(models_path) = Self::get_mineru_models_dir(storage_path) {
            // 检查 MFD 模型（公式检测）
            let mfd_model = models_path.join("MFD").join("YOLO").join("yolo_v8_ft.pt");
            if mfd_model.exists() {
                return true;
            }
            
            // 检查 Layout 模型
            let layout_dir = models_path.join("Layout").join("YOLO");
            if layout_dir.exists() {
                if let Ok(entries) = fs::read_dir(&layout_dir) {
                    for entry in entries.flatten() {
                        if entry.path().extension().map(|e| e == "pt").unwrap_or(false) {
                            return true;
                        }
                    }
                }
            }
        }
        
        false
    }

    /// 获取 magic-pdf 可执行文件的完整路径
    /// 通过 pip show 获取安装位置，然后推断 Scripts 目录
    pub fn get_magic_pdf_path() -> Option<String> {
        // 先检查缓存
        {
            let cached = MAGIC_PDF_PATH.read();
            if let Some(ref path) = *cached {
                // 验证路径仍然有效
                if Path::new(path).exists() {
                    return Some(path.clone());
                }
            }
        }

        // 尝试获取新路径
        let path = Self::detect_magic_pdf_path();
        if let Some(ref p) = path {
            let mut cached = MAGIC_PDF_PATH.write();
            *cached = Some(p.clone());
        }
        path
    }

    /// 刷新并重新检测 magic-pdf 路径
    pub fn refresh_magic_pdf_path() -> Option<String> {
        let path = Self::detect_magic_pdf_path();
        let mut cached = MAGIC_PDF_PATH.write();
        *cached = path.clone();
        path
    }

    /// 检测 magic-pdf 可执行文件路径
    fn detect_magic_pdf_path() -> Option<String> {
        // 方法1: 通过 pip show 获取安装位置
        let pip_show = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(["/C", "pip", "show", "magic-pdf"])
                .output()
        } else {
            Command::new("pip")
                .args(["show", "magic-pdf"])
                .output()
        };

        if let Ok(result) = pip_show {
            if result.status.success() {
                let output = String::from_utf8_lossy(&result.stdout);
                // 解析 Location 字段
                for line in output.lines() {
                    if line.starts_with("Location:") {
                        let location = line.trim_start_matches("Location:").trim();
                        // 从 site-packages 路径推断 Scripts 目录
                        // 例如: C:\...\Python311\site-packages -> C:\...\Python311\Scripts\magic-pdf.exe
                        if let Some(scripts_dir) = Self::get_scripts_dir_from_location(location) {
                            let magic_pdf_exe = if cfg!(target_os = "windows") {
                                scripts_dir.join("magic-pdf.exe")
                            } else {
                                scripts_dir.join("magic-pdf")
                            };
                            
                            if magic_pdf_exe.exists() {
                                return Some(magic_pdf_exe.to_string_lossy().to_string());
                            }
                        }
                    }
                }
            }
        }

        // 方法2: 使用 python -c 获取 Scripts 目录
        let python_scripts = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(["/C", "python", "-c", "import sysconfig; print(sysconfig.get_path('scripts'))"])
                .output()
        } else {
            Command::new("python")
                .args(["-c", "import sysconfig; print(sysconfig.get_path('scripts'))"])
                .output()
        };

        if let Ok(result) = python_scripts {
            if result.status.success() {
                let scripts_path = String::from_utf8_lossy(&result.stdout).trim().to_string();
                let magic_pdf_exe = if cfg!(target_os = "windows") {
                    PathBuf::from(&scripts_path).join("magic-pdf.exe")
                } else {
                    PathBuf::from(&scripts_path).join("magic-pdf")
                };
                
                if magic_pdf_exe.exists() {
                    return Some(magic_pdf_exe.to_string_lossy().to_string());
                }
            }
        }

        // 方法3: 检查常见的 Python 安装路径
        if cfg!(target_os = "windows") {
            let common_paths = [
                std::env::var("LOCALAPPDATA").ok().map(|p| PathBuf::from(p).join("Programs\\Python\\Python311\\Scripts\\magic-pdf.exe")),
                std::env::var("LOCALAPPDATA").ok().map(|p| PathBuf::from(p).join("Programs\\Python\\Python312\\Scripts\\magic-pdf.exe")),
                std::env::var("USERPROFILE").ok().map(|p| PathBuf::from(p).join("AppData\\Local\\Programs\\Python\\Python311\\Scripts\\magic-pdf.exe")),
                std::env::var("USERPROFILE").ok().map(|p| PathBuf::from(p).join("AppData\\Local\\Programs\\Python\\Python312\\Scripts\\magic-pdf.exe")),
            ];

            for path_opt in common_paths.iter().flatten() {
                if path_opt.exists() {
                    return Some(path_opt.to_string_lossy().to_string());
                }
            }
        }

        None
    }

    /// 从 site-packages 路径推断 Scripts 目录
    fn get_scripts_dir_from_location(location: &str) -> Option<PathBuf> {
        let path = PathBuf::from(location);
        
        // 向上查找直到找到包含 Scripts 目录的父目录
        let mut current = path.as_path();
        while let Some(parent) = current.parent() {
            let scripts = parent.join("Scripts");
            if scripts.exists() && scripts.is_dir() {
                return Some(scripts);
            }
            current = parent;
        }
        
        None
    }

    /// 获取可用的解析模式
    /// 如果模型文件存在，使用 auto 模式（更好的效果）
    /// 如果模型文件不存在，使用 txt 模式（不需要模型，仅提取文本）
    pub fn get_available_parse_mode() -> String {
        Self::get_available_parse_mode_with_storage(None)
    }

    /// 获取可用的解析模式（带存储路径）
    pub fn get_available_parse_mode_with_storage(storage_path: Option<&str>) -> String {
        // 使用 get_mineru_models_dir 检测实际的模型目录
        if let Some(models_path) = Self::get_mineru_models_dir(storage_path) {
            // 检查 MFD 模型（公式检测）- 这是 auto 模式必需的
            let mfd_model = models_path.join("MFD").join("YOLO").join("yolo_v8_ft.pt");
            if mfd_model.exists() {
                return "auto".to_string();
            }
            
            // 检查 Layout 模型
            let layout_dir = models_path.join("Layout").join("YOLO");
            if layout_dir.exists() {
                if let Ok(entries) = fs::read_dir(&layout_dir) {
                    for entry in entries.flatten() {
                        if entry.path().extension().map(|e| e == "pt").unwrap_or(false) {
                            return "auto".to_string();
                        }
                    }
                }
            }
        }
        
        // 默认使用 txt 模式（不需要模型）
        "txt".to_string()
    }

    /// 检查模型是否已下载
    pub fn check_models_downloaded() -> bool {
        let home_dir = if cfg!(target_os = "windows") {
            std::env::var("USERPROFILE").ok()
        } else {
            std::env::var("HOME").ok()
        };
        
        if let Some(home) = home_dir {
            let models_dir = PathBuf::from(&home).join("mineru_models");
            let layout_model = models_dir.join("Layout").join("YOLO");
            
            if layout_model.exists() {
                if let Ok(entries) = fs::read_dir(&layout_model) {
                    for entry in entries.flatten() {
                        if entry.path().extension().map(|e| e == "pt").unwrap_or(false) {
                            return true;
                        }
                    }
                }
            }
        }
        
        false
    }

    /// 获取模型存储目录（存放下载的模型包的根目录）
    pub fn get_models_dir(storage_path: Option<&str>) -> PathBuf {
        if let Some(path) = storage_path {
            if !path.is_empty() {
                return PathBuf::from(path).join("mineru_models");
            }
        }
        
        // 默认使用用户主目录
        let home_dir = if cfg!(target_os = "windows") {
            std::env::var("USERPROFILE").ok()
        } else {
            std::env::var("HOME").ok()
        };
        
        if let Some(home) = home_dir {
            PathBuf::from(&home).join("mineru_models")
        } else {
            PathBuf::from("mineru_models")
        }
    }

    /// 获取 MinerU 配置需要的 models-dir 路径
    /// MinerU 期望 models-dir 指向包含 MFD、Layout、OCR 等子目录的路径
    /// 这通常是 PDF-Extract-Kit-1.0/models 目录
    pub fn get_mineru_models_dir(storage_path: Option<&str>) -> Option<PathBuf> {
        let models_dir = Self::get_models_dir(storage_path);
        
        // 优先检查 PDF-Extract-Kit-1.0/models 目录（这是正确的结构）
        let pek_models = models_dir.join("PDF-Extract-Kit-1.0").join("models");
        if pek_models.exists() {
            let mfd_dir = pek_models.join("MFD");
            let layout_dir = pek_models.join("Layout");
            if mfd_dir.exists() || layout_dir.exists() {
                return Some(pek_models);
            }
        }
        
        // 检查直接包含 MFD/Layout 的目录
        let direct_mfd = models_dir.join("MFD");
        let direct_layout = models_dir.join("Layout");
        if direct_mfd.exists() || direct_layout.exists() {
            return Some(models_dir);
        }
        
        // 检查用户主目录
        let home_dir = if cfg!(target_os = "windows") {
            std::env::var("USERPROFILE").ok()
        } else {
            std::env::var("HOME").ok()
        };
        
        if let Some(home) = home_dir {
            let home_models = PathBuf::from(&home).join("mineru_models");
            
            // 检查 PDF-Extract-Kit
            let home_pek = home_models.join("PDF-Extract-Kit-1.0").join("models");
            if home_pek.exists() {
                let mfd_dir = home_pek.join("MFD");
                if mfd_dir.exists() {
                    return Some(home_pek);
                }
            }
            
            // 检查直接目录
            let home_mfd = home_models.join("MFD");
            if home_mfd.exists() {
                return Some(home_models);
            }
        }
        
        None
    }

    /// 检查 OCR 模型是否已下载
    pub fn check_ocr_models_downloaded(storage_path: Option<&str>) -> bool {
        // 检查 PDF-Extract-Kit 的 OCR 模型
        if let Some(models_path) = Self::get_mineru_models_dir(storage_path) {
            let ocr_dir = models_path.join("OCR").join("paddleocr_torch");
            if ocr_dir.exists() {
                if let Ok(entries) = fs::read_dir(&ocr_dir) {
                    for entry in entries.flatten() {
                        if entry.path().extension().map(|e| e == "pth").unwrap_or(false) {
                            return true;
                        }
                    }
                }
            }
        }
        
        false
    }

    /// 检查主模型（MFD/Layout）是否已下载
    pub fn check_main_models_downloaded(storage_path: Option<&str>) -> bool {
        if let Some(models_path) = Self::get_mineru_models_dir(storage_path) {
            // 检查 MFD 模型
            let mfd_model = models_path.join("MFD").join("YOLO").join("yolo_v8_ft.pt");
            if mfd_model.exists() {
                return true;
            }
            
            // 检查 Layout 模型
            let layout_dir = models_path.join("Layout").join("YOLO");
            if layout_dir.exists() {
                if let Ok(entries) = fs::read_dir(&layout_dir) {
                    for entry in entries.flatten() {
                        if entry.path().extension().map(|e| e == "pt").unwrap_or(false) {
                            return true;
                        }
                    }
                }
            }
        }
        
        false
    }

    /// 获取 MinerU 安装信息（用于显示在设置界面）
    pub fn get_install_info() -> MineruInstallInfo {
        Self::get_install_info_with_storage(None)
    }

    /// 获取 MinerU 安装信息（带存储路径）
    pub fn get_install_info_with_storage(storage_path: Option<&str>) -> MineruInstallInfo {
        let is_installed = Self::check_installed();
        let magic_pdf_path = Self::get_magic_pdf_path();
        // 使用带存储路径的检测方法
        let command_available = magic_pdf_path.is_some() && Self::check_all_models_exist_with_storage(storage_path);
        let models_downloaded = Self::check_main_models_downloaded(storage_path);
        let ocr_models_downloaded = Self::check_ocr_models_downloaded(storage_path);
        let modelscope_installed = Self::check_modelscope_installed();
        
        // 显示实际的模型目录（PDF-Extract-Kit-1.0/models）
        let models_dir_display = match Self::get_mineru_models_dir(storage_path) {
            Some(path) => path.to_string_lossy().to_string(),
            None => {
                let base = Self::get_models_dir(storage_path);
                base.join("PDF-Extract-Kit-1.0").join("models").to_string_lossy().to_string()
            }
        };
        
        MineruInstallInfo {
            is_installed,
            command_available,
            executable_path: magic_pdf_path,
            models_downloaded,
            ocr_models_downloaded,
            models_dir: Some(models_dir_display),
            modelscope_installed,
        }
    }

    /// 下载 MinerU 主模型（通过 modelscope）
    pub fn download_main_models_with_events(app_handle: &tauri::AppHandle, storage_path: Option<&str>) -> Result<String> {
        use std::io::{BufRead, BufReader};
        use std::process::Stdio;
        use crate::logger;

        let models_dir = Self::get_models_dir(storage_path);
        
        // 确保目录存在
        if !models_dir.exists() {
            fs::create_dir_all(&models_dir)?;
        }

        logger::info("mineru", &format!("开始下载 MinerU 主模型到: {}", models_dir.display()));

        let _ = app_handle.emit_all("mineru-model-output", 
            serde_json::json!({
                "type": "cmd", 
                "model_type": "main",
                "message": format!("> 下载 MinerU 2.5 模型到: {}\n", models_dir.display())
            }));

        // 创建临时 Python 脚本文件
        let temp_dir = std::env::temp_dir();
        let script_path = temp_dir.join("mineru_download_main.py");
        let target_dir = models_dir.join("MinerU2.5-2509-1.2B");
        
        let python_script = format!(
            r#"# -*- coding: utf-8 -*-
import sys
import os

print("正在初始化 ModelScope...", flush=True)
try:
    from modelscope import snapshot_download
    print("ModelScope 已加载", flush=True)
except ImportError as e:
    print(f"错误: 无法导入 modelscope: {{e}}", flush=True)
    sys.exit(1)

target_dir = r'{}'
print(f"目标目录: {{target_dir}}", flush=True)

try:
    print("开始下载 MinerU 2.5 模型，请耐心等待...", flush=True)
    model_dir = snapshot_download(
        'OpenDataLab/MinerU2.5-2509-1.2B', 
        local_dir=target_dir
    )
    print(f"模型下载成功，存放路径为: {{model_dir}}", flush=True)
except Exception as e:
    print(f"下载失败: {{e}}", flush=True)
    sys.exit(1)
"#,
            target_dir.to_string_lossy().replace("\\", "\\\\")
        );

        // 写入脚本文件
        fs::write(&script_path, &python_script)?;
        logger::debug("mineru", &format!("脚本文件: {}", script_path.display()));

        let _ = app_handle.emit_all("mineru-model-output", 
            serde_json::json!({
                "type": "info", 
                "model_type": "main",
                "message": "正在从 ModelScope 下载 MinerU 2.5 模型...\n这可能需要几分钟到几十分钟，取决于网络速度...\n"
            }));

        // 使用 python 执行脚本文件
        let mut child = Command::new("python")
            .arg(&script_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // 读取 stdout - 需要在主线程中等待
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        
        let app_handle_stdout = app_handle.clone();
        let stdout_thread = std::thread::spawn(move || {
            if let Some(stdout) = stdout {
                let reader = BufReader::new(stdout);
                for line in reader.lines().flatten() {
                    let _ = app_handle_stdout.emit_all("mineru-model-output",
                        serde_json::json!({
                            "type": "info", 
                            "model_type": "main",
                            "message": format!("{}\n", line)
                        }));
                }
            }
        });

        let app_handle_stderr = app_handle.clone();
        let stderr_thread = std::thread::spawn(move || {
            if let Some(stderr) = stderr {
                let reader = BufReader::new(stderr);
                for line in reader.lines().flatten() {
                    // ModelScope 的进度信息也走 stderr
                    let msg_type = if line.to_lowercase().contains("error") || line.to_lowercase().contains("failed") {
                        "error"
                    } else {
                        "info"
                    };
                    let _ = app_handle_stderr.emit_all("mineru-model-output",
                        serde_json::json!({
                            "type": msg_type, 
                            "model_type": "main",
                            "message": format!("{}\n", line)
                        }));
                }
            }
        });

        // 等待输出线程完成
        let _ = stdout_thread.join();
        let _ = stderr_thread.join();

        // 等待进程完成
        let status = child.wait()?;

        // 清理临时文件
        let _ = fs::remove_file(&script_path);

        if status.success() {
            // 验证模型是否真的下载成功
            if target_dir.exists() && target_dir.read_dir().map(|mut d| d.next().is_some()).unwrap_or(false) {
                logger::info("mineru", "MinerU 主模型下载成功");
                let _ = app_handle.emit_all("mineru-model-output",
                    serde_json::json!({
                        "type": "success", 
                        "model_type": "main",
                        "message": "\n✓ MinerU 2.5 模型下载成功！\n"
                    }));
                Ok("MinerU 主模型下载成功".to_string())
            } else {
                logger::error("mineru", "下载完成但未找到模型文件");
                let _ = app_handle.emit_all("mineru-model-output",
                    serde_json::json!({
                        "type": "error", 
                        "model_type": "main",
                        "message": "\n✗ 下载完成但未找到模型文件，请检查网络连接后重试\n"
                    }));
                Err(anyhow!("下载完成但未找到模型文件"))
            }
        } else {
            logger::error("mineru", &format!("MinerU 主模型下载失败，退出码: {:?}", status.code()));
            let _ = app_handle.emit_all("mineru-model-output",
                serde_json::json!({
                    "type": "error", 
                    "model_type": "main",
                    "message": format!("\n✗ 下载失败 (退出码: {:?})，请检查错误信息\n", status.code())
                }));
            Err(anyhow!("模型下载失败"))
        }
    }

    /// 下载 OCR 模型（PDF-Extract-Kit-1.0）
    pub fn download_ocr_models_with_events(app_handle: &tauri::AppHandle, storage_path: Option<&str>) -> Result<String> {
        use std::io::{BufRead, BufReader};
        use std::process::Stdio;
        use crate::logger;

        let models_dir = Self::get_models_dir(storage_path);
        
        // 确保目录存在
        if !models_dir.exists() {
            fs::create_dir_all(&models_dir)?;
        }

        logger::info("mineru", &format!("开始下载 OCR 模型到: {}", models_dir.display()));

        let _ = app_handle.emit_all("mineru-model-output", 
            serde_json::json!({
                "type": "cmd", 
                "model_type": "ocr",
                "message": format!("> 下载 PDF-Extract-Kit-1.0 OCR 模型到: {}\n", models_dir.display())
            }));

        // 创建临时 Python 脚本文件
        let temp_dir = std::env::temp_dir();
        let script_path = temp_dir.join("mineru_download_ocr.py");
        let target_dir = models_dir.join("PDF-Extract-Kit-1.0");
        
        let python_script = format!(
            r#"# -*- coding: utf-8 -*-
import sys
import os

print("正在初始化 ModelScope...", flush=True)
try:
    from modelscope import snapshot_download
    print("ModelScope 已加载", flush=True)
except ImportError as e:
    print(f"错误: 无法导入 modelscope: {{e}}", flush=True)
    sys.exit(1)

target_dir = r'{}'
print(f"目标目录: {{target_dir}}", flush=True)

try:
    print("开始下载 PDF-Extract-Kit-1.0 OCR 模型，这可能需要较长时间...", flush=True)
    model_dir = snapshot_download(
        'OpenDataLab/PDF-Extract-Kit-1.0', 
        local_dir=target_dir,
        max_workers=16
    )
    print(f"OCR 模型下载成功，存放路径为: {{model_dir}}", flush=True)
except Exception as e:
    print(f"下载失败: {{e}}", flush=True)
    sys.exit(1)
"#,
            target_dir.to_string_lossy().replace("\\", "\\\\")
        );

        // 写入脚本文件
        fs::write(&script_path, &python_script)?;
        logger::debug("mineru", &format!("脚本文件: {}", script_path.display()));

        let _ = app_handle.emit_all("mineru-model-output", 
            serde_json::json!({
                "type": "info", 
                "model_type": "ocr",
                "message": "正在从 ModelScope 下载 PDF-Extract-Kit-1.0 OCR 模型...\n这可能需要较长时间，请耐心等待...\n"
            }));

        // 使用 python 执行脚本文件
        let mut child = Command::new("python")
            .arg(&script_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // 读取输出
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        
        let app_handle_stdout = app_handle.clone();
        let stdout_thread = std::thread::spawn(move || {
            if let Some(stdout) = stdout {
                let reader = BufReader::new(stdout);
                for line in reader.lines().flatten() {
                    let _ = app_handle_stdout.emit_all("mineru-model-output",
                        serde_json::json!({
                            "type": "info", 
                            "model_type": "ocr",
                            "message": format!("{}\n", line)
                        }));
                }
            }
        });

        let app_handle_stderr = app_handle.clone();
        let stderr_thread = std::thread::spawn(move || {
            if let Some(stderr) = stderr {
                let reader = BufReader::new(stderr);
                for line in reader.lines().flatten() {
                    let msg_type = if line.to_lowercase().contains("error") || line.to_lowercase().contains("failed") {
                        "error"
                    } else {
                        "info"
                    };
                    let _ = app_handle_stderr.emit_all("mineru-model-output",
                        serde_json::json!({
                            "type": msg_type, 
                            "model_type": "ocr",
                            "message": format!("{}\n", line)
                        }));
                }
            }
        });

        // 等待输出线程完成
        let _ = stdout_thread.join();
        let _ = stderr_thread.join();

        // 等待进程完成
        let status = child.wait()?;

        // 清理临时文件
        let _ = fs::remove_file(&script_path);

        if status.success() {
            // 验证模型是否真的下载成功
            if target_dir.exists() && target_dir.read_dir().map(|mut d| d.next().is_some()).unwrap_or(false) {
                logger::info("mineru", "OCR 模型下载成功");
                let _ = app_handle.emit_all("mineru-model-output",
                    serde_json::json!({
                        "type": "success", 
                        "model_type": "ocr",
                        "message": "\n✓ PDF-Extract-Kit-1.0 OCR 模型下载成功！\n"
                    }));
                Ok("OCR 模型下载成功".to_string())
            } else {
                logger::error("mineru", "下载完成但未找到 OCR 模型文件");
                let _ = app_handle.emit_all("mineru-model-output",
                    serde_json::json!({
                        "type": "error", 
                        "model_type": "ocr",
                        "message": "\n✗ 下载完成但未找到模型文件，请检查网络连接后重试\n"
                    }));
                Err(anyhow!("下载完成但未找到模型文件"))
            }
        } else {
            logger::error("mineru", &format!("OCR 模型下载失败，退出码: {:?}", status.code()));
            let _ = app_handle.emit_all("mineru-model-output",
                serde_json::json!({
                    "type": "error", 
                    "model_type": "ocr",
                    "message": format!("\n✗ 下载失败 (退出码: {:?})，请检查错误信息\n", status.code())
                }));
            Err(anyhow!("OCR 模型下载失败"))
        }
    }

    /// 安装 modelscope 依赖
    pub fn install_modelscope_with_events(app_handle: &tauri::AppHandle) -> Result<String> {
        use std::io::{BufRead, BufReader};
        use std::process::Stdio;
        use crate::logger;

        logger::info("mineru", "开始安装 modelscope 依赖");

        let _ = app_handle.emit_all("mineru-model-output", 
            serde_json::json!({
                "type": "cmd", 
                "model_type": "deps",
                "message": "> pip install -U modelscope\n正在安装，请稍候...\n"
            }));

        // 使用 pip 安装
        let mut child = Command::new("pip")
            .args(["install", "-U", "modelscope"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // 读取输出
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        
        let app_handle_stdout = app_handle.clone();
        let stdout_thread = std::thread::spawn(move || {
            if let Some(stdout) = stdout {
                let reader = BufReader::new(stdout);
                for line in reader.lines().flatten() {
                    let _ = app_handle_stdout.emit_all("mineru-model-output",
                        serde_json::json!({
                            "type": "info", 
                            "model_type": "deps",
                            "message": format!("{}\n", line)
                        }));
                }
            }
        });

        let app_handle_stderr = app_handle.clone();
        let stderr_thread = std::thread::spawn(move || {
            if let Some(stderr) = stderr {
                let reader = BufReader::new(stderr);
                for line in reader.lines().flatten() {
                    // pip 的警告信息也走 stderr，不一定是错误
                    let msg_type = if line.to_lowercase().contains("error") {
                        "error"
                    } else {
                        "info"
                    };
                    let _ = app_handle_stderr.emit_all("mineru-model-output",
                        serde_json::json!({
                            "type": msg_type, 
                            "model_type": "deps",
                            "message": format!("{}\n", line)
                        }));
                }
            }
        });

        // 等待输出线程完成
        let _ = stdout_thread.join();
        let _ = stderr_thread.join();

        // 等待进程完成
        let status = child.wait()?;

        if status.success() {
            logger::info("mineru", "modelscope 安装成功");
            let _ = app_handle.emit_all("mineru-model-output",
                serde_json::json!({
                    "type": "success", 
                    "model_type": "deps",
                    "message": "\n✓ modelscope 安装成功！\n"
                }));
            Ok("modelscope 安装成功".to_string())
        } else {
            logger::error("mineru", &format!("modelscope 安装失败，退出码: {:?}", status.code()));
            let _ = app_handle.emit_all("mineru-model-output",
                serde_json::json!({
                    "type": "error", 
                    "model_type": "deps",
                    "message": format!("\n✗ 安装失败 (退出码: {:?})，请检查错误信息\n", status.code())
                }));
            Err(anyhow!("modelscope 安装失败"))
        }
    }

    /// 更新 magic-pdf.json 配置文件以使用指定的模型目录
    pub fn update_config_with_models(storage_path: Option<&str>) -> Result<()> {
        use crate::logger;
        
        // 获取用户主目录
        let home_dir = if cfg!(target_os = "windows") {
            std::env::var("USERPROFILE").ok()
        } else {
            std::env::var("HOME").ok()
        };
        
        let home_dir = match home_dir {
            Some(dir) => PathBuf::from(dir),
            None => return Err(anyhow!("无法获取用户主目录")),
        };
        
        let config_path = home_dir.join("magic-pdf.json");
        
        // 使用 get_mineru_models_dir 获取实际的模型目录
        let models_dir_path = match Self::get_mineru_models_dir(storage_path) {
            Some(path) => path,
            None => {
                let base_dir = Self::get_models_dir(storage_path);
                base_dir.join("PDF-Extract-Kit-1.0").join("models")
            }
        };
        
        logger::info("mineru", &format!("更新配置文件: {}", config_path.display()));
        logger::info("mineru", &format!("模型目录: {}", models_dir_path.display()));
        
        // 检查是否有 OCR 模型
        let ocr_enabled = Self::check_ocr_models_downloaded(storage_path);
        let has_models = Self::check_main_models_downloaded(storage_path);
        
        // 创建配置
        let config = if has_models {
            // 模型已下载，使用完整配置
            serde_json::json!({
                "models-dir": models_dir_path.to_string_lossy().to_string().replace("\\", "/"),
                "device-mode": "cuda",
                "table-config": {
                    "model": "TableMaster",
                    "is_table_recog_enable": false,
                    "max_time": 400
                },
                "layout-config": {
                    "model": "doclayout_yolo"
                },
                "formula-config": {
                    "mfd_model": "yolo_v8_mfd",
                    "mfr_model": "unimernet_small",
                    "enable": true
                },
                "ocr-config": {
                    "model": "native",
                    "enable": ocr_enabled
                },
                "latex-delimiter-config": {
                    "inline": {
                        "left": "$",
                        "right": "$"
                    },
                    "display": {
                        "left": "$$",
                        "right": "$$"
                    }
                }
            })
        } else {
            // 模型未下载，使用基本配置
            serde_json::json!({
                "models-dir": models_dir_path.to_string_lossy().to_string().replace("\\", "/"),
                "device-mode": "cpu",
                "table-config": {
                    "model": "TableMaster",
                    "is_table_recog_enable": false,
                    "max_time": 400
                },
                "layout-config": {
                    "model": "doclayout_yolo"
                },
                "formula-config": {
                    "mfd_model": "yolo_v8_mfd",
                    "mfr_model": "unimernet_small",
                    "enable": false
                },
                "latex-delimiter-config": {
                    "inline": {
                        "left": "$",
                        "right": "$"
                    },
                    "display": {
                        "left": "$$",
                        "right": "$$"
                    }
                }
            })
        };
        
        // 写入配置文件
        let config_str = serde_json::to_string_pretty(&config)?;
        let mut file = fs::File::create(&config_path)?;
        file.write_all(config_str.as_bytes())?;
        
        logger::info("mineru", "配置文件更新成功");
        
        Ok(())
    }

    /// 安装 MinerU（使用 pip），通过事件发送实时输出
    pub fn install_with_events(app_handle: &tauri::AppHandle) -> Result<String> {
        use std::io::{BufRead, BufReader};
        use std::process::Stdio;

        let _ = app_handle.emit_all("mineru-install-output", 
            serde_json::json!({"type": "cmd", "message": "> pip install -U \"magic-pdf[full]\"\n"}));

        let mut child = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(["/C", "pip", "install", "-U", "magic-pdf[full]"])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
        } else {
            Command::new("pip")
                .args(["install", "-U", "magic-pdf[full]"])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
        }?;

        // 读取 stdout
        if let Some(stdout) = child.stdout.take() {
            let app_handle_clone = app_handle.clone();
            std::thread::spawn(move || {
                let reader = BufReader::new(stdout);
                for line in reader.lines().flatten() {
                    let _ = app_handle_clone.emit_all("mineru-install-output",
                        serde_json::json!({"type": "info", "message": format!("{}\n", line)}));
                }
            });
        }

        // 读取 stderr
        if let Some(stderr) = child.stderr.take() {
            let app_handle_clone = app_handle.clone();
            std::thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines().flatten() {
                    let _ = app_handle_clone.emit_all("mineru-install-output",
                        serde_json::json!({"type": "error", "message": format!("{}\n", line)}));
                }
            });
        }

        // 等待进程完成
        let status = child.wait()?;

        if status.success() {
            let _ = app_handle.emit_all("mineru-install-output",
                serde_json::json!({"type": "success", "message": "\n✓ MinerU 安装成功！\n"}));
            Ok("MinerU 安装成功".to_string())
        } else {
            let _ = app_handle.emit_all("mineru-install-output",
                serde_json::json!({"type": "error", "message": "\n✗ 安装失败，请检查错误信息\n"}));
            Err(anyhow!("安装失败"))
        }
    }

    /// 安装 MinerU（使用 pip）- 旧版本保留
    pub async fn install() -> Result<String> {
        let output = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(["/C", "pip", "install", "-U", "magic-pdf[full]"])
                .output()
        } else {
            Command::new("pip")
                .args(["install", "-U", "magic-pdf[full]"])
                .output()
        };

        match output {
            Ok(result) => {
                if result.status.success() {
                    Ok("MinerU 安装成功".to_string())
                } else {
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    Err(anyhow!("安装失败: {}", stderr))
                }
            }
            Err(e) => Err(anyhow!("执行安装命令失败: {}", e)),
        }
    }

    /// 确保 magic-pdf.json 配置文件存在且格式正确
    /// MinerU 需要这个配置文件才能正常运行
    pub fn ensure_config_file() -> Result<()> {
        Self::ensure_config_file_with_storage(None)
    }

    /// 确保 magic-pdf.json 配置文件存在且格式正确（带存储路径）
    pub fn ensure_config_file_with_storage(storage_path: Option<&str>) -> Result<()> {
        use crate::logger;
        
        // 获取用户主目录
        let home_dir = if cfg!(target_os = "windows") {
            std::env::var("USERPROFILE").ok()
        } else {
            std::env::var("HOME").ok()
        };
        
        let home_dir = match home_dir {
            Some(dir) => PathBuf::from(dir),
            None => return Err(anyhow!("无法获取用户主目录")),
        };
        
        let config_path = home_dir.join("magic-pdf.json");
        
        // 使用 get_mineru_models_dir 获取实际的模型目录
        // 这个目录应该包含 MFD、Layout、OCR 等子目录
        let models_dir_path = match Self::get_mineru_models_dir(storage_path) {
            Some(path) => path,
            None => {
                // 如果没有找到模型，使用默认路径
                let base_dir = Self::get_models_dir(storage_path);
                base_dir.join("PDF-Extract-Kit-1.0").join("models")
            }
        };
        
        // 检查配置文件内容是否需要更新
        let need_update = if config_path.exists() {
            match fs::read_to_string(&config_path) {
                Ok(content) => {
                    // 检查 models-dir 是否指向正确的目录
                    let expected_path = models_dir_path.to_string_lossy().to_string().replace("\\", "/");
                    !content.contains(&expected_path)
                }
                Err(_) => true
            }
        } else {
            true
        };
        
        if !need_update {
            logger::debug("mineru", &format!("配置文件已存在且有效: {}", config_path.display()));
            return Ok(());
        }
        
        logger::info("mineru", &format!("创建/更新 MinerU 配置文件: {}", config_path.display()));
        logger::info("mineru", &format!("使用模型目录: {}", models_dir_path.display()));
        
        // 确保模型目录存在
        if !models_dir_path.exists() {
            fs::create_dir_all(&models_dir_path)?;
        }
        
        // 检测是否有 CUDA 可用
        let device_mode = "cpu"; // 默认使用 CPU，更安全
        
        // 创建配置 - 使用正确的模型目录（PDF-Extract-Kit-1.0/models）
        let config = serde_json::json!({
            "models-dir": models_dir_path.to_string_lossy().to_string().replace("\\", "/"),
            "device-mode": device_mode,
            "table-config": {
                "model": "TableMaster",
                "is_table_recog_enable": false,
                "max_time": 400
            },
            "layout-config": {
                "model": "doclayout_yolo"
            },
            "formula-config": {
                "mfd_model": "yolo_v8_mfd",
                "mfr_model": "unimernet_small",
                "enable": true
            },
            "latex-delimiter-config": {
                "inline": {
                    "left": "$",
                    "right": "$"
                },
                "display": {
                    "left": "$$",
                    "right": "$$"
                }
            }
        });
        
        // 写入配置文件
        let config_str = serde_json::to_string_pretty(&config)?;
        let mut file = fs::File::create(&config_path)?;
        file.write_all(config_str.as_bytes())?;
        
        logger::info("mineru", "MinerU 配置文件创建成功");
        
        Ok(())
    }

    /// 将 PDF 单页转换为 Markdown
    pub async fn convert_pdf_page(
        &self,
        pdf_path: &str,
        _page_number: u32,
        output_dir: &Path,
    ) -> Result<String> {
        // 确保 MinerU 配置文件存在
        Self::ensure_config_file()?;
        
        // 确保输出目录存在
        fs::create_dir_all(output_dir)?;

        // MinerU 使用 magic-pdf 命令行工具
        // magic-pdf -p <pdf_path> -o <output_dir> -m auto
        let output = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args([
                    "/C",
                    "magic-pdf",
                    "-p",
                    pdf_path,
                    "-o",
                    output_dir.to_str().unwrap_or("."),
                    "-m",
                    "auto",
                ])
                .output()
        } else {
            Command::new("magic-pdf")
                .args([
                    "-p",
                    pdf_path,
                    "-o",
                    output_dir.to_str().unwrap_or("."),
                    "-m",
                    "auto",
                ])
                .output()
        };

        match output {
            Ok(result) => {
                if result.status.success() {
                    // 读取生成的 Markdown 文件
                    // MinerU 会在输出目录生成以 PDF 文件名命名的子目录
                    let pdf_name = Path::new(pdf_path)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("output");

                    let auto_dir = output_dir.join(pdf_name).join("auto");
                    let md_file = auto_dir.join(format!("{}.md", pdf_name));

                    if md_file.exists() {
                        let content = fs::read_to_string(&md_file)?;
                        // 提取指定页面的内容（MinerU 会生成整个文档的 Markdown）
                        // 这里简化处理，返回完整内容，实际可能需要按页分割
                        Ok(content)
                    } else {
                        // 尝试查找其他可能的输出文件
                        self.find_markdown_output(&auto_dir, pdf_name)
                    }
                } else {
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    Err(anyhow!("转换失败: {}", stderr))
                }
            }
            Err(e) => Err(anyhow!("执行 MinerU 命令失败: {}", e)),
        }
    }

    /// 将整个 PDF 转换为 Markdown
    pub async fn convert_pdf_full(
        &self,
        pdf_path: &str,
        output_dir: &Path,
    ) -> Result<Vec<String>> {
        self.convert_pdf_full_with_storage(pdf_path, output_dir, None).await
    }

    /// 将整个 PDF 转换为 Markdown（带存储路径）
    pub async fn convert_pdf_full_with_storage(
        &self,
        pdf_path: &str,
        output_dir: &Path,
        storage_path: Option<&str>,
    ) -> Result<Vec<String>> {
        use crate::logger;
        
        // 确保 MinerU 配置文件存在，使用正确的模型路径
        if let Err(e) = Self::ensure_config_file_with_storage(storage_path) {
            logger::warn("mineru", &format!("创建配置文件失败: {}", e));
        }
        
        // 确保输出目录存在
        fs::create_dir_all(output_dir)?;
        
        logger::info("mineru", &format!("开始转换 PDF: {}", pdf_path));
        logger::info("mineru", &format!("输出目录: {}", output_dir.display()));

        // 获取 magic-pdf 可执行文件路径
        let magic_pdf_path = Self::get_magic_pdf_path();
        
        // 检查是否有模型文件，决定使用哪种模式
        // txt 模式不需要模型，auto 模式需要下载模型
        let parse_mode = Self::get_available_parse_mode_with_storage(storage_path);
        logger::info("mineru", &format!("使用解析模式: {}", parse_mode));
        
        let output = if let Some(ref exe_path) = magic_pdf_path {
            // 使用完整路径直接调用可执行文件（不通过 cmd）
            logger::info("mineru", &format!("使用路径: {}", exe_path));
            Command::new(exe_path)
                .args([
                    "-p",
                    pdf_path,
                    "-o",
                    output_dir.to_str().unwrap_or("."),
                    "-m",
                    &parse_mode,
                ])
                .output()
        } else if cfg!(target_os = "windows") {
            // 回退到通过 cmd 调用（依赖 PATH）
            logger::warn("mineru", "未找到完整路径，尝试直接调用 magic-pdf");
            Command::new("cmd")
                .args([
                    "/C",
                    "chcp",
                    "65001",
                    ">nul",
                    "&&",
                    "magic-pdf",
                    "-p",
                    pdf_path,
                    "-o",
                    output_dir.to_str().unwrap_or("."),
                    "-m",
                    &parse_mode,
                ])
                .output()
        } else {
            Command::new("magic-pdf")
                .args([
                    "-p",
                    pdf_path,
                    "-o",
                    output_dir.to_str().unwrap_or("."),
                    "-m",
                    &parse_mode,
                ])
                .output()
        };

        match output {
            Ok(result) => {
                // 记录输出
                let stdout = String::from_utf8_lossy(&result.stdout);
                let stderr = String::from_utf8_lossy(&result.stderr);
                
                if !stdout.is_empty() {
                    logger::debug("mineru", &format!("stdout: {}", stdout));
                }
                if !stderr.is_empty() {
                    logger::warn("mineru", &format!("stderr: {}", stderr));
                }
                
                if result.status.success() {
                    logger::info("mineru", "PDF 转换成功");
                    
                    let pdf_name = Path::new(pdf_path)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("output");

                    // 查找所有生成的 Markdown 文件
                    let auto_dir = output_dir.join(pdf_name).join("auto");
                    let mut markdown_files = Vec::new();

                    if auto_dir.exists() {
                        for entry in fs::read_dir(&auto_dir)? {
                            let entry = entry?;
                            let path = entry.path();
                            if path.extension().map(|e| e == "md").unwrap_or(false) {
                                markdown_files.push(path.to_string_lossy().to_string());
                            }
                        }
                    }

                    if markdown_files.is_empty() {
                        let err_msg = "未找到生成的 Markdown 文件";
                        logger::error("mineru", err_msg);
                        Err(anyhow!("{}", err_msg))
                    } else {
                        logger::info("mineru", &format!("找到 {} 个 Markdown 文件", markdown_files.len()));
                        Ok(markdown_files)
                    }
                } else {
                    let err_msg = format!("转换失败, 返回码: {:?}", result.status.code());
                    logger::error("mineru", &err_msg);
                    Err(anyhow!("{}", err_msg))
                }
            }
            Err(e) => {
                let err_msg = format!("执行 MinerU 命令失败: {}。请确认 MinerU 已正确安装。", e);
                logger::error("mineru", &err_msg);
                Err(anyhow!("{}", err_msg))
            }
        }
    }

    /// 查找 Markdown 输出文件
    fn find_markdown_output(&self, dir: &Path, _base_name: &str) -> Result<String> {
        if !dir.exists() {
            return Err(anyhow!("输出目录不存在"));
        }

        // 遍历目录查找 .md 文件
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map(|e| e == "md").unwrap_or(false) {
                return fs::read_to_string(&path).map_err(|e| anyhow!("读取文件失败: {}", e));
            }
        }

        Err(anyhow!("未找到 Markdown 输出文件"))
    }
}

/// 按页面分割 Markdown 内容
/// MinerU 生成的 Markdown 可能包含页面标记
pub fn split_markdown_by_pages(content: &str) -> Vec<String> {
    let mut pages = Vec::new();
    let mut current_page = String::new();
    
    for line in content.lines() {
        // 检查是否是页面分隔标记
        if line.starts_with("---") && line.contains("Page") {
            if !current_page.is_empty() {
                pages.push(current_page.clone());
                current_page.clear();
            }
        } else {
            current_page.push_str(line);
            current_page.push('\n');
        }
    }
    
    // 添加最后一页
    if !current_page.is_empty() {
        pages.push(current_page);
    }
    
    // 如果没有页面分隔符，返回整个内容作为一页
    if pages.is_empty() {
        pages.push(content.to_string());
    }
    
    pages
}

/// 获取 MinerU 输出目录
pub fn get_mineru_output_dir(app_handle: &AppHandle, file_id: &str) -> PathBuf {
    let config = crate::config::get_config_sync(app_handle);
    let base_path = if !config.storage_path.is_empty() {
        PathBuf::from(&config.storage_path)
    } else {
        app_handle
            .path_resolver()
            .app_data_dir()
            .unwrap()
            .join("files")
    };
    base_path.join(file_id).join("mineru_output")
}
