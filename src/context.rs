use crate::filter;
use crate::history;
use crate::profile::Profile;
use crate::session;

/// Build the system prompt that tells the LLM who it is and who the user is.
pub fn build_system_prompt(profile: &Profile) -> String {
    let user_context = profile.as_context_string();

    format!(
        r#"You are "dude", a shell companion. You help the user by correcting typos and suggesting commands.

Rules:
- Reply with ONLY the corrected command. No explanation, no markdown, no backticks.
- If the input is a typo of a real command, return the corrected command with the same arguments.
- If the input is natural language, return the shell command that accomplishes it.
- If you're not sure, return the single best guess.
- Never suggest destructive commands (rm -rf /, dd if=/dev/zero, etc.) without explicit paths.
- Match the user's style (short flags vs long flags, tools they use).
- If the user references a previous exchange ("now filter that", "do that again but..."), use the conversation history to understand what they mean.
- This is simple — do not overthink it. Just output the command immediately.

User context:
{user_context}"#
    )
}

/// Build the system prompt for pipe mode — allows text analysis responses.
pub fn build_pipe_system_prompt(profile: &Profile) -> String {
    let user_context = profile.as_context_string();

    format!(
        r#"You are "dude", a shell companion. The user has piped data to you for analysis.

Rules:
- Analyze the piped input and answer the user's question about it.
- Be concise and direct. No markdown formatting.
- If the user asks for a summary, give a short summary.
- If the user asks to filter, show only the matching lines.
- If the user asks why something failed, explain the error.
- Keep responses short — a few lines max.
- Do not overthink. Just answer directly.

User context:
{user_context}"#
    )
}

/// Build the user prompt for a command-not-found scenario.
pub fn build_command_prompt(failed_command: &str, args: &[String], history_count: usize) -> String {
    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "unknown".into());

    let recent = history::recent_history(history_count);
    let history_str = if recent.is_empty() {
        String::new()
    } else {
        let hist: Vec<&str> = recent.iter().map(|s| s.as_str()).collect();
        format!("\nRecent commands:\n{}", hist.join("\n"))
    };

    let full_command = if args.is_empty() {
        failed_command.to_string()
    } else {
        format!("{} {}", failed_command, args.join(" "))
    };

    let session_str = session::session_context_string();

    format!("Command not found: {full_command}\nCWD: {cwd}{history_str}{session_str}")
}

/// Build prompt for a direct "? question" query.
pub fn build_question_prompt(question: &str, history_count: usize) -> String {
    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "unknown".into());

    let recent = history::recent_history(history_count);
    let history_str = if recent.is_empty() {
        String::new()
    } else {
        let last_few: Vec<&str> = recent
            .iter()
            .rev()
            .take(5)
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        format!("\nRecent commands:\n{}", last_few.join("\n"))
    };

    let session_str = session::session_context_string();
    let last_cmd = load_last_command_context();

    format!("User asks: {question}\nCWD: {cwd}{history_str}{session_str}{last_cmd}")
}

/// Build prompt for pipe mode — includes piped stdin content.
pub fn build_pipe_prompt(question: &str, piped_input: &str, history_count: usize) -> String {
    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "unknown".into());

    let recent = history::recent_history(history_count);
    let history_str = if recent.is_empty() {
        String::new()
    } else {
        let last_few: Vec<&str> = recent
            .iter()
            .rev()
            .take(5)
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        format!("\nRecent commands:\n{}", last_few.join("\n"))
    };

    // Truncate piped input to avoid overwhelming the LLM
    let truncated = if piped_input.len() > 4000 {
        let head: String = piped_input.chars().take(2000).collect();
        let tail: String = piped_input
            .chars()
            .rev()
            .take(1500)
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        format!("{head}\n... [truncated] ...\n{tail}")
    } else {
        piped_input.to_string()
    };

    let filtered = filter::redact_secrets(&truncated);

    format!(
        "User asks: {question}\nCWD: {cwd}{history_str}\n\nPiped input:\n```\n{filtered}\n```"
    )
}

/// Load last command context from the temp file written by the shell plugin.
fn load_last_command_context() -> String {
    let path = crate::config::last_cmd_path();
    if !path.exists() {
        return String::new();
    }

    match std::fs::read_to_string(&path) {
        Ok(content) if !content.trim().is_empty() => {
            format!("\nLast command context:\n{}", content.trim())
        }
        _ => String::new(),
    }
}

/// Build the full context string for the `dude context` transparency command.
pub fn build_full_context_display(question: &str, profile: &Profile, history_count: usize) -> String {
    let system = build_system_prompt(profile);
    let prompt = build_question_prompt(question, history_count);
    let filtered_prompt = filter::redact_secrets(&prompt);

    format!(
        "=== SYSTEM PROMPT ===\n{system}\n\n=== USER PROMPT ===\n{filtered_prompt}\n"
    )
}
