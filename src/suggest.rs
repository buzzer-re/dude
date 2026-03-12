use crate::claude;
use crate::config::Config;
use crate::context;
use crate::corrections::Corrections;
use crate::filter;
use crate::ollama;
use crate::profile::Profile;
use crate::session;

pub enum Suggestion {
    Command(String),
    /// Free-form text response (pipe mode analysis)
    Text(String),
    NotAvailable(String),
}

/// Query the configured provider (ollama or claude).
/// Applies secret redaction to the prompt before sending.
fn query_provider(system: &str, prompt: &str, config: &Config) -> Result<String, String> {
    let redacted_prompt = filter::redact_secrets(prompt);
    if config.use_claude() {
        claude::query(system, &redacted_prompt, config)
    } else {
        ollama::query(system, &redacted_prompt, config)
    }
}

/// Check if the configured provider is available.
fn provider_available(config: &Config) -> bool {
    if config.use_claude() {
        claude::check_available(config)
    } else {
        ollama::check_available(config)
    }
}

fn provider_unavailable_msg(config: &Config) -> String {
    if config.use_claude() {
        "dude: claude API key not set. add claude_api_key to config or set ANTHROPIC_API_KEY".into()
    } else {
        "dude: ollama isn't running. start it with: ollama serve".into()
    }
}

/// Main suggestion logic: fast path (local corrections) → slow path (LLM).
pub fn suggest_correction(
    failed_command: &str,
    args: &[String],
    config: &Config,
    profile: &Profile,
) -> Suggestion {
    // Fast path: check local corrections database
    if let Ok(corrections) = Corrections::open() {
        let full = if args.is_empty() {
            failed_command.to_string()
        } else {
            format!("{} {}", failed_command, args.join(" "))
        };

        // Check full command string first (e.g. "gti status" -> "git status")
        if let Some(correction) = corrections.confident_correction(&full) {
            return Suggestion::Command(correction);
        }

        // Check just the command word (e.g. "gti" -> "git"), then append original args
        if let Some(correction) = corrections.confident_correction(failed_command) {
            let suggested = if args.is_empty() || correction.contains(' ') {
                correction
            } else {
                format!("{} {}", correction, args.join(" "))
            };
            return Suggestion::Command(suggested);
        }
    }

    // Slow path: ask LLM
    if !provider_available(config) {
        return Suggestion::NotAvailable(provider_unavailable_msg(config));
    }

    let system = context::build_system_prompt(profile);
    let prompt = context::build_command_prompt(failed_command, args, config.history_context);

    match query_provider(&system, &prompt, config) {
        Ok(response) => {
            let cleaned = clean_response(&response);
            if cleaned.is_empty() {
                Suggestion::NotAvailable("dude: got nothing useful back".into())
            } else {
                Suggestion::Command(cleaned)
            }
        }
        Err(e) => Suggestion::NotAvailable(e),
    }
}

/// Ask a direct question (the "? question" mode).
pub fn ask_question(question: &str, config: &Config, profile: &Profile) -> Suggestion {
    if !provider_available(config) {
        return Suggestion::NotAvailable(provider_unavailable_msg(config));
    }

    let system = context::build_system_prompt(profile);
    let prompt = context::build_question_prompt(question, config.history_context);

    match query_provider(&system, &prompt, config) {
        Ok(response) => {
            let cleaned = clean_response(&response);
            if cleaned.is_empty() {
                Suggestion::NotAvailable("dude: no idea, sorry".into())
            } else {
                // Save exchange to session for follow-up context
                session::save_exchange(question, &cleaned);
                Suggestion::Command(cleaned)
            }
        }
        Err(e) => Suggestion::NotAvailable(e),
    }
}

/// Handle piped input mode: cat something | dude ask "question"
/// Returns Text instead of Command — pipe mode is for analysis, not execution.
pub fn ask_with_pipe(
    question: &str,
    piped_input: &str,
    config: &Config,
    profile: &Profile,
) -> Suggestion {
    if !provider_available(config) {
        return Suggestion::NotAvailable(provider_unavailable_msg(config));
    }

    let system = context::build_pipe_system_prompt(profile);
    let prompt = context::build_pipe_prompt(question, piped_input, config.history_context);

    match query_provider(&system, &prompt, config) {
        Ok(response) => {
            // Strip thinking tags from reasoning models (qwen3, etc.)
            let cleaned = strip_thinking_tags(&response);
            let trimmed = cleaned.trim().to_string();
            if trimmed.is_empty() {
                Suggestion::NotAvailable("dude: no idea, sorry".into())
            } else {
                session::save_exchange(question, &trimmed);
                Suggestion::Text(trimmed)
            }
        }
        Err(e) => Suggestion::NotAvailable(e),
    }
}

/// Strip <think>...</think> tags from reasoning models (qwen3, deepseek, etc.)
fn strip_thinking_tags(response: &str) -> String {
    let mut result = response.to_string();
    // Handle <think>...</think> blocks (qwen3, deepseek-r1)
    while let Some(start) = result.find("<think>") {
        if let Some(end) = result[start..].find("</think>") {
            let abs_end = start + end + 8;
            result = format!("{}{}", &result[..start], &result[abs_end..]);
        } else {
            // Unclosed think tag — strip from <think> to end
            result = result[..start].to_string();
            break;
        }
    }
    result
}

/// Clean up LLM response — strip thinking tags, markdown fences, backticks.
fn clean_response(response: &str) -> String {
    let stripped_think = strip_thinking_tags(response);
    let trimmed = stripped_think.trim();

    // Strip markdown code fences
    let stripped = if trimmed.starts_with("```") {
        let inner = trimmed
            .trim_start_matches("```")
            .trim_start_matches("bash")
            .trim_start_matches("sh")
            .trim_start_matches("zsh")
            .trim_start_matches('\n');
        inner.trim_end_matches("```").trim()
    } else {
        trimmed
    };

    // Strip inline backticks
    let stripped = stripped.trim_matches('`');

    // If multi-line, take just the first line (likely the command)
    let first_line = stripped.lines().next().unwrap_or("").trim();

    first_line.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_response_plain() {
        assert_eq!(clean_response("git status"), "git status");
    }

    #[test]
    fn test_clean_response_backticks() {
        assert_eq!(clean_response("`git status`"), "git status");
    }

    #[test]
    fn test_clean_response_code_fence() {
        assert_eq!(clean_response("```bash\ngit status\n```"), "git status");
    }

    #[test]
    fn test_clean_response_multiline_takes_first() {
        assert_eq!(clean_response("git add .\ngit commit"), "git add .");
    }

    #[test]
    fn test_strip_thinking_tags() {
        let input = "<think>let me think</think>git status";
        assert_eq!(strip_thinking_tags(input), "git status");
    }

    #[test]
    fn test_strip_thinking_tags_unclosed() {
        let input = "<think>still thinking...";
        assert_eq!(strip_thinking_tags(input), "");
    }

    #[test]
    fn test_strip_thinking_tags_with_content_after() {
        let input = "prefix<think>thinking</think>suffix";
        assert_eq!(strip_thinking_tags(input), "prefixsuffix");
    }

    #[test]
    fn test_strip_thinking_tags_malformed_order() {
        // </think> appears before <think> — should not corrupt
        let input = "</think>before<think>thinking</think>after";
        let result = strip_thinking_tags(input);
        assert!(result.contains("after") || result.contains("before"));
    }
}
