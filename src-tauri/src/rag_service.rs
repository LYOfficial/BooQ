// RAG 服务模块 - 检索增强生成

#![allow(dead_code)]

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub content: String,
    pub metadata: DocumentMetadata,
    pub embedding: Option<Vec<f32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub file_id: String,
    pub page_number: u32,
    pub chunk_index: u32,
    pub doc_type: String, // "knowledge", "example", "exercise"
    pub chapter: String,
    pub section: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub document: Document,
    pub score: f32,
}

/// RAG 知识库
pub struct RAGStore {
    documents: Vec<Document>,
    index_path: PathBuf,
}

impl RAGStore {
    /// 创建新的 RAG 存储
    pub fn new(index_path: PathBuf) -> Self {
        let documents = if index_path.exists() {
            let content = fs::read_to_string(&index_path).unwrap_or_default();
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            Vec::new()
        };
        
        Self {
            documents,
            index_path,
        }
    }
    
    /// 添加文档
    pub fn add_document(&mut self, doc: Document) {
        // 检查是否已存在相同 ID 的文档
        if !self.documents.iter().any(|d| d.id == doc.id) {
            self.documents.push(doc);
            self.save().ok();
        }
    }
    
    /// 批量添加文档
    pub fn add_documents(&mut self, docs: Vec<Document>) {
        for doc in docs {
            self.add_document(doc);
        }
    }
    
    /// 搜索相关文档（基于关键词匹配的简单实现）
    pub fn search(&self, query: &str, top_k: usize) -> Vec<SearchResult> {
        let query_lower = query.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();
        
        let mut results: Vec<SearchResult> = self
            .documents
            .iter()
            .map(|doc| {
                let content_lower = doc.content.to_lowercase();
                
                // 计算匹配分数
                let mut score = 0.0f32;
                for word in &query_words {
                    if content_lower.contains(word) {
                        score += 1.0;
                    }
                }
                
                // 考虑文档类型权重
                let type_weight = match doc.metadata.doc_type.as_str() {
                    "example" => 1.5,
                    "knowledge" => 1.2,
                    "exercise" => 1.0,
                    _ => 0.8,
                };
                
                SearchResult {
                    document: doc.clone(),
                    score: score * type_weight,
                }
            })
            .filter(|r| r.score > 0.0)
            .collect();
        
        // 按分数排序
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        
        // 返回 top_k 结果
        results.into_iter().take(top_k).collect()
    }
    
    /// 按类型获取文档
    pub fn get_by_type(&self, doc_type: &str) -> Vec<&Document> {
        self.documents
            .iter()
            .filter(|d| d.metadata.doc_type == doc_type)
            .collect()
    }
    
    /// 按章节获取文档
    pub fn get_by_chapter(&self, chapter: &str) -> Vec<&Document> {
        self.documents
            .iter()
            .filter(|d| d.metadata.chapter == chapter)
            .collect()
    }
    
    /// 获取所有例题
    pub fn get_examples(&self) -> Vec<&Document> {
        self.get_by_type("example")
    }
    
    /// 获取所有知识点
    pub fn get_knowledge(&self) -> Vec<&Document> {
        self.get_by_type("knowledge")
    }
    
    /// 构建上下文
    pub fn build_context(&self, query: &str, max_tokens: usize) -> String {
        let results = self.search(query, 10);
        
        let mut context = String::new();
        let mut token_count = 0;
        
        for result in results {
            let doc_text = format!(
                "【{}】{}\n{}\n\n",
                result.document.metadata.doc_type,
                if !result.document.metadata.chapter.is_empty() {
                    format!("（{}）", result.document.metadata.chapter)
                } else {
                    String::new()
                },
                result.document.content
            );
            
            let doc_tokens = doc_text.len() / 4; // 粗略估计
            if token_count + doc_tokens > max_tokens {
                break;
            }
            
            context.push_str(&doc_text);
            token_count += doc_tokens;
        }
        
        context
    }
    
    /// 保存到文件
    fn save(&self) -> Result<()> {
        let content = serde_json::to_string_pretty(&self.documents)?;
        
        if let Some(parent) = self.index_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        fs::write(&self.index_path, content)?;
        Ok(())
    }
    
    /// 清空存储
    pub fn clear(&mut self) {
        self.documents.clear();
        self.save().ok();
    }
    
    /// 获取文档数量
    pub fn len(&self) -> usize {
        self.documents.len()
    }
    
    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }
}

/// 文本分块器
pub struct TextChunker {
    chunk_size: usize,
    overlap: usize,
}

impl TextChunker {
    pub fn new(chunk_size: usize, overlap: usize) -> Self {
        Self { chunk_size, overlap }
    }
    
    /// 将文本分割成块
    pub fn chunk(&self, text: &str) -> Vec<String> {
        let mut chunks = Vec::new();
        let chars: Vec<char> = text.chars().collect();
        
        let mut start = 0;
        while start < chars.len() {
            let end = (start + self.chunk_size).min(chars.len());
            let chunk: String = chars[start..end].iter().collect();
            
            if !chunk.trim().is_empty() {
                chunks.push(chunk);
            }
            
            if end >= chars.len() {
                break;
            }
            
            start = end - self.overlap;
        }
        
        chunks
    }
    
    /// 按段落分割
    pub fn chunk_by_paragraph(&self, text: &str) -> Vec<String> {
        let paragraphs: Vec<&str> = text.split("\n\n").collect();
        
        let mut chunks = Vec::new();
        let mut current_chunk = String::new();
        
        for para in paragraphs {
            if current_chunk.len() + para.len() > self.chunk_size {
                if !current_chunk.trim().is_empty() {
                    chunks.push(current_chunk.clone());
                }
                current_chunk = para.to_string();
            } else {
                if !current_chunk.is_empty() {
                    current_chunk.push_str("\n\n");
                }
                current_chunk.push_str(para);
            }
        }
        
        if !current_chunk.trim().is_empty() {
            chunks.push(current_chunk);
        }
        
        chunks
    }
}
