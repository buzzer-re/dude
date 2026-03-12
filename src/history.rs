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

    let lines: Vec<String> = parse_history_lines(&content);

    // Return the last N entries
    let start = lines.len().saturating_sub(max_lines);
    lines[start..].to_vec()
}

/// Parse shell history content into command lines.
/// Handles zsh extended format (`: timestamp:0;command`), bash, and plain formats.
fn parse_history_lines(content: &str) -> Vec<String> {
    content
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
        .collect()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_zsh_extended_format() {
        let content = ": 1700000000:0;git status\n: 1700000001:0;cargo test\n";
        let lines = parse_history_lines(content);
        assert_eq!(lines, vec!["git status", "cargo test"]);
    }

    #[test]
    fn test_parse_bash_format() {
        let content = "ls -la\ncd /tmp\npwd\n";
        let lines = parse_history_lines(content);
        assert_eq!(lines, vec!["ls -la", "cd /tmp", "pwd"]);
    }

    #[test]
    fn test_parse_skips_comments_and_empty() {
        let content = "# comment\n\nls\n  \ngit status\n";
        let lines = parse_history_lines(content);
        assert_eq!(lines, vec!["ls", "git status"]);
    }

    #[test]
    fn test_parse_mixed_format() {
        let content = ": 1700000000:0;git pull\nls -la\n# comment\n";
        let lines = parse_history_lines(content);
        assert_eq!(lines, vec!["git pull", "ls -la"]);
    }

    #[test]
    fn test_parse_empty_content() {
        let lines = parse_history_lines("");
        assert!(lines.is_empty());
    }
}
