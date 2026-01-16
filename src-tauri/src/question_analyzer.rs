// 题目分析模块 - 核心业务逻辑

use crate::{ai_service, config, ocr_service, rag_service};
use crate::commands::{AnalysisProgress, Question};
use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::AppHandle;
use once_cell::sync::Lazy;

// 分析状态存储
static ANALYSIS_STATE: Lazy<Arc<Mutex<HashMap<String, AnalysisState>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

#[derive(Debug, Clone)]
struct AnalysisState {
    progress: AnalysisProgress,
    should_stop: bool,
}

/// 获取文件存储路径
fn get_file_storage_path(app_handle: &AppHandle, file_id: &str) -> PathBuf {
    let config = config::get_config_sync(app_handle);
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

/// 开始分析
pub async fn start_analysis(app_handle: &AppHandle, file_id: &str) -> Result<()> {
    let file_path = get_file_storage_path(app_handle, file_id);
    
    // 检查文件是否存在
    let meta_path = file_path.join("meta.json");
    if !meta_path.exists() {
        return Err(anyhow!("文件不存在"));
    }
    
    // 读取文件元数据
    let meta_content = fs::read_to_string(&meta_path)?;
    let file_info: crate::commands::FileInfo = serde_json::from_str(&meta_content)?;
    
    // 初始化分析状态
    let initial_progress = AnalysisProgress {
        file_id: file_id.to_string(),
        status: "analyzing".to_string(),
        current_page: 0,
        total_pages: file_info.total_pages,
        current_step: "初始化".to_string(),
        questions_found: 0,
        message: "正在准备分析...".to_string(),
    };
    
    {
        let mut states = ANALYSIS_STATE.lock().unwrap();
        states.insert(
            file_id.to_string(),
            AnalysisState {
                progress: initial_progress,
                should_stop: false,
            },
        );
    }
    
    // 获取配置
    let app_config = config::get_config_sync(app_handle);
    
    // 创建 RAG 存储
    let rag_path = file_path.join("rag_index.json");
    let mut rag_store = rag_service::RAGStore::new(rag_path);
    
    // 创建问题存储目录
    let questions_dir = file_path.join("questions");
    fs::create_dir_all(&questions_dir)?;
    
    // 文本分块器
    let chunker = rag_service::TextChunker::new(1000, 100);
    
    let total_pages = file_info.total_pages;
    let batch_size = if total_pages > 400 { 20 } else { total_pages };
    
    let mut all_questions: Vec<Question> = Vec::new();
    let mut current_batch_start = 1u32;
    
    // 分批处理页面
    while current_batch_start <= total_pages {
        // 检查是否需要停止
        {
            let states = ANALYSIS_STATE.lock().unwrap();
            if let Some(state) = states.get(file_id) {
                if state.should_stop {
                    return Ok(());
                }
            }
        }
        
        let batch_end = (current_batch_start + batch_size - 1).min(total_pages);
        
        // 更新进度
        update_progress(
            file_id,
            "analyzing",
            current_batch_start,
            total_pages,
            &format!("正在分析第 {} - {} 页", current_batch_start, batch_end),
            all_questions.len() as u32,
        );
        
        // 处理当前批次的页面
        for page in current_batch_start..=batch_end {
            // 检查是否需要停止
            {
                let states = ANALYSIS_STATE.lock().unwrap();
                if let Some(state) = states.get(file_id) {
                    if state.should_stop {
                        return Ok(());
                    }
                }
            }
            
            // 获取页面的 Markdown 内容
            let markdown_content = ocr_service::convert_page_to_markdown(
                app_handle,
                file_id,
                page,
            )
            .await
            .unwrap_or_default();
            
            if markdown_content.trim().is_empty() {
                continue;
            }
            
            // 将内容添加到 RAG
            let chunks = chunker.chunk_by_paragraph(&markdown_content);
            for (i, chunk) in chunks.iter().enumerate() {
                let doc = rag_service::Document {
                    id: format!("{}_{}_{}", file_id, page, i),
                    content: chunk.clone(),
                    metadata: rag_service::DocumentMetadata {
                        file_id: file_id.to_string(),
                        page_number: page,
                        chunk_index: i as u32,
                        doc_type: "knowledge".to_string(),
                        chapter: String::new(),
                        section: String::new(),
                    },
                    embedding: None,
                };
                rag_store.add_document(doc);
            }
            
            // 更新进度
            update_progress(
                file_id,
                "analyzing",
                page,
                total_pages,
                &format!("正在识别第 {} 页的题目", page),
                all_questions.len() as u32,
            );
            
            // 使用 AI 分析页面内容，提取题目
            if let Some(model) = get_analysis_model(&app_config) {
                let ai_service = ai_service::create_ai_service(
                    &model.api_url,
                    &model.api_key,
                    &model.model_name,
                );
                
                // 分析例题
                if let Ok(examples_json) = ai_service.analyze_examples(&markdown_content).await {
                    if let Ok(questions) = parse_examples_response(&examples_json, file_id, page) {
                        for q in questions {
                            // 添加例题到 RAG
                            let doc = rag_service::Document {
                                id: q.id.clone(),
                                content: format!("题目：{}\n答案：{}", q.question_text, q.answer),
                                metadata: rag_service::DocumentMetadata {
                                    file_id: file_id.to_string(),
                                    page_number: page,
                                    chunk_index: 0,
                                    doc_type: "example".to_string(),
                                    chapter: q.chapter.clone(),
                                    section: q.section.clone(),
                                },
                                embedding: None,
                            };
                            rag_store.add_document(doc);
                            all_questions.push(q);
                        }
                    }
                }
                
                // 分析课后习题（使用 RAG 上下文）
                let context = rag_store.build_context(&markdown_content, 4000);
                if let Ok(exercises_json) = ai_service.analyze_exercises(&markdown_content, &context).await {
                    if let Ok(questions) = parse_exercises_response(&exercises_json, file_id, page) {
                        for q in questions {
                            all_questions.push(q);
                        }
                    }
                }
            }
        }
        
        current_batch_start = batch_end + 1;
    }
    
    // 保存所有问题
    let questions_json = serde_json::to_string_pretty(&all_questions)?;
    fs::write(questions_dir.join("all_questions.json"), questions_json)?;
    
    // 更新最终进度
    update_progress(
        file_id,
        "completed",
        total_pages,
        total_pages,
        "分析完成",
        all_questions.len() as u32,
    );
    
    Ok(())
}

/// 停止分析
pub async fn stop_analysis(_app_handle: &AppHandle, file_id: &str) -> Result<()> {
    let mut states = ANALYSIS_STATE.lock().unwrap();
    if let Some(state) = states.get_mut(file_id) {
        state.should_stop = true;
        state.progress.status = "stopped".to_string();
        state.progress.message = "分析已停止".to_string();
    }
    Ok(())
}

/// 获取分析进度
pub async fn get_analysis_progress(_app_handle: &AppHandle, file_id: &str) -> Result<AnalysisProgress> {
    let states = ANALYSIS_STATE.lock().unwrap();
    if let Some(state) = states.get(file_id) {
        Ok(state.progress.clone())
    } else {
        Ok(AnalysisProgress {
            file_id: file_id.to_string(),
            status: "idle".to_string(),
            current_page: 0,
            total_pages: 0,
            current_step: "".to_string(),
            questions_found: 0,
            message: "未开始分析".to_string(),
        })
    }
}

/// 获取题目列表
pub async fn get_questions(app_handle: &AppHandle, file_id: &str) -> Result<Vec<Question>> {
    let file_path = get_file_storage_path(app_handle, file_id);
    let questions_file = file_path.join("questions").join("all_questions.json");
    
    if questions_file.exists() {
        let content = fs::read_to_string(&questions_file)?;
        let questions: Vec<Question> = serde_json::from_str(&content)?;
        Ok(questions)
    } else {
        Ok(Vec::new())
    }
}

/// 获取题目详情
pub async fn get_question_detail(
    app_handle: &AppHandle,
    file_id: &str,
    question_id: &str,
) -> Result<Question> {
    let questions = get_questions(app_handle, file_id).await?;
    questions
        .into_iter()
        .find(|q| q.id == question_id)
        .ok_or_else(|| anyhow!("题目不存在"))
}

/// 更新进度
fn update_progress(
    file_id: &str,
    status: &str,
    current_page: u32,
    total_pages: u32,
    message: &str,
    questions_found: u32,
) {
    let mut states = ANALYSIS_STATE.lock().unwrap();
    if let Some(state) = states.get_mut(file_id) {
        state.progress.status = status.to_string();
        state.progress.current_page = current_page;
        state.progress.total_pages = total_pages;
        state.progress.message = message.to_string();
        state.progress.questions_found = questions_found;
    }
}

/// 获取分析模型配置
fn get_analysis_model(config: &crate::commands::AppConfig) -> Option<&crate::commands::ModelConfig> {
    config
        .models
        .iter()
        .find(|m| m.id == config.analysis_model)
        .or_else(|| config.models.first())
}

/// 解析例题响应
fn parse_examples_response(json_str: &str, file_id: &str, page: u32) -> Result<Vec<Question>> {
    #[derive(Deserialize)]
    struct ExamplesResponse {
        examples: Vec<ExampleItem>,
    }
    
    #[derive(Deserialize)]
    struct ExampleItem {
        question: String,
        answer: String,
        analysis: Option<String>,
        knowledge_points: Option<Vec<String>>,
        chapter: Option<String>,
        section: Option<String>,
    }
    
    // 尝试提取 JSON
    let json_str = extract_json(json_str);
    
    let response: ExamplesResponse = serde_json::from_str(&json_str)?;
    
    let questions: Vec<Question> = response
        .examples
        .into_iter()
        .enumerate()
        .map(|(i, item)| Question {
            id: format!("{}_{}_example_{}", file_id, page, i),
            file_id: file_id.to_string(),
            question_type: "example".to_string(),
            chapter: item.chapter.unwrap_or_default(),
            section: item.section.unwrap_or_default(),
            knowledge_points: item.knowledge_points.unwrap_or_default(),
            question_text: item.question,
            answer: item.answer,
            analysis: item.analysis.unwrap_or_default(),
            page_number: page,
            has_original_answer: true,
        })
        .collect();
    
    Ok(questions)
}

/// 解析习题响应
fn parse_exercises_response(json_str: &str, file_id: &str, page: u32) -> Result<Vec<Question>> {
    #[derive(Deserialize)]
    struct ExercisesResponse {
        exercises: Vec<ExerciseItem>,
    }
    
    #[derive(Deserialize)]
    struct ExerciseItem {
        question: String,
        answer: String,
        analysis: Option<String>,
        knowledge_points: Option<Vec<String>>,
        chapter: Option<String>,
        section: Option<String>,
    }
    
    // 尝试提取 JSON
    let json_str = extract_json(json_str);
    
    let response: ExercisesResponse = serde_json::from_str(&json_str)?;
    
    let questions: Vec<Question> = response
        .exercises
        .into_iter()
        .enumerate()
        .map(|(i, item)| Question {
            id: format!("{}_{}_exercise_{}", file_id, page, i),
            file_id: file_id.to_string(),
            question_type: "exercise".to_string(),
            chapter: item.chapter.unwrap_or_default(),
            section: item.section.unwrap_or_default(),
            knowledge_points: item.knowledge_points.unwrap_or_default(),
            question_text: item.question,
            answer: item.answer,
            analysis: item.analysis.unwrap_or_default(),
            page_number: page,
            has_original_answer: false,
        })
        .collect();
    
    Ok(questions)
}

/// 从字符串中提取 JSON
fn extract_json(text: &str) -> String {
    // 尝试找到 JSON 对象
    if let Some(start) = text.find('{') {
        if let Some(end) = text.rfind('}') {
            return text[start..=end].to_string();
        }
    }
    text.to_string()
}
