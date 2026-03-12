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
pub fn needs_confirmation(command: &str, safety_mode: &str) -> bool {
    match safety_mode {
        "yolo" => false,
        "auto" => {
            if is_destructive(command) {
                return true;
            }
            !is_safe_command(command)
        }
        // "confirm" or anything else — always confirm
        _ => true,
    }
}

/// Check if a command is "safe" enough to run in auto mode without confirmation.
pub fn is_safe_command(command: &str) -> bool {
    let first_word = command.split_whitespace().next().unwrap_or("");
    let safe_commands = [
        "ls",
        "pwd",
        "echo",
        "cat",
        "head",
        "tail",
        "wc",
        "which",
        "where",
        "whoami",
        "date",
        "cal",
        "uptime",
        "df",
        "du",
        "free",
        "uname",
        "git status",
        "git log",
        "git diff",
        "git branch",
        "cargo check",
        "cargo test",
        "cargo build",
        "npm test",
        "npm run",
        "python --version",
        "node --version",
        "rustc --version",
    ];

    // Check full command match first
    if safe_commands.iter().any(|s| command.starts_with(s)) {
        return true;
    }

    // Check first-word-only safe list
    let safe_first = [
        "ls", "pwd", "echo", "whoami", "date", "cal", "uptime", "which", "where",
    ];
    safe_first.contains(&first_word)
}

/// Return the safety mode description for display.
pub fn describe_mode(mode: &str) -> &str {
    match mode {
        "auto" => "auto (safe commands run without confirmation, destructive commands blocked)",
        "yolo" => "yolo (no confirmations — live dangerously)",
        _ => "confirm (always ask before running)",
    }
}
