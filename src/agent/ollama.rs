use anyhow::{Context, Result};
use futures_util::StreamExt;
use reqwest::Client;
use serde::Deserialize;
use tracing::{debug, instrument, warn};

use crate::models::{ChatOptions, ChatRequest, ChatResponse, Message};

const MAX_RETRIES: u32 = 3;
const RETRY_BASE_MS: u64 = 1000; // exponential backoff base

/// Streaming chunk (stream=true response)
#[derive(Debug, Deserialize)]
pub struct StreamChunk {
    pub message: Message,
    pub done: bool,
}

/// Ollama API client
pub struct OllamaClient {
    client: Client,
    base_url: String,
    model: String,
}

impl OllamaClient {
    pub fn new(base_url: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(600)) // 10 min (allow long responses)
                .build()
                .expect("Failed to create HTTP client"),
            base_url: base_url.into(),
            model: model.into(),
        }
    }

    /// Create a client from environment variables
    pub fn from_env() -> Self {
        let base_url = std::env::var("OLLAMA_API_URL")
            .unwrap_or_else(|_| "http://localhost:11434".to_string());
        let model = std::env::var("OLLAMA_MODEL")
            .unwrap_or_else(|_| "gemma4:e4b".to_string());
        Self::new(base_url, model)
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    /// Health-check the Ollama server
    pub async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/api/tags", self.base_url);
        Ok(self.client.get(&url).send().await.is_ok())
    }

    /// List available models
    pub async fn list_models(&self) -> Result<Vec<String>> {
        let url = format!("{}/api/tags", self.base_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to connect to Ollama server")?;

        let body: serde_json::Value = resp.json().await.context("Failed to parse response")?;
        let models = body["models"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m["name"].as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        Ok(models)
    }

    /// Chat completion request (non-streaming, with retries)
    #[instrument(skip(self, messages), fields(model = %self.model, msg_count = messages.len()))]
    pub async fn chat(&self, messages: Vec<Message>) -> Result<ChatResponse> {
        let url = format!("{}/api/chat", self.base_url);
        let request = ChatRequest {
            model: self.model.clone(),
            messages,
            stream: false,
            options: Some(ChatOptions {
                temperature: Some(0.7),
                num_predict: Some(4096),
                ..Default::default()
            }),
        };

        let mut last_err = anyhow::anyhow!("Retry limit exceeded");
        for attempt in 0..MAX_RETRIES {
            if attempt > 0 {
                let delay = RETRY_BASE_MS * (1 << attempt);
                warn!("Ollama retry {}/{} (after {}ms)", attempt, MAX_RETRIES, delay);
                tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
            }

            debug!("Sending Ollama request (attempt {})", attempt + 1);
            let resp = match self.client.post(&url).json(&request).send().await {
                Ok(r) => r,
                Err(e) => { last_err = anyhow::anyhow!("Connection failed: {}", e); continue; }
            };

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                // retry on 5xx, fail immediately on 4xx
                if status.is_server_error() {
                    last_err = anyhow::anyhow!("Ollama server error {}: {}", status, body);
                    continue;
                }
                anyhow::bail!("Ollama API error {}: {}", status, body);
            }

            let chat_resp: ChatResponse = resp.json().await.context("Failed to parse response JSON")?;
            debug!("Response received");
            return Ok(chat_resp);
        }
        Err(last_err)
    }

    /// Single-turn non-streaming request (helper)
    #[allow(dead_code)]
    pub async fn chat_simple(&self, prompt: &str) -> Result<String> {
        let msgs = vec![Message::user(prompt)];
        let resp = self.chat(msgs).await?;
        Ok(resp.message.content)
    }

    /// Streaming chat request — calls the callback for each token as it arrives (with retries)
    /// Returns the full response text
    pub async fn chat_stream<F>(
        &self,
        messages: Vec<Message>,
        mut on_token: F,
    ) -> Result<String>
    where
        F: FnMut(&str),
    {
        let url = format!("{}/api/chat", self.base_url);
        let request = ChatRequest {
            model: self.model.clone(),
            messages,
            stream: true,
            options: Some(ChatOptions {
                temperature: Some(0.7),
                num_predict: Some(4096),
                ..Default::default()
            }),
        };

        let mut last_err = anyhow::anyhow!("Retry limit exceeded");
        for attempt in 0..MAX_RETRIES {
            if attempt > 0 {
                let delay = RETRY_BASE_MS * (1 << attempt);
                warn!("Streaming retry {}/{} (after {}ms)", attempt, MAX_RETRIES, delay);
                tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
            }

            let resp = match self.client.post(&url).json(&request).send().await {
                Ok(r) => r,
                Err(e) => { last_err = anyhow::anyhow!("Connection failed: {}", e); continue; }
            };

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                if status.is_server_error() {
                    last_err = anyhow::anyhow!("Server error {}: {}", status, body);
                    continue;
                }
                anyhow::bail!("Ollama API error {}: {}", status, body);
            }

            let mut stream = resp.bytes_stream();
            let mut full_content = String::new();
            let mut line_buf = String::new();

            while let Some(item) = stream.next().await {
                let bytes = item.context("Stream read error")?;
                line_buf.push_str(&String::from_utf8_lossy(&bytes));

                while let Some(nl) = line_buf.find('\n') {
                    let line = line_buf[..nl].trim().to_string();
                    line_buf = line_buf[nl + 1..].to_string();

                    if line.is_empty() { continue; }

                    if let Ok(chunk) = serde_json::from_str::<StreamChunk>(&line) {
                        let tok = &chunk.message.content;
                        if !tok.is_empty() {
                            on_token(tok);
                            full_content.push_str(tok);
                        }
                        if chunk.done { break; }
                    }
                }
            }

            return Ok(full_content);
        }
        Err(last_err)
    }
}
