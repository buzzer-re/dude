use serde::Serialize;
use std::fs::OpenOptions;
use std::io::Write;

use crate::config::history_path;

#[derive(Serialize, serde::Deserialize)]
pub struct AuditEntry {
    pub timestamp: String,
    pub input: String,
    pub suggestion: Option<String>,
    pub accepted: bool,
}

pub fn log_interaction(input: &str, suggestion: Option<&str>, accepted: bool) {
    let path = history_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let entry = AuditEntry {
        timestamp: chrono::Utc::now().to_rfc3339(),
        input: input.to_string(),
        suggestion: suggestion.map(|s| s.to_string()),
        accepted,
    };

    let Ok(json) = serde_json::to_string(&entry) else {
        return;
    };

    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) else {
        return;
    };

    let _ = writeln!(file, "{json}");
}
