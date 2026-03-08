use acp_core::AcpError;
use async_trait::async_trait;

use crate::provider::EmbeddingProvider;

#[derive(Debug, Clone)]
pub struct OpenAIConfig {
    pub api_key: String,
    pub model: OpenAIModel,
    pub base_url: String,
    pub timeout_secs: u64,
}

#[derive(Debug, Clone, Copy)]
pub enum OpenAIModel {
    TextEmbedding3Small,
    TextEmbedding3Large,
    Ada002,
}

impl OpenAIModel {
    pub fn id(&self) -> &str {
        match self {
            Self::TextEmbedding3Small => "text-embedding-3-small",
            Self::TextEmbedding3Large => "text-embedding-3-large",
            Self::Ada002 => "text-embedding-ada-002",
        }
    }

    pub fn dimensions(&self) -> usize {
        match self {
            Self::TextEmbedding3Small => 1536,
            Self::TextEmbedding3Large => 3072,
            Self::Ada002 => 1536,
        }
    }
}

impl Default for OpenAIConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            model: OpenAIModel::TextEmbedding3Small,
            base_url: "https://api.openai.com/v1".into(),
            timeout_secs: 30,
        }
    }
}

pub struct OpenAIEmbeddings {
    config: OpenAIConfig,
    client: reqwest::Client,
}

impl OpenAIEmbeddings {
    pub fn new(config: OpenAIConfig) -> Result<Self, AcpError> {
        if config.api_key.is_empty() {
            return Err(AcpError::Internal("OPENAI_API_KEY is required".into()));
        }

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(|e| AcpError::Internal(e.to_string()))?;

        Ok(Self { config, client })
    }
}

#[async_trait]
impl EmbeddingProvider for OpenAIEmbeddings {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, AcpError> {
        let response = self
            .client
            .post(format!("{}/embeddings", self.config.base_url))
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .json(&serde_json::json!({
                "model": self.config.model.id(),
                "input": text,
            }))
            .send()
            .await
            .map_err(|e| AcpError::Internal(format!("OpenAI API error: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(AcpError::Internal(format!(
                "OpenAI API error {}: {}",
                status, error_text
            )));
        }

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AcpError::Internal(e.to_string()))?;

        body["data"][0]["embedding"]
            .as_array()
            .ok_or_else(|| AcpError::Internal("Invalid embedding response".into()))
            .map(|arr| {
                arr.iter()
                    .map(|v| v.as_f64().unwrap_or(0.0) as f32)
                    .collect()
            })
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, AcpError> {
        let response = self
            .client
            .post(format!("{}/embeddings", self.config.base_url))
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .json(&serde_json::json!({
                "model": self.config.model.id(),
                "input": texts,
            }))
            .send()
            .await
            .map_err(|e| AcpError::Internal(format!("OpenAI API error: {}", e)))?;

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AcpError::Internal(e.to_string()))?;

        body["data"]
            .as_array()
            .ok_or_else(|| AcpError::Internal("Invalid batch response".into()))
            .map(|arr| {
                arr.iter()
                    .map(|item| {
                        item["embedding"]
                            .as_array()
                            .unwrap_or(&Vec::new())
                            .iter()
                            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
                            .collect()
                    })
                    .collect()
            })
    }

    fn dimensions(&self) -> usize {
        self.config.model.dimensions()
    }

    fn model_id(&self) -> &str {
        self.config.model.id()
    }
}
