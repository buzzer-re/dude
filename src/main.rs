mod audit;
mod claude;
mod commands;
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
mod tui;

use clap::{Parser, Subcommand};

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
        commands::ask(&question);
        return;
    }

    let cli = Cli::parse();

    match cli.command {
        None => commands::help(),
        Some(Commands::Learn) => commands::learn(),
        Some(Commands::Profile) => commands::show_profile(),
        Some(Commands::History { count }) => commands::history(count),
        Some(Commands::Forget) => commands::forget(),
        Some(Commands::Config) => commands::edit_config(),
        Some(Commands::Ask { question }) => commands::ask(&question.join(" ")),
        Some(Commands::Accept { typo, correction }) => commands::accept(&typo, &correction),
        Some(Commands::Status) => commands::status(),
        Some(Commands::Context { question }) => commands::show_context(&question.join(" ")),
        Some(Commands::Model { name }) => commands::model(name),
        Some(Commands::Provider { name }) => commands::provider(name),
        Some(Commands::ClearSession) => commands::clear_session(),
        Some(Commands::SafetyCheck { command }) => commands::safety_check(&command.join(" ")),
    }
}

/// Check if an argument is a known subcommand (so we don't intercept it as a bare query).
fn is_known_subcommand(arg: &str) -> bool {
    matches!(
        arg,
        "learn"
            | "profile"
            | "history"
            | "forget"
            | "config"
            | "ask"
            | "accept"
            | "status"
            | "context"
            | "model"
            | "provider"
            | "clear"
            | "safety-check"
            | "help"
            | "--help"
            | "-h"
            | "--version"
            | "-V"
    )
}
