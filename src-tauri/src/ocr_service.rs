// OCR 服务模块 - 处理文字识别和 Markdown 转换
// 支持 PaddleOCR-VL API

#![allow(dead_code)]

use anyhow::{anyhow, Result};
use std::env;
use std::fs;
use std::path::PathBuf;
use tauri::AppHandle;
use base64::{Engine as _, engine::general_purpose};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use lopdf::{Document, dictionary};

// ==================== PaddleOCR-VL API 数据结构 ====================

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaddleOCRRequest {
    pub file: String,  // base64 encoded file
    pub file_type: i32,  // 0 for PDF, 1 for image
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_doc_orientation_classify: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_doc_unwarping: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_chart_recognition: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PaddleOCRResponse {
    pub result: PaddleOCRResult,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaddleOCRResult {
    pub layout_parsing_results: Vec<LayoutParsingResult>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutParsingResult {
    pub markdown: MarkdownResult,
    #[serde(default)]
    pub output_images: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MarkdownResult {
    pub text: String,
    #[serde(default)]
    pub images: std::collections::HashMap<String, String>,
}

// ==================== 旧版 OCR 数据结构（兼容） ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OCRRequest {
    pub image: String,
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

// ==================== PaddleOCR-VL 客户端 ====================

pub struct PaddleOCRClient {
    client: Client,
    api_url: String,
    token: String,
}

impl PaddleOCRClient {
    /// 从环境变量创建客户端
    pub fn from_env() -> Result<Self> {
        // 加载 .env 文件（如果存在）
        let _ = dotenvy::dotenv();
        
        let api_url = env::var("PADDLE_OCR_API_URL")
            .map_err(|_| anyhow!("未设置 PADDLE_OCR_API_URL 环境变量"))?;
        let token = env::var("PADDLE_OCR_TOKEN")
            .map_err(|_| anyhow!("未设置 PADDLE_OCR_TOKEN 环境变量"))?;
        
        Ok(Self {
            client: Client::new(),
            api_url,
            token,
        })
    }
    
    /// 使用指定的 URL 和 Token 创建客户端
    pub fn new(api_url: &str, token: &str) -> Self {
        Self {
            client: Client::new(),
            api_url: api_url.to_string(),
            token: token.to_string(),
        }
    }
    
    /// 检查 API 配置是否可用
    pub fn is_configured() -> bool {
        let _ = dotenvy::dotenv();
        env::var("PADDLE_OCR_API_URL").is_ok() && env::var("PADDLE_OCR_TOKEN").is_ok()
    }
    
    /// 解析 PDF 文件，返回 Markdown 内容
    pub async fn parse_pdf(&self, file_path: &str) -> Result<Vec<LayoutParsingResult>> {
        let file_bytes = fs::read(file_path)?;
        self.parse_file_bytes(&file_bytes, 0).await
    }
    
    /// 解析 PDF 单页，返回 Markdown 内容
    pub async fn parse_pdf_page(&self, file_path: &str, page_number: u32) -> Result<LayoutParsingResult> {
        // 提取单页 PDF
        let single_page_bytes = extract_pdf_single_page(file_path, page_number)?;
        
        // 发送给 OCR API
        let results = self.parse_file_bytes(&single_page_bytes, 0).await?;
        
        // 返回第一个结果（单页 PDF 只有一个结果）
        results.into_iter().next()
            .ok_or_else(|| anyhow!("OCR 返回结果为空"))
    }
    
    /// 解析图片文件，返回 Markdown 内容
    pub async fn parse_image(&self, file_path: &str) -> Result<Vec<LayoutParsingResult>> {
        let file_bytes = fs::read(file_path)?;
        self.parse_file_bytes(&file_bytes, 1).await
    }
    
    /// 解析文件字节数据
    pub async fn parse_file_bytes(&self, file_bytes: &[u8], file_type: i32) -> Result<Vec<LayoutParsingResult>> {
        let file_data = general_purpose::STANDARD.encode(file_bytes);
        
        let request = PaddleOCRRequest {
            file: file_data,
            file_type,
            use_doc_orientation_classify: Some(false),
            use_doc_unwarping: Some(false),
            use_chart_recognition: Some(false),
        };
        
        let response = self
            .client
            .post(&self.api_url)
            .header("Authorization", format!("token {}", self.token))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow!("PaddleOCR API 请求失败: {} - {}", status, error_text));
        }
        
        let ocr_response: PaddleOCRResponse = response.json().await?;
        Ok(ocr_response.result.layout_parsing_results)
    }
    
    /// 解析 PDF 并保存 Markdown 和图片
    pub async fn parse_and_save(&self, file_path: &str, output_dir: &PathBuf) -> Result<Vec<String>> {
        let results = self.parse_pdf(file_path).await?;
        let mut markdown_files = Vec::new();
        
        fs::create_dir_all(output_dir)?;
        
        for (i, res) in results.iter().enumerate() {
            // 保存 Markdown 文件
            let md_filename = output_dir.join(format!("{:04}_page.md", i + 1));
            fs::write(&md_filename, &res.markdown.text)?;
            markdown_files.push(md_filename.to_string_lossy().to_string());
            
            // 下载并保存 Markdown 中的图片
            for (img_path, img_url) in &res.markdown.images {
                let full_img_path = output_dir.join(img_path);
                if let Some(parent) = full_img_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                
                if let Ok(img_response) = self.client.get(img_url).send().await {
                    if let Ok(img_bytes) = img_response.bytes().await {
                        let _ = fs::write(&full_img_path, &img_bytes);
                    }
                }
            }
            
            // 下载并保存输出图片
            for (img_name, img_url) in &res.output_images {
                let filename = output_dir.join(format!("{}_{}.jpg", img_name, i));
                if let Ok(img_response) = self.client.get(img_url).send().await {
                    if let Ok(img_bytes) = img_response.bytes().await {
                        let _ = fs::write(&filename, &img_bytes);
                    }
                }
            }
        }
        
        Ok(markdown_files)
    }
}

// ==================== 辅助函数 ====================

/// 从 PDF 中提取单页，返回单页 PDF 的字节数据
fn extract_pdf_single_page(file_path: &str, page_number: u32) -> Result<Vec<u8>> {
    let doc = Document::load(file_path)?;
    let pages = doc.get_pages();
    let total_pages = pages.len() as u32;
    
    if page_number == 0 || page_number > total_pages {
        return Err(anyhow!("页码 {} 超出范围 (1-{})", page_number, total_pages));
    }
    
    // 获取目标页面的对象 ID
    let target_page_id = pages.iter()
        .nth(page_number as usize - 1)
        .map(|(_, &id)| id)
        .ok_or_else(|| anyhow!("无法找到第 {} 页", page_number))?;
    
    // 创建新的单页 PDF
    let mut new_doc = Document::with_version("1.5");
    
    // 复制目标页面需要的所有对象
    let mut object_map = std::collections::HashMap::new();
    copy_object_recursive(&doc, &mut new_doc, target_page_id, &mut object_map)?;
    
    // 获取新文档中的页面 ID
    let new_page_id = *object_map.get(&target_page_id)
        .ok_or_else(|| anyhow!("复制页面失败"))?;
    
    // 创建页面树
    let pages_id = new_doc.add_object(lopdf::dictionary! {
        "Type" => "Pages",
        "Kids" => vec![new_page_id.into()],
        "Count" => 1,
    });
    
    // 更新页面的 Parent 引用
    if let Ok(page_dict) = new_doc.get_object_mut(new_page_id) {
        if let lopdf::Object::Dictionary(ref mut dict) = page_dict {
            dict.set("Parent", pages_id);
        }
    }
    
    // 创建文档目录
    let catalog_id = new_doc.add_object(lopdf::dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    
    // 设置文档 trailer
    new_doc.trailer.set("Root", catalog_id);
    
    // 保存到内存缓冲区
    let mut buffer = Vec::new();
    new_doc.save_to(&mut buffer)?;
    
    Ok(buffer)
}

/// 递归复制 PDF 对象
fn copy_object_recursive(
    src_doc: &Document,
    dst_doc: &mut Document,
    obj_id: lopdf::ObjectId,
    object_map: &mut std::collections::HashMap<lopdf::ObjectId, lopdf::ObjectId>,
) -> Result<lopdf::ObjectId> {
    // 如果已经复制过，直接返回
    if let Some(&new_id) = object_map.get(&obj_id) {
        return Ok(new_id);
    }
    
    // 获取源对象
    let obj = src_doc.get_object(obj_id)?;
    
    // 深度复制对象，处理引用
    let new_obj = copy_object_value(src_doc, dst_doc, obj.clone(), object_map)?;
    
    // 添加到新文档
    let new_id = dst_doc.add_object(new_obj);
    object_map.insert(obj_id, new_id);
    
    Ok(new_id)
}

/// 复制 PDF 对象值，递归处理引用
fn copy_object_value(
    src_doc: &Document,
    dst_doc: &mut Document,
    obj: lopdf::Object,
    object_map: &mut std::collections::HashMap<lopdf::ObjectId, lopdf::ObjectId>,
) -> Result<lopdf::Object> {
    match obj {
        lopdf::Object::Reference(ref_id) => {
            // 递归复制引用的对象
            let new_id = copy_object_recursive(src_doc, dst_doc, ref_id, object_map)?;
            Ok(lopdf::Object::Reference(new_id))
        }
        lopdf::Object::Array(arr) => {
            let mut new_arr = Vec::new();
            for item in arr {
                new_arr.push(copy_object_value(src_doc, dst_doc, item, object_map)?);
            }
            Ok(lopdf::Object::Array(new_arr))
        }
        lopdf::Object::Dictionary(dict) => {
            let mut new_dict = lopdf::Dictionary::new();
            for (key, value) in dict.into_iter() {
                // 跳过 Parent 引用，我们会在后面设置
                if key.as_slice() != b"Parent" {
                    new_dict.set(key.clone(), copy_object_value(src_doc, dst_doc, value.clone(), object_map)?);
                }
            }
            Ok(lopdf::Object::Dictionary(new_dict))
        }
        lopdf::Object::Stream(stream) => {
            let mut new_dict = lopdf::Dictionary::new();
            for (key, value) in stream.dict.into_iter() {
                new_dict.set(key.clone(), copy_object_value(src_doc, dst_doc, value.clone(), object_map)?);
            }
            Ok(lopdf::Object::Stream(lopdf::Stream::new(new_dict, stream.content)))
        }
        // 其他类型直接返回
        other => Ok(other),
    }
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

/// 将页面转换为 Markdown（使用 PaddleOCR-VL）
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
    
    // 根据文件类型进行处理
    let markdown_content = match file_info.file_type.as_str() {
        "pdf" => {
            // 尝试使用 PaddleOCR-VL
            if PaddleOCRClient::is_configured() {
                convert_pdf_with_paddle_ocr(&file_info.path, &markdown_dir, page_number).await?
            } else {
                // 回退到简单文本提取
                convert_pdf_page_to_markdown(&file_info.path, page_number).await?
            }
        }
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

/// 使用 PaddleOCR-VL 转换 PDF 单页
async fn convert_pdf_with_paddle_ocr(
    file_path: &str,
    output_dir: &PathBuf,
    page_number: u32,
) -> Result<String> {
    let client = PaddleOCRClient::from_env()?;
    
    // 只解析请求的单页
    let result = client.parse_pdf_page(file_path, page_number).await?;
    
    // 规范化 LaTeX 代码
    let normalized_content = normalize_latex(&result.markdown.text);
    
    // 保存当前页面（规范化后的内容）
    fs::create_dir_all(output_dir)?;
    let md_filename = output_dir.join(format!("{:04}_page.md", page_number));
    fs::write(&md_filename, &normalized_content)?;
    
    // 下载并保存图片
    for (img_path, img_url) in &result.markdown.images {
        let full_img_path = output_dir.join(img_path);
        if let Some(parent) = full_img_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        if let Ok(img_response) = client.client.get(img_url).send().await {
            if let Ok(img_bytes) = img_response.bytes().await {
                let _ = fs::write(&full_img_path, &img_bytes);
            }
        }
    }
    
    Ok(normalized_content)
}

/// 规范化 LaTeX 代码
fn normalize_latex(markdown: &str) -> String {
    use regex::Regex;
    
    let mut result = markdown.to_string();
    
    // 1. 修复独立的 \begin{...} 块，确保被 $$ 包裹
    let block_envs = ["aligned", "equation", "gather", "align", "split", "cases", "matrix", "pmatrix", "bmatrix", "vmatrix", "array"];
    
    for env in block_envs {
        // 匹配 \begin{env}...\end{env} 块
        let pattern = format!(r"\\begin\{{{}\}}([\s\S]*?)\\end\{{{}\}}", env, env);
        if let Ok(re) = Regex::new(&pattern) {
            result = re.replace_all(&result, |caps: &regex::Captures| {
                let content = &caps[0];
                // 检查是否已经被 $$ 包裹
                let before_pos = result.find(content).unwrap_or(0);
                let before = &result[..before_pos];
                let is_wrapped = before.ends_with("$$") || before.ends_with("$$ ") || before.ends_with("$$\n");
                
                if !is_wrapped {
                    format!("\n$$\n{}\n$$\n", content)
                } else {
                    content.to_string()
                }
            }).to_string();
        }
    }
    
    // 2. 清理多余的 $$ 符号
    result = result.replace("$$$$", "$$");
    result = result.replace("$$\n$$", "$$");
    
    // 3. 修复公式中的中文标点
    if let Ok(re) = Regex::new(r"(\$[^$]+)。([^$]*\$)") {
        result = re.replace_all(&result, "$1.$2").to_string();
    }
    if let Ok(re) = Regex::new(r"(\$[^$]+)，([^$]*\$)") {
        result = re.replace_all(&result, "$1,$2").to_string();
    }
    
    // 4. 修复百分号（在数字后）
    if let Ok(re) = Regex::new(r"(\d)%(?!\s*\$)") {
        result = re.replace_all(&result, r"$1\%").to_string();
    }
    if let Ok(re) = Regex::new(r"(\d)％") {
        result = re.replace_all(&result, r"$1\%").to_string();
    }
    
    // 5. 确保 $$ 块前后有换行
    if let Ok(re) = Regex::new(r"([^\n\s])\$\$") {
        result = re.replace_all(&result, "$1\n$$").to_string();
    }
    if let Ok(re) = Regex::new(r"\$\$([^\n\s$])") {
        result = re.replace_all(&result, "$$\n$1").to_string();
    }
    
    // 6. 清理多余的空行
    if let Ok(re) = Regex::new(r"\n{3,}") {
        result = re.replace_all(&result, "\n\n").to_string();
    }
    
    // 7. 修复 \times 格式
    if let Ok(re) = Regex::new(r"\\times(\d)") {
        result = re.replace_all(&result, r"\times $1").to_string();
    }
    if let Ok(re) = Regex::new(r"(\d)\\times") {
        result = re.replace_all(&result, r"$1 \times").to_string();
    }
    
    result
}

/// 将 PDF 页面转换为 Markdown（简单文本提取，不使用 OCR）
async fn convert_pdf_page_to_markdown(file_path: &str, page_number: u32) -> Result<String> {
    // 尝试提取 PDF 文本
    let doc = lopdf::Document::load(file_path)?;
    let pages = doc.get_pages();
    
    if let Some((_, &page_id)) = pages.iter().nth(page_number as usize - 1) {
        let text = extract_pdf_text(&doc, page_id)?;
        if !text.trim().is_empty() {
            return Ok(format_as_markdown(&text));
        }
    }
    
    // 如果无法提取文本，返回提示
    Ok(format!(
        "# 第 {} 页\n\n> 此页面需要 OCR 识别。\n\n*提示：请在 `.env` 文件中配置 PaddleOCR-VL API 以启用智能文档解析。*\n\n```\nPADDLE_OCR_API_URL=your-api-url\nPADDLE_OCR_TOKEN=your-token\n```",
        page_number
    ))
}

/// 提取 PDF 文本
fn extract_pdf_text(doc: &lopdf::Document, page_id: lopdf::ObjectId) -> Result<String> {
    let mut text = String::new();
    
    if let Ok(content) = doc.get_page_content(page_id) {
        let content_str = String::from_utf8_lossy(&content);
        
        for line in content_str.lines() {
            if line.contains("Tj") || line.contains("TJ") {
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
        
        // 识别标题
        if trimmed.len() < 50 && !trimmed.ends_with('。') && !trimmed.ends_with(',') {
            if trimmed.starts_with("第") && (trimmed.contains("章") || trimmed.contains("节")) {
                markdown.push_str(&format!("## {}\n\n", trimmed));
            } else {
                markdown.push_str(trimmed);
                markdown.push('\n');
            }
        } else {
            markdown.push_str(trimmed);
            markdown.push('\n');
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
        convert_page_to_markdown(app_handle, file_id, page_number).await
    }
}

/// 获取 Markdown 源码
pub async fn get_markdown_source(
    app_handle: &AppHandle,
    file_id: &str,
    page_number: u32,
) -> Result<String> {
    get_markdown_content(app_handle, file_id, page_number).await
}

/// 清除 Markdown 缓存
pub async fn clear_markdown_cache(
    app_handle: &AppHandle,
    file_id: &str,
    page_number: Option<u32>,
) -> Result<()> {
    let file_path = get_file_storage_path(app_handle, file_id);
    let markdown_dir = file_path.join("markdown");
    
    if !markdown_dir.exists() {
        return Ok(());
    }
    
    match page_number {
        Some(page) => {
            // 删除指定页面的缓存
            let md_file_name = format!("{:04}_page.md", page);
            let md_file_path = markdown_dir.join(&md_file_name);
            if md_file_path.exists() {
                fs::remove_file(&md_file_path)?;
            }
        }
        None => {
            // 删除所有缓存
            fs::remove_dir_all(&markdown_dir)?;
        }
    }
    
    Ok(())
}

/// 旧版 OCR 服务客户端（兼容保留）
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
