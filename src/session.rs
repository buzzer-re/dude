use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;

const SESSION_TTL_SECS: i64 = 900; // 15 minutes
const MAX_SESSION_ENTRIES: usize = 10;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionEntry {
    pub timestamp: String,
    pub role: String, // "user" or "assistant"
    pub content: String,
}

/// Load recent session entries, filtering out expired ones.
pub fn load_session() -> Vec<SessionEntry> {
    let path = crate::config::session_path();
    if !path.exists() {
        return Vec::new();
    }

    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let now = chrono::Utc::now();
    let cutoff = now - chrono::Duration::seconds(SESSION_TTL_SECS);

    let mut entries: Vec<SessionEntry> = content
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .filter(|entry: &SessionEntry| {
            chrono::DateTime::parse_from_rfc3339(&entry.timestamp)
                .map(|ts| ts > cutoff)
                .unwrap_or(false)
        })
        .collect();

    // Keep only the last N entries
    if entries.len() > MAX_SESSION_ENTRIES {
        entries = entries[entries.len() - MAX_SESSION_ENTRIES..].to_vec();
    }

    entries
}

/// Append a user+assistant exchange to the session file.
pub fn save_exchange(question: &str, response: &str) {
    let path = crate::config::session_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let now = chrono::Utc::now().to_rfc3339();

    let user_entry = SessionEntry {
        timestamp: now.clone(),
        role: "user".into(),
        content: question.to_string(),
    };
    let assistant_entry = SessionEntry {
        timestamp: now,
        role: "assistant".into(),
        content: response.to_string(),
    };

    // Load existing (prunes expired), add new, enforce cap, rewrite
    let mut entries = load_session();
    entries.push(user_entry);
    entries.push(assistant_entry);

    // Enforce the cap on write, not just on load
    if entries.len() > MAX_SESSION_ENTRIES {
        entries = entries[entries.len() - MAX_SESSION_ENTRIES..].to_vec();
    }

    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&path)
    {
        for e in &entries {
            if let Ok(j) = serde_json::to_string(e) {
                let _ = writeln!(file, "{j}");
            }
        }
    }
}

/// Clear the session.
pub fn clear_session() {
    let path = crate::config::session_path();
    let _ = fs::remove_file(path);
}

/// Format session history for inclusion in the LLM prompt.
pub fn session_context_string() -> String {
    let entries = load_session();
    if entries.is_empty() {
        return String::new();
    }

    let mut ctx = String::from("\nRecent conversation:\n");
    for entry in &entries {
        let prefix = match entry.role.as_str() {
            "user" => "User asked",
            _ => "You replied",
        };
        ctx.push_str(&format!("  {}: {}\n", prefix, entry.content));
    }
    ctx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_entry_serialization() {
        let entry = SessionEntry {
            timestamp: "2026-01-01T00:00:00+00:00".into(),
            role: "user".into(),
            content: "how do I list files".into(),
        };
        let json = serde_json::to_string(&entry).unwrap();
        let parsed: SessionEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.role, "user");
        assert_eq!(parsed.content, "how do I list files");
    }

    #[test]
    fn test_session_ttl_constant() {
        assert_eq!(SESSION_TTL_SECS, 900);
    }

    #[test]
    fn test_max_session_entries_constant() {
        assert_eq!(MAX_SESSION_ENTRIES, 10);
    }
}
