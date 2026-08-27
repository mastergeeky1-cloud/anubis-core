use crate::config::LlmConfig;
use crate::error::{AnubisError, Result};
use serde_json::Value;

/// Noxis Core — the local LLM "brain".
///
/// Talks to a LOCAL llama.cpp-compatible OpenAI-style endpoint
/// (e.g. `llama-cpp-server` or `ollama`). No data leaves the machine.
///
/// Endpoint used: POST {base_url}/v1/chat/completions
/// Body:   { "model", "messages":[{role,content}], "max_tokens", "temperature" }
/// Response: choices[0].message.content  (and we parse optional tool calls
///           out of the text as a simple JSON block, see `parse_tools`).
pub struct NoxisCore {
    client: reqwest::Client,
    cfg: LlmConfig,
    enabled: bool,
}

impl NoxisCore {
    pub fn new(cfg: LlmConfig) -> Self {
        let enabled = !cfg.base_url.trim().is_empty();
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .expect("reqwest client"),
            cfg,
            enabled,
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// Ask the brain a question. `history` is prior turns (oldest first).
    pub async fn ask(&self, user_text: &str, lang: &str) -> Result<String> {
        if !self.enabled {
            return Err(AnubisError::Llm("Noxis Core LLM URL not configured".into()));
        }
        let system = format!(
            "{}\nRespond in language code: {}. Be concise and helpful.",
            self.cfg.system_prompt, lang
        );
        let body = serde_json::json!({
            "model": self.cfg.model,
            "max_tokens": self.cfg.max_tokens,
            "temperature": 0.7,
            "messages": [
                { "role": "system", "content": system },
                { "role": "user", "content": user_text }
            ]
        });
        let resp = self
            .client
            .post(format!("{}/v1/chat/completions", self.cfg.base_url))
            .json(&body)
            .send()
            .await
            .map_err(|e| AnubisError::Llm(format!("request: {e}")))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let b = resp.text().await.unwrap_or_default();
            return Err(AnubisError::Llm(format!("server {status}: {b}")));
        }
        let json: Value = resp
            .json()
            .await
            .map_err(|e| AnubisError::Llm(format!("json: {e}")))?;
        let content = json
            .get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .to_string();
        if content.trim().is_empty() {
            return Err(AnubisError::Llm("empty response from brain".into()));
        }
        Ok(content)
    }
}
