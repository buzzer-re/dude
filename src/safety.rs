/// Check if a command is destructive and should always require confirmation.
pub fn is_destructive(command: &str) -> bool {
    let patterns = [
        "rm -rf /",
        "rm -rf ~",
        "rm -rf /*",
        "dd if=/dev/zero",
        "dd if=/dev/random",
        "mkfs.",
        "> /dev/sda",
        "> /dev/disk",
        "chmod -R 777 /",
        "chmod 777 /",
        ":(){", // fork bomb
        "mv /* ",
        "mv / ",
    ];

    let lower = command.to_lowercase();
    patterns.iter().any(|p| lower.contains(p))
}

/// Check if a command needs confirmation in the given safety mode.
/// Returns: true = needs confirmation, false = safe to auto-run
pub fn needs_confirmation(command: &str, safety_mode: &crate::config::SafetyMode) -> bool {
    use crate::config::SafetyMode;
    match safety_mode {
        SafetyMode::Yolo => false,
        SafetyMode::Auto => {
            if is_destructive(command) {
                return true;
            }
            !is_safe_command(command)
        }
        SafetyMode::Confirm => true,
    }
}

/// Check if a command is "safe" enough to run in auto mode without confirmation.
pub fn is_safe_command(command: &str) -> bool {
    let first_word = command.split_whitespace().next().unwrap_or("");

    // Single-word commands that are always safe regardless of arguments
    let safe_any_args = [
        "ls", "pwd", "echo", "cat", "head", "tail", "wc", "which", "where",
        "whoami", "date", "cal", "uptime", "df", "du", "free", "uname",
    ];
    if safe_any_args.contains(&first_word) {
        return true;
    }

    // Multi-word prefixes that are safe (checked as prefix match)
    let safe_prefixes = [
        "git status", "git log", "git diff", "git branch",
        "cargo check", "cargo test", "cargo build",
        "npm test", "npm run",
        "python --version", "node --version", "rustc --version",
    ];
    safe_prefixes.iter().any(|s| command.starts_with(s))
}

/// Return the safety mode description for display.
pub fn describe_mode(mode: &crate::config::SafetyMode) -> &'static str {
    use crate::config::SafetyMode;
    match mode {
        SafetyMode::Auto => "auto (safe commands run without confirmation, destructive commands blocked)",
        SafetyMode::Yolo => "yolo (no confirmations — live dangerously)",
        SafetyMode::Confirm => "confirm (always ask before running)",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_destructive_commands() {
        assert!(is_destructive("rm -rf /"));
        assert!(is_destructive("rm -rf ~"));
        assert!(is_destructive("dd if=/dev/zero of=/dev/sda"));
        assert!(is_destructive("sudo rm -rf /"));
        assert!(!is_destructive("rm file.txt"));
        assert!(!is_destructive("ls -la"));
    }

    #[test]
    fn test_safe_commands() {
        assert!(is_safe_command("ls -la"));
        assert!(is_safe_command("pwd"));
        assert!(is_safe_command("git status"));
        assert!(is_safe_command("cargo test --release"));
        assert!(!is_safe_command("curl http://example.com"));
        assert!(!is_safe_command("sudo apt install foo"));
        assert!(!is_safe_command("git push"));
    }

    #[test]
    fn test_ls_does_not_match_lsof() {
        // Regression: old starts_with("ls") would match "lsof"
        assert!(!is_safe_command("lsof -i :8080"));
    }

    #[test]
    fn test_needs_confirmation_modes() {
        use crate::config::SafetyMode;
        assert!(!needs_confirmation("rm -rf /tmp/foo", &SafetyMode::Yolo));
        assert!(needs_confirmation("rm -rf /tmp/foo", &SafetyMode::Confirm));
        assert!(needs_confirmation("curl example.com", &SafetyMode::Auto));
        assert!(!needs_confirmation("ls -la", &SafetyMode::Auto));
        assert!(needs_confirmation("rm -rf /", &SafetyMode::Auto)); // destructive always blocked
    }

    #[test]
    fn test_describe_mode() {
        use crate::config::SafetyMode;
        assert!(describe_mode(&SafetyMode::Auto).contains("auto"));
        assert!(describe_mode(&SafetyMode::Yolo).contains("yolo"));
        assert!(describe_mode(&SafetyMode::Confirm).contains("confirm"));
    }
}
