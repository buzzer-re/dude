use serde::Serialize;
use std::fs::OpenOptions;
use std::io::Write;

use crate::config::history_path;

#[derive(Clone, Serialize, serde::Deserialize)]
pub struct AuditEntry {
    pub timestamp: String,
    pub input: String,
    pub suggestion: Option<String>,
    pub accepted: bool,
}

pub fn log_interaction(input: &str, suggestion: Option<&str>, accepted: bool) {
    if let Err(e) = try_log_interaction(input, suggestion, accepted) {
        eprintln!("dude: warning: failed to log interaction: {e}");
    }
}

fn try_log_interaction(
    input: &str,
    suggestion: Option<&str>,
    accepted: bool,
) -> Result<(), String> {
    let path = history_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("create dir: {e}"))?;
    }

    let entry = AuditEntry {
        timestamp: chrono::Utc::now().to_rfc3339(),
        input: input.to_string(),
        suggestion: suggestion.map(|s| s.to_string()),
        accepted,
    };

    let json = serde_json::to_string(&entry).map_err(|e| format!("serialize: {e}"))?;

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("open: {e}"))?;

    writeln!(file, "{json}").map_err(|e| format!("write: {e}"))
}

/// Load recent audit entries for display.
pub fn recent_entries(count: usize) -> Vec<AuditEntry> {
    let path = history_path();
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let entries: Vec<AuditEntry> = content
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();

    let start = entries.len().saturating_sub(count);
    entries[start..].to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_entry_serialization() {
        let entry = AuditEntry {
            timestamp: "2026-01-01T00:00:00+00:00".into(),
            input: "gti status".into(),
            suggestion: Some("git status".into()),
            accepted: true,
        };
        let json = serde_json::to_string(&entry).unwrap();
        let parsed: AuditEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.input, "gti status");
        assert_eq!(parsed.suggestion.as_deref(), Some("git status"));
        assert!(parsed.accepted);
    }

    #[test]
    fn test_audit_entry_no_suggestion() {
        let entry = AuditEntry {
            timestamp: "2026-01-01T00:00:00+00:00".into(),
            input: "xyz".into(),
            suggestion: None,
            accepted: false,
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"suggestion\":null"));
        let parsed: AuditEntry = serde_json::from_str(&json).unwrap();
        assert!(parsed.suggestion.is_none());
        assert!(!parsed.accepted);
    }
}
