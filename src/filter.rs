/// Filter sensitive values from text before sending to the LLM.
/// Redacts environment variables and strings that look like secrets.
const SENSITIVE_PATTERNS: &[&str] = &[
    "KEY",
    "TOKEN",
    "SECRET",
    "PASSWORD",
    "PASSWD",
    "CREDENTIAL",
    "API_KEY",
    "APIKEY",
    "AUTH",
    "PRIVATE",
    "ACCESS_KEY",
];

/// Redact environment variable values that look like secrets.
/// e.g. "AWS_SECRET_KEY=abc123" → "AWS_SECRET_KEY=[REDACTED]"
pub fn redact_secrets(text: &str) -> String {
    let mut result = String::with_capacity(text.len());

    for line in text.lines() {
        if looks_like_secret_assignment(line) {
            // Redact the value part
            if let Some(eq_pos) = line.find('=') {
                result.push_str(&line[..=eq_pos]);
                result.push_str("[REDACTED]");
            } else {
                result.push_str(line);
            }
        } else {
            result.push_str(line);
        }
        result.push('\n');
    }

    // Remove trailing newline if original didn't have one
    if !text.ends_with('\n') {
        result.pop();
    }

    result
}

fn looks_like_secret_assignment(line: &str) -> bool {
    let trimmed = line.trim();

    // Must have an = sign
    let Some(eq_pos) = trimmed.find('=') else {
        return false;
    };

    let var_name = &trimmed[..eq_pos].trim_start_matches("export ");
    let var_upper = var_name.to_uppercase();

    SENSITIVE_PATTERNS
        .iter()
        .any(|pattern| var_upper.contains(pattern))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_secrets() {
        let input = "AWS_SECRET_KEY=abc123\nHOME=/home/user\nAPI_TOKEN=xyz";
        let result = redact_secrets(input);
        assert!(result.contains("AWS_SECRET_KEY=[REDACTED]"));
        assert!(result.contains("HOME=/home/user"));
        assert!(result.contains("API_TOKEN=[REDACTED]"));
    }

    #[test]
    fn test_no_false_positives() {
        let input = "PATH=/usr/bin\nSHELL=/bin/zsh";
        let result = redact_secrets(input);
        assert_eq!(result, input);
    }
}
