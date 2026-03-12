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
    String::new()
}

fn default_ollama_url() -> String {
    String::new()
}

fn default_safety_mode() -> String {
    String::new()
}

fn default_history_context() -> usize {
    20
}

impl Default for Config {
    fn default() -> Self {
        Self {
            model: String::new(),
            ollama_url: String::new(),
            safety_mode: String::new(),
            history_context: 20,
            provider: String::new(),
            claude_api_key: None,
            claude_model: None,
        }
    }
}

impl Config {
    /// Check if this config has been set up (not a blank default).
    pub fn needs_setup(&self) -> bool {
        self.provider.is_empty()
    }

    /// Return the effective provider, falling back to "ollama".
    pub fn effective_provider(&self) -> &str {
        if self.provider.is_empty() {
            "ollama"
        } else {
            &self.provider
        }
    }

    /// Return the effective ollama model.
    pub fn effective_model(&self) -> &str {
        if self.model.is_empty() {
            "qwen2.5-coder:1.5b"
        } else {
            &self.model
        }
    }

    /// Return the effective ollama URL.
    pub fn effective_ollama_url(&self) -> &str {
        if self.ollama_url.is_empty() {
            "http://localhost:11434"
        } else {
            &self.ollama_url
        }
    }

    /// Return the effective safety mode.
    pub fn effective_safety_mode(&self) -> &str {
        if self.safety_mode.is_empty() {
            "confirm"
        } else {
            &self.safety_mode
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
