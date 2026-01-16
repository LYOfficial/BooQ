// 文件管理模块

use crate::commands::{FileInfo, PageContent};
use anyhow::{anyhow, Result};
use sha2::{Sha256, Digest};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::AppHandle;
use chrono::Utc;
use base64::{Engine as _, engine::general_purpose};

/// 生成10位哈希ID
fn generate_file_id(content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content);
    hasher.update(Utc::now().timestamp_nanos_opt().unwrap_or(0).to_le_bytes());
    let result = hasher.finalize();
    hex::encode(&result[..5]) // 10个字符
}

/// 获取存储根路径
fn get_storage_root(app_handle: &AppHandle) -> PathBuf {
    let config = crate::config::get_config_sync(app_handle);
    if !config.storage_path.is_empty() {
        PathBuf::from(&config.storage_path)
    } else {
        app_handle
            .path_resolver()
            .app_data_dir()
            .unwrap()
            .join("files")
    }
}

/// 获取文件类型
fn get_file_type(file_name: &str) -> String {
    let extension = Path::new(file_name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    
    match extension.as_str() {
        "pdf" => "pdf".to_string(),
        "doc" | "docx" => "word".to_string(),
        "ppt" | "pptx" => "ppt".to_string(),
        "txt" => "txt".to_string(),
        _ => "unknown".to_string(),
    }
}

/// 上传文件
pub async fn upload_file(
    app_handle: &AppHandle,
    file_path: &str,
    file_name: &str,
) -> Result<FileInfo> {
    let source_path = Path::new(file_path);
    if !source_path.exists() {
        return Err(anyhow!("源文件不存在"));
    }

    // 读取文件内容
    let content = fs::read(source_path)?;
    let file_size = content.len() as u64;
    
    // 生成文件ID
    let file_id = generate_file_id(&content);
    
    // 创建文件目录
    let storage_root = get_storage_root(app_handle);
    let file_dir = storage_root.join(&file_id);
    fs::create_dir_all(&file_dir)?;
    
    // 复制文件到存储目录
    let file_type = get_file_type(file_name);
    let stored_file_path = file_dir.join(format!("source.{}", 
        Path::new(file_name).extension().and_then(|e| e.to_str()).unwrap_or("bin")));
    fs::write(&stored_file_path, &content)?;
    
    // 创建元数据文件
    let file_info = FileInfo {
        id: file_id.clone(),
        name: file_name.to_string(),
        display_name: file_name.to_string(),
        file_type,
        path: stored_file_path.to_string_lossy().to_string(),
        size: file_size,
        created_at: Utc::now().to_rfc3339(),
        total_pages: get_file_pages(&stored_file_path).unwrap_or(1),
    };
    
    // 保存元数据
    let meta_path = file_dir.join("meta.json");
    let meta_json = serde_json::to_string_pretty(&file_info)?;
    fs::write(meta_path, meta_json)?;
    
    // 创建 markdown 目录
    fs::create_dir_all(file_dir.join("markdown"))?;
    
    // 创建 questions 目录
    fs::create_dir_all(file_dir.join("questions"))?;
    
    Ok(file_info)
}

/// 获取文件页数
fn get_file_pages(file_path: &Path) -> Result<u32> {
    let extension = file_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    
    match extension.as_str() {
        "pdf" => {
            // 使用 lopdf 获取 PDF 页数
            match lopdf::Document::load(file_path) {
                Ok(doc) => Ok(doc.get_pages().len() as u32),
                Err(_) => Ok(1),
            }
        }
        "txt" => Ok(1),
        _ => Ok(1), // Word 和 PPT 需要更复杂的处理
    }
}

/// 获取文件列表
pub async fn get_file_list(app_handle: &AppHandle) -> Result<Vec<FileInfo>> {
    let storage_root = get_storage_root(app_handle);
    
    if !storage_root.exists() {
        return Ok(Vec::new());
    }
    
    let mut files = Vec::new();
    
    for entry in fs::read_dir(&storage_root)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_dir() {
            let meta_path = path.join("meta.json");
            if meta_path.exists() {
                if let Ok(content) = fs::read_to_string(&meta_path) {
                    if let Ok(file_info) = serde_json::from_str::<FileInfo>(&content) {
                        files.push(file_info);
                    }
                }
            }
        }
    }
    
    // 按创建时间排序
    files.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    
    Ok(files)
}

/// 获取单个文件信息
pub async fn get_file_info(app_handle: &AppHandle, file_id: &str) -> Result<FileInfo> {
    let storage_root = get_storage_root(app_handle);
    let meta_path = storage_root.join(file_id).join("meta.json");
    
    if !meta_path.exists() {
        return Err(anyhow!("文件不存在"));
    }
    
    let content = fs::read_to_string(&meta_path)?;
    let file_info: FileInfo = serde_json::from_str(&content)?;
    Ok(file_info)
}

/// 删除文件
pub async fn delete_file(app_handle: &AppHandle, file_id: &str) -> Result<()> {
    let storage_root = get_storage_root(app_handle);
    let file_dir = storage_root.join(file_id);
    
    if file_dir.exists() {
        fs::remove_dir_all(file_dir)?;
    }
    
    Ok(())
}

/// 重命名文件
pub async fn rename_file(app_handle: &AppHandle, file_id: &str, new_name: &str) -> Result<()> {
    let storage_root = get_storage_root(app_handle);
    let meta_path = storage_root.join(file_id).join("meta.json");
    
    if !meta_path.exists() {
        return Err(anyhow!("文件不存在"));
    }
    
    let content = fs::read_to_string(&meta_path)?;
    let mut file_info: FileInfo = serde_json::from_str(&content)?;
    file_info.display_name = new_name.to_string();
    
    let meta_json = serde_json::to_string_pretty(&file_info)?;
    fs::write(meta_path, meta_json)?;
    
    Ok(())
}

/// 复制文件
pub async fn copy_file(app_handle: &AppHandle, file_id: &str) -> Result<FileInfo> {
    let storage_root = get_storage_root(app_handle);
    let source_dir = storage_root.join(file_id);
    
    if !source_dir.exists() {
        return Err(anyhow!("源文件不存在"));
    }
    
    // 读取原文件元数据
    let meta_path = source_dir.join("meta.json");
    let content = fs::read_to_string(&meta_path)?;
    let source_info: FileInfo = serde_json::from_str(&content)?;
    
    // 读取源文件
    let source_file = fs::read(&source_info.path)?;
    
    // 生成新的文件ID
    let new_id = generate_file_id(&source_file);
    let new_dir = storage_root.join(&new_id);
    
    // 复制整个目录
    copy_dir_recursive(&source_dir, &new_dir)?;
    
    // 更新元数据
    let new_meta_path = new_dir.join("meta.json");
    let mut new_info = source_info.clone();
    new_info.id = new_id.clone();
    new_info.display_name = format!("{} (副本)", source_info.display_name);
    new_info.created_at = Utc::now().to_rfc3339();
    
    // 更新文件路径
    let extension = Path::new(&source_info.path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("bin");
    new_info.path = new_dir.join(format!("source.{}", extension)).to_string_lossy().to_string();
    
    let meta_json = serde_json::to_string_pretty(&new_info)?;
    fs::write(new_meta_path, meta_json)?;
    
    Ok(new_info)
}

/// 递归复制目录
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    
    Ok(())
}

/// 获取文件内容
pub async fn get_file_content(app_handle: &AppHandle, file_id: &str) -> Result<String> {
    let storage_root = get_storage_root(app_handle);
    let meta_path = storage_root.join(file_id).join("meta.json");
    
    if !meta_path.exists() {
        return Err(anyhow!("文件不存在"));
    }
    
    let content = fs::read_to_string(&meta_path)?;
    let file_info: FileInfo = serde_json::from_str(&content)?;
    
    // 对于文本文件，直接返回内容
    if file_info.file_type == "txt" {
        let file_content = fs::read_to_string(&file_info.path)?;
        return Ok(file_content);
    }
    
    // 对于其他文件，返回 base64 编码
    let file_content = fs::read(&file_info.path)?;
    Ok(general_purpose::STANDARD.encode(&file_content))
}

/// 获取文件指定页面
pub async fn get_file_page(
    app_handle: &AppHandle,
    file_id: &str,
    page_number: u32,
) -> Result<PageContent> {
    let storage_root = get_storage_root(app_handle);
    let meta_path = storage_root.join(file_id).join("meta.json");
    
    if !meta_path.exists() {
        return Err(anyhow!("文件不存在"));
    }
    
    let content = fs::read_to_string(&meta_path)?;
    let file_info: FileInfo = serde_json::from_str(&content)?;
    
    match file_info.file_type.as_str() {
        "pdf" => get_pdf_page(&file_info.path, page_number),
        "txt" => get_txt_page(&file_info.path, page_number),
        _ => Err(anyhow!("不支持的文件类型")),
    }
}

/// 获取 PDF 页面
fn get_pdf_page(file_path: &str, page_number: u32) -> Result<PageContent> {
    // 读取 PDF 文件
    let file_content = fs::read(file_path)?;
    let base64_content = general_purpose::STANDARD.encode(&file_content);
    
    Ok(PageContent {
        page_number,
        content_type: "pdf".to_string(),
        content: base64_content,
        width: 0,
        height: 0,
    })
}

/// 获取文本页面
fn get_txt_page(file_path: &str, _page_number: u32) -> Result<PageContent> {
    let content = fs::read_to_string(file_path)?;
    
    Ok(PageContent {
        page_number: 1,
        content_type: "text".to_string(),
        content,
        width: 0,
        height: 0,
    })
}

/// 获取文件总页数
pub async fn get_total_pages(app_handle: &AppHandle, file_id: &str) -> Result<u32> {
    let storage_root = get_storage_root(app_handle);
    let meta_path = storage_root.join(file_id).join("meta.json");
    
    if !meta_path.exists() {
        return Err(anyhow!("文件不存在"));
    }
    
    let content = fs::read_to_string(&meta_path)?;
    let file_info: FileInfo = serde_json::from_str(&content)?;
    
    Ok(file_info.total_pages)
}
