use colored::Colorize;
use std::io::{self, Read};

use crate::{
    audit, claude, config, context, corrections, ollama, profile, safety, session, suggest, tui,
};

pub fn help() {
    println!("{}", "dude — your shell companion".yellow().bold());
    println!();
    println!(
        "  {} {}",
        "dude learn".white().bold(),
        "analyze your shell history".dimmed()
    );
    println!(
        "  {} {}",
        "dude profile".white().bold(),
        "see what dude knows about you".dimmed()
    );
    println!(
        "  {} {}",
        "dude ask <question>".white().bold(),
        "ask dude for a command".dimmed()
    );
    println!(
        "  {} {}",
        "dude history".white().bold(),
        "see past interactions".dimmed()
    );
    println!(
        "  {} {}",
        "dude forget".white().bold(),
        "wipe all learned data".dimmed()
    );
    println!(
        "  {} {}",
        "dude config".white().bold(),
        "open config in your editor".dimmed()
    );
    println!(
        "  {} {}",
        "dude status".white().bold(),
        "check provider status".dimmed()
    );
    println!(
        "  {} {}",
        "dude context <question>".white().bold(),
        "show what would be sent to the LLM".dimmed()
    );
    println!(
        "  {} {}",
        "dude model [name]".white().bold(),
        "show or set the current model".dimmed()
    );
    println!(
        "  {} {}",
        "dude provider [name]".white().bold(),
        "show or set provider (ollama/claude)".dimmed()
    );
    println!(
        "  {} {}",
        "dude clear".white().bold(),
        "clear conversation session".dimmed()
    );
    println!();
    println!(
        "  {} {}",
        "dude <question>".white().bold(),
        "ask dude anything (no subcommand needed)".dimmed()
    );
    println!(
        "  {} {}",
        "cmd | dude <question>".white().bold(),
        "pipe mode — analyze piped output".dimmed()
    );
    println!(
        "  {} {}",
        "? <question>".white().bold(),
        "quick ask (via shell plugin)".dimmed()
    );
    println!();
    println!(
        "  {}",
        "Just type normally — dude intercepts command-not-found automatically.".dimmed()
    );
}

pub fn learn() {
    eprintln!(
        "{} analyzing your shell history...",
        "dude:".yellow().bold()
    );
    let profile = profile::Profile::analyze_and_build();
    eprintln!("{} done! learned your patterns.", "dude:".yellow().bold());
    println!();
    profile::display_profile(&profile);
}

pub fn show_profile() {
    let profile = profile::Profile::load();
    if profile.user.name.is_empty() {
        eprintln!(
            "{} no profile yet. run {} first.",
            "dude:".yellow().bold(),
            "dude learn".white().bold()
        );
    } else {
        profile::display_profile(&profile);
    }
}

pub fn history(count: usize) {
    let entries = audit::recent_entries(count);
    if entries.is_empty() {
        eprintln!("{} no history yet.", "dude:".yellow().bold());
        return;
    }

    for entry in &entries {
        let status = if entry.accepted {
            "✓".green()
        } else {
            "✗".red()
        };
        let suggestion = entry
            .suggestion
            .as_deref()
            .unwrap_or("-");
        println!(
            "  {} {} → {} {}",
            entry.timestamp.dimmed(),
            entry.input.white(),
            suggestion.cyan(),
            status
        );
    }
}

pub fn forget() {
    let dude_dir = config::dude_dir();
    if dude_dir.exists() {
        let _ = std::fs::remove_file(config::db_path());
        let _ = std::fs::remove_file(config::profile_path());
        let _ = std::fs::remove_file(config::history_path());
        session::clear_session();
        eprintln!("{} all learned data wiped.", "dude:".yellow().bold());
    } else {
        eprintln!("{} nothing to forget.", "dude:".yellow().bold());
    }
}

pub fn edit_config() {
    tui::run_config_tui();
}

pub fn ask(question: &str) {
    if question.trim().is_empty() {
        eprintln!("{} ask me something!", "dude:".yellow().bold());
        return;
    }

    let config = config::Config::load();
    let profile = profile::Profile::load();

    let piped = read_stdin_if_piped();

    let result = if let Some(piped_input) = piped {
        suggest::ask_with_pipe(question, &piped_input, &config, &profile)
    } else {
        suggest::ask_question(question, &config, &profile)
    };

    match result {
        suggest::Suggestion::Command(cmd) => {
            if safety::is_destructive(&cmd) {
                eprintln!(
                    "{} {} (blocked — looks destructive)",
                    "dude:".yellow().bold(),
                    cmd.red().bold()
                );
                audit::log_interaction(question, Some(&cmd), false);
                return;
            }
            eprintln!("{} {}", "dude:".yellow().bold(), cmd.white().bold());
            println!("{cmd}");
            audit::log_interaction(question, Some(&cmd), false);
        }
        suggest::Suggestion::Text(text) => {
            eprintln!("{}", "dude:".yellow().bold());
            eprintln!("{text}");
            audit::log_interaction(question, Some(&text), false);
        }
        suggest::Suggestion::NotAvailable(msg) => {
            eprintln!("{msg}");
        }
    }
}

pub fn command_not_found(failed_cmd: &str, args: &[String]) {
    let config = config::Config::load();
    let profile = profile::Profile::load();

    let full_input = if args.is_empty() {
        failed_cmd.to_string()
    } else {
        format!("{} {}", failed_cmd, args.join(" "))
    };

    match suggest::suggest_correction(failed_cmd, args, &config, &profile) {
        suggest::Suggestion::Command(cmd) => {
            if safety::is_destructive(&cmd) {
                eprintln!(
                    "{} {} (blocked — looks destructive)",
                    "dude:".yellow().bold(),
                    cmd.red().bold()
                );
                audit::log_interaction(&full_input, Some(&cmd), false);
                std::process::exit(2);
            }

            eprintln!("{} {}", "dude:".yellow().bold(), cmd.white().bold());
            println!("{cmd}");
            audit::log_interaction(&full_input, Some(&cmd), false);
        }
        suggest::Suggestion::Text(text) => {
            eprintln!("{} {}", "dude:".yellow().bold(), text.white());
            audit::log_interaction(&full_input, Some(&text), false);
            std::process::exit(1);
        }
        suggest::Suggestion::NotAvailable(msg) => {
            eprintln!("{msg}");
            audit::log_interaction(&full_input, None, false);
            std::process::exit(1);
        }
    }
}

pub fn accept(typo: &str, correction: &str) {
    if let Ok(corrections) = corrections::Corrections::open() {
        corrections.record(typo, correction);
        audit::log_interaction(typo, Some(correction), true);
    }
}

pub fn status() {
    let config = config::Config::load();

    if config.needs_setup() {
        eprintln!(
            "{} not configured yet. run {} to set up.",
            "dude:".yellow().bold(),
            "dude config".white().bold()
        );
        return;
    }

    eprintln!(
        "{} provider: {}",
        "dude:".yellow().bold(),
        config.effective_provider().to_string().white().bold()
    );

    if config.use_claude() {
        if claude::check_available(&config) {
            eprintln!("{} claude auth is set", "dude:".yellow().bold());
            let model = config.effective_claude_model();
            eprintln!(
                "{} model: {}",
                "dude:".yellow().bold(),
                model.white().bold()
            );
        } else {
            eprintln!("{} claude credentials not found", "dude:".red().bold());
        }
    } else if ollama::check_available(&config) {
        eprintln!(
            "{} ollama is up at {}",
            "dude:".yellow().bold(),
            config.effective_ollama_url().cyan()
        );
        eprintln!(
            "{} model: {}",
            "dude:".yellow().bold(),
            config.effective_model().white().bold()
        );
    } else {
        eprintln!(
            "{} ollama is not reachable at {}",
            "dude:".red().bold(),
            config.effective_ollama_url()
        );
        eprintln!("  try: {}", "ollama serve".white().bold());
    }

    eprintln!(
        "{} safety: {}",
        "dude:".yellow().bold(),
        safety::describe_mode(config.effective_safety_mode()).dimmed()
    );
}

pub fn show_context(question: &str) {
    let question = if question.is_empty() {
        "how do I find large files"
    } else {
        question
    };

    let config = config::Config::load();
    let profile = profile::Profile::load();

    let display = context::build_full_context_display(question, &profile, config.history_context);
    println!("{display}");

    eprintln!(
        "{} this is exactly what would be sent to {} (secrets redacted)",
        "dude:".yellow().bold(),
        config.effective_provider().to_string().white().bold()
    );
}

pub fn model(name: Option<String>) {
    let mut config = config::Config::load();

    match name {
        Some(model_name) => {
            if config.use_claude() {
                config.claude_model = Some(model_name.clone());
            } else {
                config.model = model_name.clone();
            }
            config.save();
            eprintln!(
                "{} {} model set to {}",
                "dude:".yellow().bold(),
                config.effective_provider().to_string().white(),
                model_name.white().bold()
            );
        }
        None => {
            eprintln!(
                "{} current model: {} ({})",
                "dude:".yellow().bold(),
                config.active_model().white().bold(),
                config.effective_provider()
            );
        }
    }
}

pub fn provider(name: Option<String>) {
    let mut config = config::Config::load();

    match name {
        Some(provider_name) => {
            let Some(parsed) = config::Provider::from_str_lenient(&provider_name) else {
                eprintln!(
                    "{} unknown provider '{}'. use 'ollama' or 'claude'",
                    "dude:".red().bold(),
                    provider_name
                );
                std::process::exit(1);
            };
            config.provider = parsed;
            config.save();
            eprintln!(
                "{} provider set to {}",
                "dude:".yellow().bold(),
                provider_name.white().bold()
            );
        }
        None => {
            eprintln!(
                "{} current provider: {}",
                "dude:".yellow().bold(),
                config.effective_provider().to_string().white().bold()
            );
        }
    }
}

pub fn clear_session() {
    session::clear_session();
    eprintln!("{} conversation cleared.", "dude:".yellow().bold());
}

pub fn safety_check(command: &str) {
    let config = config::Config::load();
    let needs = safety::needs_confirmation(command, config.effective_safety_mode());

    if safety::is_destructive(command) {
        eprintln!(
            "{} {} — DESTRUCTIVE, always blocked",
            "dude:".red().bold(),
            command.red()
        );
        std::process::exit(2);
    } else if needs {
        eprintln!(
            "{} {} — needs confirmation",
            "dude:".yellow().bold(),
            command.white()
        );
        std::process::exit(1);
    } else {
        eprintln!(
            "{} {} — safe to auto-run",
            "dude:".yellow().bold(),
            command.green()
        );
    }
}

/// Read stdin if it's piped (not a terminal).
fn read_stdin_if_piped() -> Option<String> {
    use std::io::IsTerminal;
    if io::stdin().is_terminal() {
        return None;
    }
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).ok()?;
    if input.is_empty() {
        None
    } else {
        Some(input)
    }
}
