use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::path::PathBuf;

pub const DEFAULT_CLAUDE_MODEL: &str = "claude-haiku-4-5-20251001";

/// LLM provider backend.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    #[default]
    Ollama,
    Claude,
}

impl fmt::Display for Provider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ollama => write!(f, "ollama"),
            Self::Claude => write!(f, "claude"),
        }
    }
}

impl Provider {
    pub fn from_str_lenient(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "ollama" => Some(Self::Ollama),
            "claude" => Some(Self::Claude),
            _ => None,
        }
    }
}

/// Safety mode controlling command confirmation behavior.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SafetyMode {
    /// Always ask before running
    #[default]
    Confirm,
    /// Safe commands auto-run, destructive ones blocked
    Auto,
    /// Never ask — live dangerously
    Yolo,
}

impl fmt::Display for SafetyMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Confirm => write!(f, "confirm"),
            Self::Auto => write!(f, "auto"),
            Self::Yolo => write!(f, "yolo"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub ollama_url: String,
    #[serde(default)]
    pub safety_mode: SafetyMode,
    #[serde(default = "default_history_context")]
    pub history_context: usize,
    #[serde(default)]
    pub provider: Provider,
    #[serde(default)]
    pub claude_api_key: Option<String>,
    #[serde(default)]
    pub claude_model: Option<String>,
}

fn default_history_context() -> usize {
    20
}

impl Default for Config {
    fn default() -> Self {
        Self {
            model: String::new(),
            ollama_url: String::new(),
            safety_mode: SafetyMode::default(),
            history_context: 20,
            provider: Provider::default(),
            claude_api_key: None,
            claude_model: None,
        }
    }
}

impl Config {
    /// Check if this config has been set up (not a blank default).
    /// A config needs setup if the selected provider has no usable configuration.
    pub fn needs_setup(&self) -> bool {
        match self.provider {
            Provider::Claude => {
                // Claude is set up if there's an API key, env var, or saved token file
                let has_key = self.claude_api_key.as_deref().is_some_and(|k| !k.is_empty());
                let has_env = std::env::var("ANTHROPIC_API_KEY")
                    .map(|k| !k.is_empty())
                    .unwrap_or(false);
                let has_token_file = token_path().exists();
                !has_key && !has_env && !has_token_file
            }
            Provider::Ollama => {
                // Ollama works out of the box with defaults — only "needs setup"
                // if nothing has been configured at all (fresh default)
                self.model.is_empty() && self.ollama_url.is_empty()
            }
        }
    }

    /// Return the effective provider.
    pub fn effective_provider(&self) -> &Provider {
        &self.provider
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

    /// Return the effective Claude model.
    pub fn effective_claude_model(&self) -> &str {
        self.claude_model
            .as_deref()
            .filter(|m| !m.is_empty())
            .unwrap_or(DEFAULT_CLAUDE_MODEL)
    }

    /// Return the effective safety mode.
    pub fn effective_safety_mode(&self) -> &SafetyMode {
        &self.safety_mode
    }

    /// Return the active model name for the current provider.
    pub fn active_model(&self) -> &str {
        match self.provider {
            Provider::Claude => self.effective_claude_model(),
            Provider::Ollama => self.effective_model(),
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
            Config::default()
        }
    }

    pub fn save(&self) {
        save_toml(&config_path(), self);
    }

    pub fn use_claude(&self) -> bool {
        self.provider == Provider::Claude
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

pub fn session_path() -> PathBuf {
    dude_dir().join("session.jsonl")
}

pub fn token_path() -> PathBuf {
    dude_dir().join(".claude_token")
}

pub fn last_cmd_path() -> PathBuf {
    let user = std::env::var("USER").unwrap_or_else(|_| "unknown".into());
    std::env::temp_dir().join(format!("dude_last_cmd.{}", user))
}

/// Build an HTTP client with a standard timeout.
pub fn http_client(timeout_secs: u64) -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))
}

/// Save a serializable value as TOML to the given path, creating parent dirs.
pub fn save_toml(path: &std::path::Path, value: &impl Serialize) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(content) = toml::to_string_pretty(value) {
        let _ = fs::write(path, content);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_needs_setup() {
        let config = Config::default();
        assert!(config.needs_setup());
    }

    #[test]
    fn test_effective_defaults() {
        let config = Config::default();
        assert_eq!(*config.effective_provider(), Provider::Ollama);
        assert_eq!(config.effective_model(), "qwen2.5-coder:1.5b");
        assert_eq!(config.effective_ollama_url(), "http://localhost:11434");
        assert_eq!(config.effective_claude_model(), DEFAULT_CLAUDE_MODEL);
        assert_eq!(*config.effective_safety_mode(), SafetyMode::Confirm);
    }

    #[test]
    fn test_effective_with_values() {
        let config = Config {
            model: "llama3:8b".into(),
            provider: Provider::Claude,
            claude_model: Some("claude-sonnet-4-6".into()),
            safety_mode: SafetyMode::Yolo,
            ..Config::default()
        };
        assert_eq!(config.effective_model(), "llama3:8b");
        assert_eq!(*config.effective_provider(), Provider::Claude);
        assert_eq!(config.effective_claude_model(), "claude-sonnet-4-6");
        assert_eq!(*config.effective_safety_mode(), SafetyMode::Yolo);
        assert!(config.use_claude());
    }

    #[test]
    fn test_provider_from_str_lenient() {
        assert_eq!(Provider::from_str_lenient("claude"), Some(Provider::Claude));
        assert_eq!(Provider::from_str_lenient("Claude"), Some(Provider::Claude));
        assert_eq!(Provider::from_str_lenient("OLLAMA"), Some(Provider::Ollama));
        assert_eq!(Provider::from_str_lenient("unknown"), None);
    }

    #[test]
    fn test_provider_display() {
        assert_eq!(Provider::Ollama.to_string(), "ollama");
        assert_eq!(Provider::Claude.to_string(), "claude");
    }

    #[test]
    fn test_safety_mode_display() {
        assert_eq!(SafetyMode::Confirm.to_string(), "confirm");
        assert_eq!(SafetyMode::Auto.to_string(), "auto");
        assert_eq!(SafetyMode::Yolo.to_string(), "yolo");
    }

    #[test]
    fn test_config_roundtrip_toml() {
        let config = Config {
            model: "test-model".into(),
            provider: Provider::Ollama,
            ..Config::default()
        };
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.model, "test-model");
        assert_eq!(parsed.provider, Provider::Ollama);
    }

    #[test]
    fn test_config_deserializes_string_provider() {
        // Config files use lowercase strings
        let toml_str = r#"
            provider = "claude"
            safety_mode = "yolo"
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.provider, Provider::Claude);
        assert_eq!(config.safety_mode, SafetyMode::Yolo);
    }

    #[test]
    fn test_setup_not_needed_with_api_key() {
        let config = Config {
            provider: Provider::Claude,
            claude_api_key: Some("sk-ant-test".into()),
            ..Config::default()
        };
        assert!(!config.needs_setup());
    }

    #[test]
    fn test_ollama_needs_setup_when_blank() {
        let config = Config::default();
        assert!(config.needs_setup());
    }

    #[test]
    fn test_ollama_not_needs_setup_with_model() {
        let config = Config {
            model: "qwen2.5-coder:7b".into(),
            ..Config::default()
        };
        assert!(!config.needs_setup());
    }

    #[test]
    fn test_active_model_ollama() {
        let config = Config {
            model: "llama3:8b".into(),
            ..Config::default()
        };
        assert_eq!(config.active_model(), "llama3:8b");
    }

    #[test]
    fn test_active_model_claude() {
        let config = Config {
            provider: Provider::Claude,
            claude_model: Some("claude-sonnet-4-6".into()),
            ..Config::default()
        };
        assert_eq!(config.active_model(), "claude-sonnet-4-6");
    }
}
