use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_ollama_url")]
    pub ollama_url: String,
    #[serde(default = "default_safety_mode")]
    pub safety_mode: String,
    #[serde(default = "default_history_context")]
    pub history_context: usize,
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub claude_api_key: Option<String>,
    #[serde(default)]
    pub claude_model: Option<String>,
}

fn default_model() -> String {
    "qwen2.5-coder:1.5b".into()
}

fn default_ollama_url() -> String {
    "http://localhost:11434".into()
}

fn default_safety_mode() -> String {
    "confirm".into()
}

fn default_history_context() -> usize {
    20
}

impl Default for Config {
    fn default() -> Self {
        Self {
            model: default_model(),
            ollama_url: default_ollama_url(),
            safety_mode: default_safety_mode(),
            history_context: default_history_context(),
            provider: "ollama".into(),
            claude_api_key: None,
            claude_model: None,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let path = config_path();
        if path.exists() {
            let content = fs::read_to_string(&path).unwrap_or_default();
            toml::from_str(&content).unwrap_or_default()
        } else {
            let config = Config::default();
            config.save();
            config
        }
    }

    pub fn save(&self) {
        let path = config_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(content) = toml::to_string_pretty(self) {
            let _ = fs::write(&path, content);
        }
    }

    pub fn use_claude(&self) -> bool {
        self.provider.eq_ignore_ascii_case("claude")
    }
}

pub fn dude_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("dude")
}

pub fn config_path() -> PathBuf {
    dude_dir().join("config.toml")
}

pub fn db_path() -> PathBuf {
    dude_dir().join("corrections.db")
}

pub fn profile_path() -> PathBuf {
    dude_dir().join("profile.toml")
}

pub fn history_path() -> PathBuf {
    dude_dir().join("history.jsonl")
}

pub fn last_cmd_path() -> PathBuf {
    let user = std::env::var("USER").unwrap_or_else(|_| "unknown".into());
    std::env::temp_dir().join(format!("dude_last_cmd.{}", user))
}
