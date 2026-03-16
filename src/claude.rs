use serde::{Deserialize, Serialize};
use std::process::Command;

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

#[derive(Deserialize)]
struct KeychainCredentials {
    #[serde(rename = "claudeAiOauth")]
    claude_ai_oauth: Option<OAuthTokens>,
}

#[derive(Deserialize)]
struct OAuthTokens {
    #[serde(rename = "accessToken")]
    access_token: String,
}

/// Auth method resolved at query time.
enum AuthMethod {
    /// OAuth token from macOS Keychain (Claude Code credentials)
    OAuth(String),
    /// Direct API key from config or env
    ApiKey(String),
}

/// Resolve the best auth method available.
fn resolve_auth(config: &Config) -> Result<AuthMethod, String> {
    // 1. ANTHROPIC_API_KEY env var (explicit override, always wins)
    if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
        if !key.is_empty() {
            return Ok(AuthMethod::ApiKey(key));
        }
    }

    // 2. Saved token file (from `dude setup-token`) — most recent explicit setup
    if let Some(token) = read_token_file() {
        if token.starts_with("sk-ant-") && !token.starts_with("sk-ant-oat") {
            return Ok(AuthMethod::ApiKey(token));
        } else {
            return Ok(AuthMethod::OAuth(token));
        }
    }

    // 3. macOS Keychain (Claude Code OAuth credentials)
    if let Some(token) = read_keychain_oauth() {
        return Ok(AuthMethod::OAuth(token));
    }

    // 4. Config file API key (fallback)
    if let Some(key) = config.claude_api_key.as_deref().filter(|k| !k.is_empty()) {
        return Ok(AuthMethod::ApiKey(key.to_string()));
    }

    Err("dude: no claude credentials found.\n  • API key: dude setup-token sk-ant-...\n  • OAuth:   dude setup-token <token> (get it from Claude Code with /oauth-token)".to_string())
}

/// Read token from the saved token file (set by `dude setup-token`).
fn read_token_file() -> Option<String> {
    let path = crate::config::token_path();
    let token = std::fs::read_to_string(path).ok()?.trim().to_string();
    if token.is_empty() {
        None
    } else {
        Some(token)
    }
}

/// Read OAuth access token from macOS Keychain (Claude Code credentials).
fn read_keychain_oauth() -> Option<String> {
    let output = Command::new("security")
        .args([
            "find-generic-password",
            "-s",
            "Claude Code-credentials",
            "-w",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let json_str = String::from_utf8(output.stdout).ok()?;
    let creds: KeychainCredentials = serde_json::from_str(json_str.trim()).ok()?;
    let token = creds.claude_ai_oauth?.access_token;

    if token.is_empty() {
        None
    } else {
        Some(token)
    }
}

pub fn query(system_prompt: &str, user_prompt: &str, config: &Config) -> Result<String, String> {
    let auth = resolve_auth(config)?;

    let client = crate::config::http_client(120)?;

    let model = config.effective_claude_model();

    let request = ClaudeRequest {
        model: model.to_string(),
        max_tokens: 300,
        system: system_prompt.to_string(),
        messages: vec![ClaudeMessage {
            role: "user".into(),
            content: user_prompt.to_string(),
        }],
    };

    let mut req_builder = client
        .post("https://api.anthropic.com/v1/messages")
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json");

    req_builder = match &auth {
        AuthMethod::OAuth(token) => req_builder
            .header("Authorization", format!("Bearer {}", token))
            .header("anthropic-beta", "oauth-2025-04-20"),
        AuthMethod::ApiKey(key) => req_builder.header("x-api-key", key),
    };

    let resp = req_builder.json(&request).send().map_err(|e| {
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
    resolve_auth(config).is_ok()
}
