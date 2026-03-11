use serde::{Deserialize, Serialize};

use crate::config::Config;

#[derive(Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: u32,
    system: String,
    messages: Vec<ClaudeMessage>,
}

#[derive(Serialize)]
struct ClaudeMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ClaudeResponse {
    content: Vec<ContentBlock>,
}

#[derive(Deserialize)]
struct ContentBlock {
    text: Option<String>,
}

pub fn query(system_prompt: &str, user_prompt: &str, config: &Config) -> Result<String, String> {
    let api_key = config
        .claude_api_key
        .as_deref()
        .filter(|k| !k.is_empty())
        .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok().as_deref().map(|_| ""))
        .ok_or_else(|| {
            "dude: no claude API key. set claude_api_key in config or ANTHROPIC_API_KEY env var"
                .to_string()
        })?;

    // Re-read from env if config was empty
    let api_key = if api_key.is_empty() {
        std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| "dude: ANTHROPIC_API_KEY not set".to_string())?
    } else {
        api_key.to_string()
    };

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))?;

    let model = config
        .claude_model
        .as_deref()
        .unwrap_or("claude-haiku-4-5-20251001");

    let request = ClaudeRequest {
        model: model.to_string(),
        max_tokens: 200,
        system: system_prompt.to_string(),
        messages: vec![ClaudeMessage {
            role: "user".into(),
            content: user_prompt.to_string(),
        }],
    };

    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&request)
        .send()
        .map_err(|e| {
            if e.is_connect() {
                "dude: can't reach Claude API".to_string()
            } else if e.is_timeout() {
                "dude: Claude API took too long".to_string()
            } else {
                format!("dude: Claude API error: {e}")
            }
        })?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        return Err(format!("dude: Claude API returned {status}: {body}"));
    }

    let parsed: ClaudeResponse = resp
        .json()
        .map_err(|e| format!("dude: bad response from Claude: {e}"))?;

    let text = parsed
        .content
        .into_iter()
        .filter_map(|b| b.text)
        .collect::<Vec<_>>()
        .join("");

    Ok(text.trim().to_string())
}

pub fn check_available(config: &Config) -> bool {
    config
        .claude_api_key
        .as_deref()
        .map(|k| !k.is_empty())
        .unwrap_or(false)
        || std::env::var("ANTHROPIC_API_KEY").is_ok()
}
