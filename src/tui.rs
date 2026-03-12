use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    prelude::CrosstermBackend,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Terminal,
};
use std::io::stdout;

use crate::claude;
use crate::config::Config;

// ─── Field definitions ──────────────────────────────────────────────────

#[derive(Clone)]
struct ConfigField {
    label: &'static str,
    kind: FieldKind,
}

#[derive(Clone)]
enum FieldKind {
    Select {
        options: Vec<(&'static str, &'static str)>, // (value, description)
        get: fn(&Config) -> String,
        set: fn(&mut Config, String),
    },
    Text {
        get: fn(&Config) -> String,
        set: fn(&mut Config, String),
        placeholder: &'static str,
    },
    Number {
        get: fn(&Config) -> usize,
        set: fn(&mut Config, usize),
    },
    ReadOnly {
        get: fn(&Config) -> String,
    },
}

fn build_fields() -> Vec<ConfigField> {
    vec![
        ConfigField {
            label: "Provider",
            kind: FieldKind::Select {
                options: vec![
                    ("ollama", "Local LLM via ollama"),
                    ("claude", "Anthropic Claude API"),
                ],
                get: |c| {
                    if c.provider.is_empty() {
                        "ollama".into()
                    } else {
                        c.provider.clone()
                    }
                },
                set: |c, v| c.provider = v,
            },
        },
        ConfigField {
            label: "Ollama Model",
            kind: FieldKind::Select {
                options: vec![
                    (
                        "qwen2.5-coder:1.5b",
                        "Fast, small — best for typo correction",
                    ),
                    ("qwen2.5-coder:7b", "Balanced speed and smarts"),
                    ("llama3.1:8b", "Good all-rounder"),
                    ("qwen3.5:2b", "Reasoning model (slower)"),
                    ("gemma3:4b", "Google's compact model"),
                ],
                get: |c| {
                    if c.model.is_empty() || c.model.starts_with("claude") {
                        String::new()
                    } else {
                        c.model.clone()
                    }
                },
                set: |c, v| c.model = v,
            },
        },
        ConfigField {
            label: "Claude Model",
            kind: FieldKind::Select {
                options: vec![
                    ("claude-haiku-4-5-20251001", "Fast and cheap"),
                    ("claude-sonnet-4-6", "Smarter, balanced"),
                    ("claude-opus-4-6", "Most capable"),
                ],
                get: |c| c.claude_model.as_deref().unwrap_or("").to_string(),
                set: |c, v| c.claude_model = Some(v),
            },
        },
        ConfigField {
            label: "Safety Mode",
            kind: FieldKind::Select {
                options: vec![
                    ("confirm", "Always ask before running"),
                    ("auto", "Safe commands auto-run, others ask"),
                    ("yolo", "Never ask — live dangerously"),
                ],
                get: |c| c.safety_mode.clone(),
                set: |c, v| c.safety_mode = v,
            },
        },
        ConfigField {
            label: "Ollama URL",
            kind: FieldKind::Text {
                get: |c| c.ollama_url.clone(),
                set: |c, v| c.ollama_url = v,
                placeholder: "http://localhost:11434",
            },
        },
        ConfigField {
            label: "History Context",
            kind: FieldKind::Number {
                get: |c| c.history_context,
                set: |c, v| c.history_context = v,
            },
        },
        ConfigField {
            label: "Claude API Key",
            kind: FieldKind::Text {
                get: |c| {
                    if let Some(k) = c.claude_api_key.as_deref().filter(|k| !k.is_empty()) {
                        format!("{}...", &k[..k.len().min(12)])
                    } else {
                        String::new()
                    }
                },
                set: |c, v| {
                    if v.is_empty() {
                        c.claude_api_key = None;
                    } else {
                        c.claude_api_key = Some(v);
                    }
                },
                placeholder: "sk-ant-... (leave empty for keychain oauth)",
            },
        },
        ConfigField {
            label: "Claude Auth",
            kind: FieldKind::ReadOnly {
                get: |c| {
                    if c.claude_api_key
                        .as_deref()
                        .map(|k| !k.is_empty())
                        .unwrap_or(false)
                    {
                        "API key (from config)".into()
                    } else if claude::check_available(c) {
                        "OAuth (from macOS Keychain)".into()
                    } else {
                        "Not configured".into()
                    }
                },
            },
        },
    ]
}

// ─── App state ──────────────────────────────────────────────────────────

enum Mode {
    /// Navigating the main field list
    Main,
    /// Selecting from options for a Select field
    Selecting { field_idx: usize, cursor: usize },
    /// Editing text for a Text field
    Editing { field_idx: usize, buffer: String },
    /// Editing a number
    EditingNumber { field_idx: usize, buffer: String },
}

struct App {
    config: Config,
    fields: Vec<ConfigField>,
    main_cursor: usize,
    mode: Mode,
    dirty: bool,
}

impl App {
    fn new(config: Config) -> Self {
        Self {
            config,
            fields: build_fields(),
            main_cursor: 0,
            mode: Mode::Main,
            dirty: false,
        }
    }

    fn current_value(&self, idx: usize) -> String {
        match &self.fields[idx].kind {
            FieldKind::Select { get, .. } => get(&self.config),
            FieldKind::Text { get, .. } => get(&self.config),
            FieldKind::Number { get, .. } => get(&self.config).to_string(),
            FieldKind::ReadOnly { get } => get(&self.config),
        }
    }
}

// ─── TUI entry point ────────────────────────────────────────────────────

pub fn run_config_tui() {
    let config = Config::load();
    let mut app = App::new(config);

    // Setup terminal — bail gracefully if not a real TTY
    if enable_raw_mode().is_err() {
        eprintln!("dude: can't open interactive config (not a terminal)");
        eprintln!("  use: dude model <name>, dude provider <name>, etc.");
        return;
    }
    stdout()
        .execute(EnterAlternateScreen)
        .expect("Failed to enter alternate screen");
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend).expect("Failed to create terminal");

    // Main loop
    loop {
        terminal.draw(|f| draw(f, &app)).expect("Failed to draw");

        if let Ok(Event::Key(key)) = event::read() {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match &mut app.mode {
                Mode::Main => match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Up | KeyCode::Char('k') => {
                        if app.main_cursor > 0 {
                            app.main_cursor -= 1;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if app.main_cursor < app.fields.len() - 1 {
                            app.main_cursor += 1;
                        }
                    }
                    KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                        enter_field(&mut app);
                    }
                    _ => {}
                },
                Mode::Selecting {
                    field_idx,
                    ref mut cursor,
                } => {
                    let field_idx = *field_idx;
                    match key.code {
                        KeyCode::Up | KeyCode::Char('k') => {
                            if *cursor > 0 {
                                *cursor -= 1;
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if let FieldKind::Select { ref options, .. } =
                                app.fields[field_idx].kind
                            {
                                if *cursor < options.len() - 1 {
                                    *cursor += 1;
                                }
                            }
                        }
                        KeyCode::Enter => {
                            let cursor_val = *cursor;
                            if let FieldKind::Select {
                                ref options, set, ..
                            } = app.fields[field_idx].kind
                            {
                                let value = options[cursor_val].0.to_string();
                                set(&mut app.config, value);
                                app.dirty = true;
                            }
                            app.mode = Mode::Main;
                        }
                        KeyCode::Esc | KeyCode::Left | KeyCode::Char('h') => {
                            app.mode = Mode::Main;
                        }
                        _ => {}
                    }
                }
                Mode::Editing {
                    field_idx,
                    ref mut buffer,
                } => {
                    let field_idx = *field_idx;
                    match key.code {
                        KeyCode::Enter => {
                            if let FieldKind::Text { set, .. } = app.fields[field_idx].kind {
                                set(&mut app.config, buffer.clone());
                                app.dirty = true;
                            }
                            app.mode = Mode::Main;
                        }
                        KeyCode::Esc => {
                            app.mode = Mode::Main;
                        }
                        KeyCode::Backspace => {
                            buffer.pop();
                        }
                        KeyCode::Char(c) => {
                            buffer.push(c);
                        }
                        _ => {}
                    }
                }
                Mode::EditingNumber {
                    field_idx,
                    ref mut buffer,
                } => {
                    let field_idx = *field_idx;
                    match key.code {
                        KeyCode::Enter => {
                            if let Ok(n) = buffer.parse::<usize>() {
                                if let FieldKind::Number { set, .. } = app.fields[field_idx].kind {
                                    set(&mut app.config, n);
                                    app.dirty = true;
                                }
                            }
                            app.mode = Mode::Main;
                        }
                        KeyCode::Esc => {
                            app.mode = Mode::Main;
                        }
                        KeyCode::Backspace => {
                            buffer.pop();
                        }
                        KeyCode::Char(c) if c.is_ascii_digit() => {
                            buffer.push(c);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    // Save if changed
    if app.dirty {
        app.config.save();
    }

    // Restore terminal
    disable_raw_mode().expect("Failed to disable raw mode");
    stdout()
        .execute(LeaveAlternateScreen)
        .expect("Failed to leave alternate screen");

    if app.dirty {
        eprintln!(
            "{} config saved.",
            colored::Colorize::bold(colored::Colorize::yellow("dude:"))
        );
    }
}

fn enter_field(app: &mut App) {
    let idx = app.main_cursor;
    match &app.fields[idx].kind {
        FieldKind::Select { options, get, .. } => {
            let current = get(&app.config);
            let cursor = options
                .iter()
                .position(|(v, _)| *v == current.as_str())
                .unwrap_or(0);
            app.mode = Mode::Selecting {
                field_idx: idx,
                cursor,
            };
        }
        FieldKind::Text { get, .. } => {
            let current = get(&app.config);
            // For API key, don't pre-fill the truncated version
            let buffer = if app.fields[idx].label == "Claude API Key" {
                app.config
                    .claude_api_key
                    .as_deref()
                    .unwrap_or("")
                    .to_string()
            } else {
                current
            };
            app.mode = Mode::Editing {
                field_idx: idx,
                buffer,
            };
        }
        FieldKind::Number { get, .. } => {
            let current = get(&app.config);
            app.mode = Mode::EditingNumber {
                field_idx: idx,
                buffer: current.to_string(),
            };
        }
        FieldKind::ReadOnly { .. } => {
            // Can't edit read-only fields
        }
    }
}

// ─── Drawing ────────────────────────────────────────────────────────────

fn draw(f: &mut ratatui::Frame, app: &App) {
    let area = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Min(0),    // Fields
            Constraint::Length(3), // Help bar
        ])
        .split(area);

    // Title
    let title_text = if app.dirty {
        " dude config *"
    } else {
        " dude config"
    };
    let title = Paragraph::new(title_text)
        .style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
    f.render_widget(title, chunks[0]);

    // Fields list
    let items: Vec<ListItem> = app
        .fields
        .iter()
        .enumerate()
        .map(|(i, field)| {
            let value = app.current_value(i);
            let is_readonly = matches!(field.kind, FieldKind::ReadOnly { .. });

            let label_style = if is_readonly {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default().fg(Color::White)
            };

            let value_style = if is_readonly {
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC)
            } else {
                Style::default().fg(Color::Cyan)
            };

            let display_value = if value.is_empty() {
                match &field.kind {
                    FieldKind::Text { placeholder, .. } => {
                        return ListItem::new(Line::from(vec![
                            Span::styled(format!("  {:<18}", field.label), label_style),
                            Span::styled(
                                placeholder.to_string(),
                                Style::default().fg(Color::DarkGray),
                            ),
                        ]));
                    }
                    _ => "(not set)".to_string(),
                }
            } else {
                value
            };

            ListItem::new(Line::from(vec![
                Span::styled(format!("  {:<18}", field.label), label_style),
                Span::styled(display_value, value_style),
            ]))
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.main_cursor));

    let list = List::new(items)
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▸ ");

    f.render_stateful_widget(list, chunks[1], &mut state);

    // Help bar
    let help_text = match &app.mode {
        Mode::Main => " ↑↓ navigate  ⏎ edit  q save & exit",
        Mode::Selecting { .. } => " ↑↓ navigate  ⏎ select  esc back",
        Mode::Editing { .. } => " type to edit  ⏎ confirm  esc cancel",
        Mode::EditingNumber { .. } => " type number  ⏎ confirm  esc cancel",
    };
    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::DarkGray))
        .block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
    f.render_widget(help, chunks[2]);

    // Overlay for select/edit modes
    match &app.mode {
        Mode::Selecting { field_idx, cursor } => {
            draw_select_popup(f, app, *field_idx, *cursor);
        }
        Mode::Editing { buffer, field_idx } => {
            draw_text_popup(f, app.fields[*field_idx].label, buffer);
        }
        Mode::EditingNumber { buffer, field_idx } => {
            draw_text_popup(f, app.fields[*field_idx].label, buffer);
        }
        _ => {}
    }
}

fn draw_select_popup(f: &mut ratatui::Frame, app: &App, field_idx: usize, cursor: usize) {
    let FieldKind::Select { ref options, .. } = app.fields[field_idx].kind else {
        return;
    };

    let popup_height = (options.len() as u16 + 4).min(f.area().height - 4);
    let popup_width = 56.min(f.area().width - 4);
    let popup_area = centered_rect(popup_width, popup_height, f.area());

    f.render_widget(Clear, popup_area);

    let items: Vec<ListItem> = options
        .iter()
        .map(|(val, desc)| {
            ListItem::new(Line::from(vec![
                Span::styled(format!(" {:<28}", val), Style::default().fg(Color::White)),
                Span::styled(*desc, Style::default().fg(Color::DarkGray)),
            ]))
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(cursor));

    let list = List::new(items)
        .block(
            Block::default()
                .title(format!(" {} ", app.fields[field_idx].label))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Yellow)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▸ ");

    f.render_stateful_widget(list, popup_area, &mut state);
}

fn draw_text_popup(f: &mut ratatui::Frame, label: &str, buffer: &str) {
    let popup_width = 56.min(f.area().width - 4);
    let popup_area = centered_rect(popup_width, 5, f.area());

    f.render_widget(Clear, popup_area);

    let input = Paragraph::new(format!(" {}", buffer))
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .title(format!(" {} ", label))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        );

    f.render_widget(input, popup_area);

    // Show cursor position
    let x = popup_area.x + buffer.len() as u16 + 2;
    let y = popup_area.y + 1;
    if x < popup_area.right() {
        f.set_cursor_position((x, y));
    }
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}
