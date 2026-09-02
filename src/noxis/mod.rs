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
///
/// `base_url` may be given with or without a trailing `/v1` (e.g. both
/// `https://api.omniroute.ai` and `https://api.omniroute.ai/v1` are accepted);
/// callers may also pass an OpenAI-style base ending in `/v1`.
fn normalize_base(base: &str) -> String {
    base.trim_end_matches('/').to_string()
}

/// Build the chat-completions URL, tolerating either `host` or `host/v1`.
fn chat_url(base: &str) -> String {
    let b = normalize_base(base);
    if b.ends_with("/v1") || b.ends_with("/v1/") {
        format!("{b}/chat/completions")
    } else {
        format!("{b}/v1/chat/completions")
    }
}
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

    /// System prompt for a given user language and teacher mode.
    fn system_for(&self, lang: &str, teacher_mode: bool) -> String {
        let base = self.cfg.system_prompt.trim();
        let lang_instruction = format!("Respond in language code: {}.", lang);

        if teacher_mode {
            // Teacher mode system prompt with multiple behaviors
            format!(
                "{}\n{}\n\n{}",
                base,
                lang_instruction,
                "You are a patient, expert teacher. Adapt your teaching to the student:\n\
                 - EXPLAIN: Break down concepts clearly with examples in the student's language\n\
                 - SOCRATIC: Ask guiding questions to help them discover answers\n\
                 - PRACTICE: Give exercises, quizzes, or problems appropriate to their level\n\
                 - FEEDBACK: Correct mistakes constructively, explain WHY it's wrong\n\
                 - ENCOURAGE: Celebrate progress, normalize struggle, growth mindset\n\
                 - ADAPT: Detect their level; simplify if lost, deepen if bored\n\
                 - STRUCTURE: Use clear steps, summaries, and check understanding\n\
                 Never just give answers — guide them to learn."
            )
        } else {
            format!("{}\n{} Be concise and helpful.", base, lang_instruction)
        }
    }

    fn build_messages<'a>(
        &'a self,
        user_text: &'a str,
        lang: &str,
        history: &'a [(String, String)],
        teacher_mode: bool,
    ) -> Vec<serde_json::Value> {
        let mut messages = vec![serde_json::json!({
            "role": "system",
            "content": self.system_for(lang, teacher_mode),
        })];
        // history: (role, content), oldest first. Role is "user" or "assistant".
        for (role, content) in history {
            messages.push(serde_json::json!({ "role": role, "content": content }));
        }
        messages.push(serde_json::json!({ "role": "user", "content": user_text }));
        messages
    }

    fn chat_body(
        &self,
        user_text: &str,
        lang: &str,
        history: &[(String, String)],
        teacher_mode: bool,
        stream: bool,
    ) -> serde_json::Value {
        serde_json::json!({
            "model": self.cfg.model,
            "max_tokens": self.cfg.max_tokens,
            "temperature": 0.7,
            "stream": stream,
            "messages": self.build_messages(user_text, lang, history, teacher_mode),
        })
    }

    /// Non-streaming ask, honoring conversation `history` (oldest first),
    /// where each entry is `(role, content)`.
    pub async fn ask(
        &self,
        user_text: &str,
        lang: &str,
        history: &[(String, String)],
        teacher_mode: bool,
    ) -> Result<String> {
        if !self.enabled {
            return Err(AnubisError::Llm("Noxis Core LLM URL not configured".into()));
        }
        let body = self.chat_body(user_text, lang, history, teacher_mode, false);
        let mut req = self.client.post(chat_url(&self.cfg.base_url)).json(&body);
        if !self.cfg.api_key.trim().is_empty() {
            req = req.bearer_auth(self.cfg.api_key.trim());
        }
        let resp = req
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

    /// Streaming ask. Calls `on_delta` with each incremental text chunk as it
    /// arrives (SSE `data:` lines). Returns the full concatenated answer.
    ///
    /// Returns `Err` only on transport/protocol failure; an empty final answer
    /// after a successful stream yields an `Llm` error as with `ask`.
    pub async fn ask_stream(
        &self,
        user_text: &str,
        lang: &str,
        history: &[(String, String)],
        teacher_mode: bool,
        mut on_delta: impl FnMut(&str),
    ) -> Result<String> {
        if !self.enabled {
            return Err(AnubisError::Llm("Noxis Core LLM URL not configured".into()));
        }
        let body = self.chat_body(user_text, lang, history, teacher_mode, true);
        let mut req = self.client.post(chat_url(&self.cfg.base_url)).json(&body);
        if !self.cfg.api_key.trim().is_empty() {
            req = req.bearer_auth(self.cfg.api_key.trim());
        }
        let resp = req
            .send()
            .await
            .map_err(|e| AnubisError::Llm(format!("request: {e}")))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let b = resp.text().await.unwrap_or_default();
            return Err(AnubisError::Llm(format!("server {status}: {b}")));
        }

        let mut stream = resp.bytes_stream();
        let mut full = String::new();
        let mut buf = String::new();
        use futures::StreamExt;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| AnubisError::Llm(format!("stream: {e}")))?;
            buf.push_str(&String::from_utf8_lossy(&chunk));
            // SSE events are separated by blank lines; each `data:` line may
            // hold a full JSON or be split across chunks, so buffer per line.
            while let Some(pos) = buf.find('\n') {
                let line: String = buf.drain(..=pos).collect();
                let line = line.trim();
                if line.is_empty() || !line.starts_with("data:") {
                    continue;
                }
                let data = line.trim_start_matches("data:").trim();
                // "[DONE]" terminates the stream.
                if data == "[DONE]" {
                    buf.clear();
                    return if full.trim().is_empty() {
                        Err(AnubisError::Llm("empty response from brain".into()))
                    } else {
                        Ok(full)
                    };
                }
                let v: Value = match serde_json::from_str(data) {
                    Ok(v) => v,
                    Err(_) => continue, // incomplete/keepalive data line
                };
                if let Some(delta) = v
                    .get("choices")
                    .and_then(|c| c.get(0))
                    .and_then(|c| c.get("delta"))
                    .and_then(|d| d.get("content"))
                    .and_then(|c| c.as_str())
                {
                    if !delta.is_empty() {
                        full.push_str(delta);
                        on_delta(delta);
                    }
                }
            }
        }

        if full.trim().is_empty() {
            Err(AnubisError::Llm("empty response from brain".into()))
        } else {
            Ok(full)
        }
    }
}
