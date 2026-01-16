// OCR 服务模块 - 处理文字识别和 Markdown 转换

#![allow(dead_code)]

use anyhow::{anyhow, Result};
use std::fs;
use std::path::PathBuf;
use tauri::AppHandle;
use base64::{Engine as _, engine::general_purpose};
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OCRRequest {
    pub image: String, // base64 encoded image
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OCRResult {
    pub text: String,
    pub confidence: f32,
    pub boxes: Vec<Vec<i32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OCRResponse {
    pub results: Vec<OCRResult>,
}

/// 获取文件存储路径
fn get_file_storage_path(app_handle: &AppHandle, file_id: &str) -> PathBuf {
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
    base_path.join(file_id)
}

/// 将页面转换为 Markdown
pub async fn convert_page_to_markdown(
    app_handle: &AppHandle,
    file_id: &str,
    page_number: u32,
) -> Result<String> {
    let file_path = get_file_storage_path(app_handle, file_id);
    let markdown_dir = file_path.join("markdown");
    
    // 检查是否已有缓存的 Markdown
    let md_file_name = format!("{:04}_page.md", page_number);
    let md_file_path = markdown_dir.join(&md_file_name);
    
    if md_file_path.exists() {
        return fs::read_to_string(&md_file_path).map_err(|e| anyhow!("读取缓存失败: {}", e));
    }
    
    // 读取文件元数据
    let meta_path = file_path.join("meta.json");
    let meta_content = fs::read_to_string(&meta_path)?;
    let file_info: crate::commands::FileInfo = serde_json::from_str(&meta_content)?;
    
    // 根据文件类型进行 OCR
    let markdown_content = match file_info.file_type.as_str() {
        "pdf" => convert_pdf_page_to_markdown(&file_info.path, page_number).await?,
        "txt" => {
            let content = fs::read_to_string(&file_info.path)?;
            content
        }
        _ => return Err(anyhow!("不支持的文件类型")),
    };
    
    // 保存 Markdown 到缓存
    fs::create_dir_all(&markdown_dir)?;
    fs::write(&md_file_path, &markdown_content)?;
    
    Ok(markdown_content)
}

/// 将 PDF 页面转换为 Markdown
async fn convert_pdf_page_to_markdown(file_path: &str, page_number: u32) -> Result<String> {
    // 这里使用简化的实现
    // 实际项目中需要：
    // 1. 将 PDF 页面渲染为图片
    // 2. 调用 OCR API（如 PaddleOCR）识别文字
    // 3. 将识别结果转换为 Markdown
    
    // 尝试提取 PDF 文本
    let doc = lopdf::Document::load(file_path)?;
    let pages = doc.get_pages();
    
    if let Some((_, &page_id)) = pages.iter().nth(page_number as usize - 1) {
        // 尝试提取文本
        let text = extract_pdf_text(&doc, page_id)?;
        if !text.trim().is_empty() {
            return Ok(format_as_markdown(&text));
        }
    }
    
    // 如果无法提取文本，返回占位符
    Ok(format!(
        "# 第 {} 页\n\n> 此页面需要 OCR 识别。请确保已配置 OCR 服务。\n\n*提示：可以在设置中配置 PaddleOCR 或其他 OCR 服务来识别扫描版 PDF。*",
        page_number
    ))
}

/// 提取 PDF 文本
fn extract_pdf_text(doc: &lopdf::Document, page_id: lopdf::ObjectId) -> Result<String> {
    // 简化的文本提取实现
    let mut text = String::new();
    
    if let Ok(content) = doc.get_page_content(page_id) {
        // 尝试解析内容流中的文本
        let content_str = String::from_utf8_lossy(&content);
        
        // 查找文本内容（简化处理）
        for line in content_str.lines() {
            if line.contains("Tj") || line.contains("TJ") {
                // 提取括号中的文本
                if let Some(start) = line.find('(') {
                    if let Some(end) = line.rfind(')') {
                        let extracted = &line[start + 1..end];
                        text.push_str(extracted);
                        text.push('\n');
                    }
                }
            }
        }
    }
    
    Ok(text)
}

/// 格式化为 Markdown
fn format_as_markdown(text: &str) -> String {
    let mut markdown = String::new();
    let lines: Vec<&str> = text.lines().collect();
    
    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            markdown.push_str("\n\n");
            continue;
        }
        
        // 识别标题（简单启发式）
        if trimmed.len() < 50 && !trimmed.ends_with('。') && !trimmed.ends_with(',') {
            // 可能是标题
            if trimmed.starts_with("第") && (trimmed.contains("章") || trimmed.contains("节")) {
                markdown.push_str(&format!("## {}\n\n", trimmed));
            } else if trimmed.parse::<f64>().is_err() && trimmed.chars().all(|c| !c.is_ascii_digit() || c == '.' || c == ' ' || c.is_alphabetic()) {
                markdown.push_str(&format!("### {}\n\n", trimmed));
            } else {
                markdown.push_str(trimmed);
                markdown.push_str("\n");
            }
        } else {
            markdown.push_str(trimmed);
            markdown.push_str("\n");
        }
    }
    
    markdown
}

/// 获取已缓存的 Markdown 内容
pub async fn get_markdown_content(
    app_handle: &AppHandle,
    file_id: &str,
    page_number: u32,
) -> Result<String> {
    let file_path = get_file_storage_path(app_handle, file_id);
    let markdown_dir = file_path.join("markdown");
    let md_file_name = format!("{:04}_page.md", page_number);
    let md_file_path = markdown_dir.join(&md_file_name);
    
    if md_file_path.exists() {
        fs::read_to_string(&md_file_path).map_err(|e| anyhow!("读取失败: {}", e))
    } else {
        // 如果没有缓存，执行转换
        convert_page_to_markdown(app_handle, file_id, page_number).await
    }
}

/// 获取 Markdown 源码
pub async fn get_markdown_source(
    app_handle: &AppHandle,
    file_id: &str,
    page_number: u32,
) -> Result<String> {
    // Markdown 源码就是 Markdown 内容本身
    get_markdown_content(app_handle, file_id, page_number).await
}

/// OCR 服务客户端
pub struct OCRClient {
    client: Client,
    api_url: String,
}

impl OCRClient {
    pub fn new(api_url: &str) -> Self {
        Self {
            client: Client::new(),
            api_url: api_url.to_string(),
        }
    }
    
    /// 识别图片中的文字
    pub async fn recognize(&self, image_data: &[u8]) -> Result<Vec<OCRResult>> {
        let base64_image = general_purpose::STANDARD.encode(image_data);
        
        let request = OCRRequest {
            image: base64_image,
        };
        
        let response = self
            .client
            .post(&self.api_url)
            .json(&request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(anyhow!("OCR 请求失败"));
        }
        
        let ocr_response: OCRResponse = response.json().await?;
        Ok(ocr_response.results)
    }
}
