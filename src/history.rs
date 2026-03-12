use std::fs;
use std::path::PathBuf;

pub fn read_shell_history(max_lines: usize) -> Vec<String> {
    let history_file = find_history_file();
    let Some(path) = history_file else {
        return Vec::new();
    };

    // zsh history files often contain non-UTF8 bytes, so read as bytes
    let raw = match fs::read(&path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let content = String::from_utf8_lossy(&raw);

    let lines: Vec<String> = content
        .lines()
        .filter_map(|line| {
            // zsh extended history format: ": timestamp:0;command"
            if line.starts_with(": ") {
                line.split_once(';').map(|(_, s)| s.to_string())
            } else if line.starts_with('#') || line.trim().is_empty() {
                None
            } else {
                Some(line.to_string())
            }
        })
        .collect();

    // Return the last N entries
    let start = lines.len().saturating_sub(max_lines);
    lines[start..].to_vec()
}

pub fn recent_history(count: usize) -> Vec<String> {
    read_shell_history(count)
}

fn find_history_file() -> Option<PathBuf> {
    // Check HISTFILE env first
    if let Ok(histfile) = std::env::var("HISTFILE") {
        let path = PathBuf::from(&histfile);
        if path.exists() {
            return Some(path);
        }
    }

    let home = dirs::home_dir()?;

    // Try common history file locations
    let candidates = [
        home.join(".zsh_history"),
        home.join(".zhistory"),
        home.join(".bash_history"),
        home.join(".history"),
    ];

    candidates.into_iter().find(|p| p.exists())
}
