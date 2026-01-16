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
        // 首先检查是否有检测到的完整路径
        if let Some(exe_path) = Self::get_magic_pdf_path() {
            if std::path::Path::new(&exe_path).exists() {
                // 命令可用，但还需要检查模型是否存在
                return Self::check_all_models_exist();
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
                return Self::check_all_models_exist();
            }
        }

        false
    }

    /// 检查所有必需的模型文件是否存在
    /// MinerU 需要 OCR 模型才能正常工作
    pub fn check_all_models_exist() -> bool {
        let home_dir = if cfg!(target_os = "windows") {
            std::env::var("USERPROFILE").ok()
        } else {
            std::env::var("HOME").ok()
        };
        
        if let Some(home) = home_dir {
            let models_dir = PathBuf::from(&home).join("mineru_models");
            
            // 检查 OCR 模型（这是必需的）
            let ocr_model = models_dir.join("OCR").join("paddleocr_torch").join("ch_PP-OCRv3_det_infer.pth");
            if ocr_model.exists() {
                return true;
            }
            
            // 检查 OCR 目录下是否有任何 .pth 文件
            let ocr_dir = models_dir.join("OCR").join("paddleocr_torch");
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
        // 检查模型目录是否存在
        let home_dir = if cfg!(target_os = "windows") {
            std::env::var("USERPROFILE").ok()
        } else {
            std::env::var("HOME").ok()
        };
        
        if let Some(home) = home_dir {
            let models_dir = PathBuf::from(&home).join("mineru_models");
            let layout_model = models_dir.join("Layout").join("YOLO");
            
            // 检查是否有任何 .pt 模型文件
            if layout_model.exists() {
                if let Ok(entries) = fs::read_dir(&layout_model) {
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

    /// 获取 MinerU 安装信息（用于显示在设置界面）
    pub fn get_install_info() -> MineruInstallInfo {
        let is_installed = Self::check_installed();
        let magic_pdf_path = Self::get_magic_pdf_path();
        let command_available = magic_pdf_path.is_some() || Self::check_command_available();
        
        MineruInstallInfo {
            is_installed,
            command_available,
            executable_path: magic_pdf_path,
        }
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
        
        // 检查配置文件是否需要更新
        let need_update = if config_path.exists() {
            // 读取现有配置检查是否有效
            match fs::read_to_string(&config_path) {
                Ok(content) => {
                    // 检查是否包含正确的 layout-config 格式
                    // 如果不包含 "model": null 说明需要更新
                    !content.contains("\"model\": null") && content.contains("doclayout_yolo")
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
        
        // 创建模型目录
        let models_dir = home_dir.join("mineru_models");
        if !models_dir.exists() {
            fs::create_dir_all(&models_dir)?;
        }
        
        // 创建默认配置 - 禁用所有需要模型的功能
        // 这样即使没有下载模型也能使用基本的 PDF 文本提取
        let config = serde_json::json!({
            "models-dir": models_dir.to_string_lossy().to_string().replace("\\", "/"),
            "device-mode": "cpu",
            "table-config": {
                "model": "TableMaster",
                "is_table_recog_enable": false,
                "max_time": 400
            },
            "layout-config": {
                "model": null
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
        use crate::logger;
        
        // 确保 MinerU 配置文件存在
        if let Err(e) = Self::ensure_config_file() {
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
        let parse_mode = Self::get_available_parse_mode();
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
