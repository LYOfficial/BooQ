// AI 服务模块 - 处理大模型 API 调用

#![allow(dead_code)]

use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChoice {
    pub index: u32,
    pub message: ChatMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<ChatChoice>,
}

#[derive(Debug, Clone)]
pub struct AIService {
    client: Client,
    api_url: String,
    api_key: String,
    model_name: String,
}

impl AIService {
    pub fn new(api_url: &str, api_key: &str, model_name: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .unwrap();

        Self {
            client,
            api_url: api_url.to_string(),
            api_key: api_key.to_string(),
            model_name: model_name.to_string(),
        }
    }

    /// 发送聊天请求
    pub async fn chat(&self, messages: Vec<ChatMessage>) -> Result<String> {
        let request = ChatRequest {
            model: self.model_name.clone(),
            messages,
            temperature: Some(0.7),
            max_tokens: Some(4096),
            stream: Some(false),
        };

        let response = self
            .client
            .post(&self.api_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("API 请求失败: {}", error_text));
        }

        let chat_response: ChatResponse = response.json().await?;
        
        if let Some(choice) = chat_response.choices.first() {
            Ok(choice.message.content.clone())
        } else {
            Err(anyhow!("API 返回空响应"))
        }
    }

    /// 分析文本中的例题
    pub async fn analyze_examples(&self, text: &str) -> Result<String> {
        let system_prompt = r#"你是一个专业的教育内容分析助手。请分析以下文本，识别出其中的例题（带有完整答案或解析的题目）。

对于每道例题，请提取：
1. 题目内容
2. 答案或解析
3. 涉及的知识点
4. 所属章节（如果能识别）

请以 JSON 格式返回结果：
{
  "examples": [
    {
      "question": "题目内容",
      "answer": "答案内容",
      "analysis": "详细解析",
      "knowledge_points": ["知识点1", "知识点2"],
      "chapter": "章节名称",
      "section": "小节名称"
    }
  ]
}"#;

        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: format!("请分析以下文本中的例题：\n\n{}", text),
            },
        ];

        self.chat(messages).await
    }

    /// 分析文本中的课后习题
    pub async fn analyze_exercises(&self, text: &str, context: &str) -> Result<String> {
        let system_prompt = r#"你是一个专业的教育内容分析助手。请分析以下文本，识别出其中的课后习题（没有答案的练习题）。

参考以下知识点和例题上下文来解答这些题目。

对于每道习题，请提取并生成：
1. 题目内容
2. 详细答案（根据知识点和例题推理）
3. 解题思路分析
4. 涉及的知识点
5. 所属章节（如果能识别）

请以 JSON 格式返回结果：
{
  "exercises": [
    {
      "question": "题目内容",
      "answer": "生成的答案",
      "analysis": "详细解析",
      "knowledge_points": ["知识点1", "知识点2"],
      "chapter": "章节名称",
      "section": "小节名称"
    }
  ]
}"#;

        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: format!(
                    "参考上下文：\n{}\n\n请分析以下文本中的课后习题并给出答案：\n\n{}",
                    context, text
                ),
            },
        ];

        self.chat(messages).await
    }

    /// 生成题目答案
    pub async fn generate_answer(&self, question: &str, context: &str) -> Result<String> {
        let system_prompt = r#"你是一个专业的教育内容分析助手。请根据提供的知识点和上下文，为给定的题目生成详细的答案和解析。

请以 JSON 格式返回结果：
{
  "answer": "简洁的答案",
  "analysis": "详细的解题步骤和思路分析",
  "knowledge_points": ["涉及的知识点"]
}"#;

        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: format!(
                    "参考知识点和上下文：\n{}\n\n请为以下题目生成答案：\n\n{}",
                    context, question
                ),
            },
        ];

        self.chat(messages).await
    }

    /// 提取章节结构
    pub async fn extract_structure(&self, text: &str) -> Result<String> {
        let system_prompt = r#"你是一个专业的教育内容分析助手。请分析以下文本，识别出章节结构和主要知识点。

请以 JSON 格式返回结果：
{
  "chapters": [
    {
      "name": "章节名称",
      "sections": [
        {
          "name": "小节名称",
          "knowledge_points": ["知识点1", "知识点2"]
        }
      ]
    }
  ]
}"#;

        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: format!("请分析以下文本的章节结构：\n\n{}", text),
            },
        ];

        self.chat(messages).await
    }
}

/// 创建 AI 服务实例
pub fn create_ai_service(api_url: &str, api_key: &str, model_name: &str) -> AIService {
    AIService::new(api_url, api_key, model_name)
}
