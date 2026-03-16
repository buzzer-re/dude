use serde::{Deserialize, Serialize};

use crate::config::Config;

#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    system: String,
    stream: bool,
    options: OllamaOptions,
}

#[derive(Serialize)]
struct OllamaOptions {
    temperature: f32,
    num_predict: i32,
}

#[derive(Deserialize)]
struct OllamaResponse {
    response: String,
    /// Reasoning models (qwen3, deepseek-r1) put chain-of-thought here
    #[serde(default)]
    thinking: Option<String>,
}

// ─── OpenAI-compatible request/response types ──────────────────────────

#[derive(Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    temperature: f32,
    max_tokens: i32,
    stream: bool,
}

#[derive(Serialize)]
struct OpenAIMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
}

#[derive(Deserialize)]
struct OpenAIChoice {
    message: OpenAIRespMessage,
}

#[derive(Deserialize)]
struct OpenAIRespMessage {
    content: String,
}

pub fn query(system_prompt: &str, user_prompt: &str, config: &Config) -> Result<String, String> {
    let base_url = config.effective_ollama_url();
    let model = config.effective_model();

    let token_budget = 2000;

    // Try Ollama format first (short timeout — if it's not Ollama, fail fast)
    let probe_client = crate::config::http_client(5)?;
    if let Ok(answer) = query_ollama(&probe_client, base_url, model, system_prompt, user_prompt, token_budget) {
        return Ok(answer);
    }

    // Fall back to OpenAI-compatible format (LM Studio, LocalAI, etc.)
    let client = crate::config::http_client(120)?;
    query_openai(&client, base_url, model, system_prompt, user_prompt, token_budget)
}

fn query_ollama(
    client: &reqwest::blocking::Client,
    base_url: &str,
    model: &str,
    system_prompt: &str,
    user_prompt: &str,
    token_budget: i32,
) -> Result<String, String> {
    let request = OllamaRequest {
        model: model.to_string(),
        prompt: user_prompt.to_string(),
        system: system_prompt.to_string(),
        stream: false,
        options: OllamaOptions {
            temperature: 0.1,
            num_predict: token_budget,
        },
    };

    let url = format!("{}/api/generate", base_url);

    let resp = client.post(&url).json(&request).send().map_err(|e| {
        if e.is_connect() {
            "dude: can't reach server. is it running?".to_string()
        } else if e.is_timeout() {
            "dude: server took too long to respond".to_string()
        } else {
            format!("dude: server error: {e}")
        }
    })?;

    if !resp.status().is_success() {
        return Err("ollama format failed".into());
    }

    let parsed: OllamaResponse = resp.json().map_err(|_| "ollama parse failed".to_string())?;

    let answer = if parsed.response.trim().is_empty() {
        if let Some(thinking) = &parsed.thinking {
            extract_answer_from_thinking(thinking)
        } else {
            String::new()
        }
    } else {
        parsed.response.trim().to_string()
    };

    if answer.is_empty() {
        Err("empty response".into())
    } else {
        Ok(answer)
    }
}

fn query_openai(
    client: &reqwest::blocking::Client,
    base_url: &str,
    model: &str,
    system_prompt: &str,
    user_prompt: &str,
    token_budget: i32,
) -> Result<String, String> {
    let request = OpenAIRequest {
        model: model.to_string(),
        messages: vec![
            OpenAIMessage {
                role: "system".into(),
                content: system_prompt.to_string(),
            },
            OpenAIMessage {
                role: "user".into(),
                content: user_prompt.to_string(),
            },
        ],
        temperature: 0.1,
        max_tokens: token_budget,
        stream: false,
    };

    let url = format!("{}/v1/chat/completions", base_url);

    let resp = client.post(&url).json(&request).send().map_err(|e| {
        if e.is_connect() {
            "dude: can't reach server. is it running?".to_string()
        } else if e.is_timeout() {
            "dude: server took too long to respond".to_string()
        } else {
            format!("dude: server error: {e}")
        }
    })?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        return Err(format!("dude: server returned {status}: {body}"));
    }

    let parsed: OpenAIResponse = resp
        .json()
        .map_err(|e| format!("dude: bad response from server: {e}"))?;

    let text = parsed
        .choices
        .first()
        .map(|c| c.message.content.trim().to_string())
        .unwrap_or_default();

    // If response is only <think> content (reasoning model ran out of tokens),
    // try to extract from the thinking text
    if text.starts_with("<think>") && !text.contains("</think>") {
        let thinking = text.trim_start_matches("<think>").trim();
        let extracted = extract_answer_from_thinking(thinking);
        if !extracted.is_empty() {
            return Ok(extracted);
        }
    }

    Ok(text)
}

/// When a reasoning model burns all tokens on thinking and produces no response,
/// try to extract the best candidate answer from its chain-of-thought.
fn extract_answer_from_thinking(thinking: &str) -> String {
    let mut best_candidate = String::new();

    // Scan from the end — the last backtick-wrapped command is usually the final answer
    for line in thinking.lines().rev() {
        let trimmed = line.trim();

        if let Some(start) = trimmed.find('`') {
            if let Some(end) = trimmed[start + 1..].find('`') {
                let cmd = &trimmed[start + 1..start + 1 + end];
                if !cmd.is_empty() && !cmd.contains("No") {
                    best_candidate = cmd.to_string();
                }
            }
        }

        // If we found a candidate near a "final"/"output"/"answer" line, use it
        let lower = trimmed.to_lowercase();
        if (lower.contains("final") || lower.contains("output") || lower.contains("answer"))
            && !best_candidate.is_empty()
        {
            return best_candidate;
        }
    }

    best_candidate
}

/// Fetch installed model names. Tries Ollama API first, then OpenAI-compatible.
pub fn list_models_from_url(base_url: &str) -> Vec<String> {
    let Ok(client) = crate::config::http_client(3) else {
        return vec![];
    };

    // Try Ollama format: /api/tags
    if let Some(models) = try_ollama_models(&client, base_url) {
        if !models.is_empty() {
            return models;
        }
    }

    // Try OpenAI-compatible format: /v1/models (LM Studio, LocalAI, etc.)
    if let Some(models) = try_openai_models(&client, base_url) {
        if !models.is_empty() {
            return models;
        }
    }

    vec![]
}

fn try_ollama_models(client: &reqwest::blocking::Client, base_url: &str) -> Option<Vec<String>> {
    #[derive(Deserialize)]
    struct TagsResponse {
        models: Vec<ModelEntry>,
    }
    #[derive(Deserialize)]
    struct ModelEntry {
        name: String,
    }

    let url = format!("{}/api/tags", base_url);
    let resp = client.get(&url).send().ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let tags: TagsResponse = resp.json().ok()?;
    Some(tags.models.into_iter().map(|m| m.name).collect())
}

fn try_openai_models(client: &reqwest::blocking::Client, base_url: &str) -> Option<Vec<String>> {
    #[derive(Deserialize)]
    struct ModelsResponse {
        data: Vec<ModelEntry>,
    }
    #[derive(Deserialize)]
    struct ModelEntry {
        id: String,
    }

    let url = format!("{}/v1/models", base_url);
    let resp = client.get(&url).send().ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let models: ModelsResponse = resp.json().ok()?;
    Some(models.data.into_iter().map(|m| m.id).collect())
}

pub fn list_models(config: &Config) -> Vec<String> {
    list_models_from_url(config.effective_ollama_url())
}

pub fn check_available(config: &Config) -> bool {
    let Ok(client) = crate::config::http_client(2) else {
        return false;
    };
    client
        .get(config.effective_ollama_url())
        .send()
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}
