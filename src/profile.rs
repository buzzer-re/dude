use crate::config::{profile_path, save_toml};
use crate::history;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Profile {
    pub user: UserInfo,
    pub patterns: Patterns,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct UserInfo {
    pub name: String,
    pub shell: String,
    pub os: String,
    pub common_tools: Vec<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Patterns {
    pub typical_directories: Vec<String>,
    pub top_commands: Vec<String>,
}

impl Profile {
    pub fn load() -> Self {
        let path = profile_path();
        if path.exists() {
            let content = fs::read_to_string(&path).unwrap_or_default();
            toml::from_str(&content).unwrap_or_default()
        } else {
            Profile::default()
        }
    }

    pub fn save(&self) {
        save_toml(&profile_path(), self);
    }

    pub fn analyze_and_build() -> Self {
        let name = std::env::var("USER").unwrap_or_else(|_| "unknown".into());
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "zsh".into());
        let os = std::env::consts::OS.to_string();

        let history_entries = history::read_shell_history(500);
        let (top_commands, common_dirs, tools) = analyze_history(&history_entries);

        let profile = Profile {
            user: UserInfo {
                name,
                shell,
                os,
                common_tools: tools,
            },
            patterns: Patterns {
                typical_directories: common_dirs,
                top_commands,
            },
        };

        profile.save();
        profile
    }

    pub fn as_context_string(&self) -> String {
        let mut ctx = format!(
            "User: {}, OS: {}, Shell: {}\n",
            self.user.name, self.user.os, self.user.shell
        );
        if !self.user.common_tools.is_empty() {
            ctx.push_str(&format!(
                "Common tools: {}\n",
                self.user.common_tools.join(", ")
            ));
        }
        if !self.patterns.top_commands.is_empty() {
            ctx.push_str(&format!(
                "Frequently used: {}\n",
                self.patterns.top_commands.join(", ")
            ));
        }
        ctx
    }
}

fn analyze_history(entries: &[String]) -> (Vec<String>, Vec<String>, Vec<String>) {
    let mut cmd_counts: HashMap<String, usize> = HashMap::new();
    let mut dir_counts: HashMap<String, usize> = HashMap::new();

    let known_tools = [
        "git",
        "docker",
        "cargo",
        "npm",
        "yarn",
        "pip",
        "brew",
        "kubectl",
        "terraform",
        "python",
        "node",
        "ruby",
        "go",
        "rustc",
        "gcc",
        "make",
        "cmake",
        "curl",
        "wget",
        "ssh",
        "scp",
        "rsync",
        "vim",
        "nvim",
        "code",
        "tmux",
        "screen",
    ];

    for entry in entries {
        let cmd = entry.split_whitespace().next().unwrap_or("").to_string();
        if !cmd.is_empty() {
            *cmd_counts.entry(cmd).or_default() += 1;
        }

        // Extract directories from cd commands
        if let Some(dir) = entry.strip_prefix("cd ") {
            let dir = dir.trim().to_string();
            if !dir.is_empty() && dir != "-" {
                *dir_counts.entry(dir).or_default() += 1;
            }
        }
    }

    let mut top_commands: Vec<_> = cmd_counts.into_iter().collect();
    top_commands.sort_by(|a, b| b.1.cmp(&a.1));
    let top_commands: Vec<String> = top_commands
        .iter()
        .take(15)
        .map(|(k, _)| k.clone())
        .collect();

    let mut top_dirs: Vec<_> = dir_counts.into_iter().collect();
    top_dirs.sort_by(|a, b| b.1.cmp(&a.1));
    let common_dirs: Vec<String> = top_dirs.iter().take(10).map(|(k, _)| k.clone()).collect();

    let tools: Vec<String> = known_tools
        .iter()
        .filter(|t| top_commands.contains(&t.to_string()))
        .map(|t| t.to_string())
        .collect();

    (top_commands, common_dirs, tools)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_history_extracts_commands() {
        let entries = vec![
            "git status".into(),
            "git commit -m test".into(),
            "cargo test".into(),
            "ls -la".into(),
            "git push".into(),
        ];
        let (top_cmds, _, tools) = analyze_history(&entries);
        assert!(top_cmds.contains(&"git".to_string()));
        assert!(top_cmds.contains(&"cargo".to_string()));
        assert!(tools.contains(&"git".to_string()));
        assert!(tools.contains(&"cargo".to_string()));
    }

    #[test]
    fn test_analyze_history_extracts_dirs() {
        let entries = vec![
            "cd /tmp".into(),
            "cd ~/projects".into(),
            "cd /tmp".into(),
        ];
        let (_, dirs, _) = analyze_history(&entries);
        assert!(dirs.contains(&"/tmp".to_string()));
        assert!(dirs.contains(&"~/projects".to_string()));
    }

    #[test]
    fn test_analyze_history_empty() {
        let (cmds, dirs, tools) = analyze_history(&[]);
        assert!(cmds.is_empty());
        assert!(dirs.is_empty());
        assert!(tools.is_empty());
    }

    #[test]
    fn test_as_context_string() {
        let profile = Profile {
            user: UserInfo {
                name: "tester".into(),
                shell: "zsh".into(),
                os: "macos".into(),
                common_tools: vec!["git".into()],
            },
            patterns: Patterns {
                typical_directories: vec![],
                top_commands: vec!["git".into(), "ls".into()],
            },
        };
        let ctx = profile.as_context_string();
        assert!(ctx.contains("tester"));
        assert!(ctx.contains("macos"));
        assert!(ctx.contains("git"));
    }

    #[test]
    fn test_profile_default() {
        let profile = Profile::default();
        assert!(profile.user.name.is_empty());
        assert!(profile.patterns.top_commands.is_empty());
    }
}

pub fn display_profile(profile: &Profile) {
    use colored::Colorize;

    println!("{}", "dude knows:".yellow().bold());
    println!();
    println!("  {} {}", "User:".dimmed(), profile.user.name);
    println!("  {} {}", "OS:".dimmed(), profile.user.os);
    println!("  {} {}", "Shell:".dimmed(), profile.user.shell);

    if !profile.user.common_tools.is_empty() {
        println!(
            "  {} {}",
            "Tools:".dimmed(),
            profile.user.common_tools.join(", ")
        );
    }

    if !profile.patterns.top_commands.is_empty() {
        println!(
            "  {} {}",
            "Top cmds:".dimmed(),
            profile.patterns.top_commands.join(", ")
        );
    }

    if !profile.patterns.typical_directories.is_empty() {
        println!(
            "  {} {}",
            "Dirs:".dimmed(),
            profile.patterns.typical_directories.join(", ")
        );
    }

    if let Ok(corrections) = crate::corrections::Corrections::open() {
        let count = corrections.count();
        if count > 0 {
            println!("  {} {}", "Learned corrections:".dimmed(), count);
        }
    }
}
