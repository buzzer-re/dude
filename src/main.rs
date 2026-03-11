mod audit;
mod claude;
mod config;
mod context;
mod corrections;
mod filter;
mod history;
mod ollama;
mod profile;
mod safety;
mod session;
mod suggest;

use clap::{Parser, Subcommand};
use colored::Colorize;
use std::io::{self, Read};

#[derive(Parser)]
#[command(name = "dude", about = "Your shell companion", version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Analyze shell history and build your profile
    Learn,
    /// Show what dude knows about you
    Profile,
    /// Show past dude interactions
    History {
        /// Number of entries to show
        #[arg(short, long, default_value_t = 20)]
        count: usize,
    },
    /// Wipe all learned data
    Forget,
    /// Open config in $EDITOR
    Config,
    /// Ask dude a question (returns a command). Supports piped input.
    Ask {
        /// The question
        question: Vec<String>,
    },
    /// Handle a command-not-found (called by the shell plugin)
    #[command(name = "cnf")]
    CommandNotFound {
        /// The command that wasn't found
        cmd: String,
        /// Arguments that were passed
        args: Vec<String>,
    },
    /// Record that the user accepted a suggestion
    #[command(name = "accept")]
    Accept {
        /// The original typo
        typo: String,
        /// The accepted correction
        correction: String,
    },
    /// Check if ollama is reachable
    Status,
    /// Show what would be sent to the LLM (transparency)
    Context {
        /// Example question to show context for
        question: Vec<String>,
    },
    /// Set the ollama model
    Model {
        /// Model name (e.g. qwen2.5-coder:1.5b)
        name: Option<String>,
    },
    /// Set the provider (ollama or claude)
    Provider {
        /// Provider name
        name: Option<String>,
    },
    /// Clear conversation session
    #[command(name = "clear")]
    ClearSession,
    /// Check if a command needs confirmation in the current safety mode
    #[command(name = "safety-check")]
    SafetyCheck {
        /// The command to check
        command: Vec<String>,
    },
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // If first arg isn't a known subcommand, treat all args as a bare query:
    //   dude "what is this"
    //   cat file | dude "summarize"
    if args.len() > 1 && !is_known_subcommand(&args[1]) {
        let question = args[1..].join(" ");
        cmd_ask(&question);
        return;
    }

    let cli = Cli::parse();

    match cli.command {
        None => cmd_help(),
        Some(Commands::Learn) => cmd_learn(),
        Some(Commands::Profile) => cmd_profile(),
        Some(Commands::History { count }) => cmd_history(count),
        Some(Commands::Forget) => cmd_forget(),
        Some(Commands::Config) => cmd_config(),
        Some(Commands::Ask { question }) => cmd_ask(&question.join(" ")),
        Some(Commands::CommandNotFound { cmd, args }) => cmd_cnf(&cmd, &args),
        Some(Commands::Accept { typo, correction }) => cmd_accept(&typo, &correction),
        Some(Commands::Status) => cmd_status(),
        Some(Commands::Context { question }) => cmd_context(&question.join(" ")),
        Some(Commands::Model { name }) => cmd_model(name),
        Some(Commands::Provider { name }) => cmd_provider(name),
        Some(Commands::ClearSession) => cmd_clear_session(),
        Some(Commands::SafetyCheck { command }) => cmd_safety_check(&command.join(" ")),
    }
}

fn cmd_help() {
    println!("{}", "dude — your shell companion".yellow().bold());
    println!();
    println!("  {} {}", "dude learn".white().bold(), "analyze your shell history".dimmed());
    println!("  {} {}", "dude profile".white().bold(), "see what dude knows about you".dimmed());
    println!("  {} {}", "dude ask <question>".white().bold(), "ask dude for a command".dimmed());
    println!("  {} {}", "dude history".white().bold(), "see past interactions".dimmed());
    println!("  {} {}", "dude forget".white().bold(), "wipe all learned data".dimmed());
    println!("  {} {}", "dude config".white().bold(), "open config in your editor".dimmed());
    println!("  {} {}", "dude status".white().bold(), "check provider status".dimmed());
    println!("  {} {}", "dude context <question>".white().bold(), "show what would be sent to the LLM".dimmed());
    println!("  {} {}", "dude model [name]".white().bold(), "show or set the current model".dimmed());
    println!("  {} {}", "dude provider [name]".white().bold(), "show or set provider (ollama/claude)".dimmed());
    println!("  {} {}", "dude clear".white().bold(), "clear conversation session".dimmed());
    println!();
    println!("  {} {}", "dude <question>".white().bold(), "ask dude anything (no subcommand needed)".dimmed());
    println!("  {} {}", "cmd | dude <question>".white().bold(), "pipe mode — analyze piped output".dimmed());
    println!("  {} {}", "? <question>".white().bold(), "quick ask (via shell plugin)".dimmed());
    println!();
    println!(
        "  {}",
        "Just type normally — dude intercepts command-not-found automatically.".dimmed()
    );
}

fn cmd_learn() {
    eprintln!("{} analyzing your shell history...", "dude:".yellow().bold());
    let profile = profile::Profile::analyze_and_build();
    eprintln!("{} done! learned your patterns.", "dude:".yellow().bold());
    println!();
    profile::display_profile(&profile);
}

fn cmd_profile() {
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

fn cmd_history(count: usize) {
    let path = config::history_path();
    if !path.exists() {
        eprintln!("{} no history yet.", "dude:".yellow().bold());
        return;
    }

    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => {
            eprintln!("{} couldn't read history.", "dude:".yellow().bold());
            return;
        }
    };

    let lines: Vec<&str> = content.lines().collect();
    let start = lines.len().saturating_sub(count);

    for line in &lines[start..] {
        if let Ok(entry) = serde_json::from_str::<audit::AuditEntry>(line) {
            let status = if entry.accepted {
                "✓".green()
            } else {
                "✗".red()
            };
            let suggestion = entry.suggestion.unwrap_or_else(|| "-".into());
            println!(
                "  {} {} → {} {}",
                entry.timestamp.dimmed(),
                entry.input.white(),
                suggestion.cyan(),
                status
            );
        }
    }
}

fn cmd_forget() {
    let dude_dir = config::dude_dir();
    if dude_dir.exists() {
        // Remove learned data but keep config
        let _ = std::fs::remove_file(config::db_path());
        let _ = std::fs::remove_file(config::profile_path());
        let _ = std::fs::remove_file(config::history_path());
        session::clear_session();
        eprintln!("{} all learned data wiped.", "dude:".yellow().bold());
    } else {
        eprintln!("{} nothing to forget.", "dude:".yellow().bold());
    }
}

fn cmd_config() {
    let path = config::config_path();
    // Ensure config exists
    let _ = config::Config::load();

    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".into());
    let status = std::process::Command::new(&editor)
        .arg(path.to_str().unwrap_or(""))
        .status();

    if let Err(e) = status {
        eprintln!("{} couldn't open editor: {e}", "dude:".yellow().bold());
    }
}

fn cmd_ask(question: &str) {
    if question.trim().is_empty() {
        eprintln!("{} ask me something!", "dude:".yellow().bold());
        return;
    }

    let config = config::Config::load();
    let profile = profile::Profile::load();

    // Check if stdin has piped content
    let piped = read_stdin_if_piped();

    let result = if let Some(piped_input) = piped {
        suggest::ask_with_pipe(question, &piped_input, &config, &profile)
    } else {
        suggest::ask_question(question, &config, &profile)
    };

    match result {
        suggest::Suggestion::Command(cmd) => {
            eprintln!("{} {}", "dude:".yellow().bold(), cmd.white().bold());
            println!("{cmd}");
            audit::log_interaction(question, Some(&cmd), false);
        }
        suggest::Suggestion::Text(text) => {
            // Pipe mode: print analysis text directly (no "run it?" prompt)
            eprintln!("{}", "dude:".yellow().bold());
            eprintln!("{text}");
            audit::log_interaction(question, Some(&text), false);
        }
        suggest::Suggestion::NotAvailable(msg) => {
            eprintln!("{msg}");
        }
    }
}

fn cmd_cnf(failed_cmd: &str, args: &[String]) {
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

            // Output the suggestion to stderr (visible to user)
            eprintln!("{} {}", "dude:".yellow().bold(), cmd.white().bold());
            // Output the command to stdout (captured by shell plugin)
            println!("{cmd}");
            audit::log_interaction(&full_input, Some(&cmd), false);
        }
        suggest::Suggestion::Text(text) => {
            // Shouldn't happen in cnf mode, but handle it
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

fn cmd_accept(typo: &str, correction: &str) {
    if let Ok(corrections) = corrections::Corrections::open() {
        corrections.record(typo, correction);
        audit::log_interaction(typo, Some(correction), true);
    }
}

fn cmd_status() {
    let config = config::Config::load();

    eprintln!(
        "{} provider: {}",
        "dude:".yellow().bold(),
        if config.use_claude() { "claude" } else { "ollama" }.white().bold()
    );

    if config.use_claude() {
        if claude::check_available(&config) {
            eprintln!(
                "{} claude API key is set",
                "dude:".yellow().bold(),
            );
            let model = config.claude_model.as_deref().unwrap_or("claude-haiku-4-5-20251001");
            eprintln!(
                "{} model: {}",
                "dude:".yellow().bold(),
                model.white().bold()
            );
        } else {
            eprintln!(
                "{} claude API key not set",
                "dude:".red().bold(),
            );
        }
    } else {
        if ollama::check_available(&config) {
            eprintln!(
                "{} ollama is up at {}",
                "dude:".yellow().bold(),
                config.ollama_url.cyan()
            );
            eprintln!(
                "{} model: {}",
                "dude:".yellow().bold(),
                config.model.white().bold()
            );
        } else {
            eprintln!(
                "{} ollama is not reachable at {}",
                "dude:".red().bold(),
                config.ollama_url
            );
            eprintln!("  try: {}", "ollama serve".white().bold());
        }
    }

    eprintln!(
        "{} safety: {}",
        "dude:".yellow().bold(),
        safety::describe_mode(&config.safety_mode).dimmed()
    );
}

fn cmd_context(question: &str) {
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
        if config.use_claude() { "claude" } else { "ollama" }.white().bold()
    );
}

fn cmd_model(name: Option<String>) {
    let mut config = config::Config::load();

    match name {
        Some(model_name) => {
            config.model = model_name.clone();
            config.save();
            eprintln!(
                "{} model set to {}",
                "dude:".yellow().bold(),
                model_name.white().bold()
            );
        }
        None => {
            eprintln!(
                "{} current model: {}",
                "dude:".yellow().bold(),
                config.model.white().bold()
            );
        }
    }
}

fn cmd_provider(name: Option<String>) {
    let mut config = config::Config::load();

    match name {
        Some(provider_name) => {
            let valid = matches!(provider_name.as_str(), "ollama" | "claude");
            if !valid {
                eprintln!(
                    "{} unknown provider '{}'. use 'ollama' or 'claude'",
                    "dude:".red().bold(),
                    provider_name
                );
                std::process::exit(1);
            }
            config.provider = provider_name.clone();
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
                config.provider.white().bold()
            );
        }
    }
}

fn cmd_clear_session() {
    session::clear_session();
    eprintln!("{} conversation cleared.", "dude:".yellow().bold());
}

fn cmd_safety_check(command: &str) {
    let config = config::Config::load();
    let needs = safety::needs_confirmation(command, &config.safety_mode);

    if safety::is_destructive(command) {
        eprintln!(
            "{} {} — DESTRUCTIVE, always blocked",
            "dude:".red().bold(),
            command.red()
        );
        // Exit code 2 = destructive
        std::process::exit(2);
    } else if needs {
        eprintln!(
            "{} {} — needs confirmation",
            "dude:".yellow().bold(),
            command.white()
        );
        // Exit code 1 = needs confirmation
        std::process::exit(1);
    } else {
        eprintln!(
            "{} {} — safe to auto-run",
            "dude:".yellow().bold(),
            command.green()
        );
        // Exit code 0 = safe
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

/// Check if an argument is a known subcommand (so we don't intercept it as a bare query).
fn is_known_subcommand(arg: &str) -> bool {
    matches!(
        arg,
        "learn" | "profile" | "history" | "forget" | "config" | "ask"
            | "cnf" | "accept" | "status" | "context" | "model"
            | "provider" | "clear" | "safety-check" | "help" | "--help"
            | "-h" | "--version" | "-V"
    )
}
