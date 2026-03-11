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

pub fn query(system_prompt: &str, user_prompt: &str, config: &Config) -> Result<String, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))?;

    // Reasoning models burn tokens on <think> tags. Give them more budget.
    let is_reasoning_model = config.model.starts_with("qwen3")
        || config.model.contains("deepseek-r1");
    let token_budget = if is_reasoning_model { 1000 } else { 300 };

    let request = OllamaRequest {
        model: config.model.clone(),
        prompt: user_prompt.to_string(),
        system: system_prompt.to_string(),
        stream: false,
        options: OllamaOptions {
            temperature: 0.1,
            num_predict: token_budget,
        },
    };

    let url = format!("{}/api/generate", config.ollama_url);

    let resp = client
        .post(&url)
        .json(&request)
        .send()
        .map_err(|e| {
            if e.is_connect() {
                "dude: can't reach ollama. is it running? try: ollama serve".to_string()
            } else if e.is_timeout() {
                "dude: ollama took too long to respond".to_string()
            } else {
                format!("dude: ollama error: {e}")
            }
        })?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        return Err(format!("dude: ollama returned {status}: {body}"));
    }

    let parsed: OllamaResponse = resp
        .json()
        .map_err(|e| format!("dude: bad response from ollama: {e}"))?;

    // Reasoning models may put the answer in response, or burn all tokens on thinking.
    // If response is empty but thinking exists, try to extract the answer from thinking.
    let answer = if parsed.response.trim().is_empty() {
        if let Some(thinking) = &parsed.thinking {
            extract_answer_from_thinking(thinking)
        } else {
            String::new()
        }
    } else {
        parsed.response.trim().to_string()
    };

    Ok(answer)
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

pub fn check_available(config: &Config) -> bool {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build();

    let Ok(client) = client else { return false };

    client
        .get(&config.ollama_url)
        .send()
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}
