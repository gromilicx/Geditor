mod ai;
mod app;
mod config;
mod document;
mod highlight;
mod markdown;
mod search;
mod shell;
mod tree;
mod ui;
use anyhow::{Context, Result};
use crossterm::{
    event::{
        self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
        Event, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{
    env, fs,
    io::{IsTerminal, stdout},
    path::PathBuf,
    time::Duration,
};
fn restore() {
    let _ = disable_raw_mode();
    let _ = execute!(
        stdout(),
        PopKeyboardEnhancementFlags,
        DisableBracketedPaste,
        DisableMouseCapture,
        LeaveAlternateScreen,
        crossterm::cursor::Show
    );
}
fn main() -> Result<()> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.iter().any(|x| x == "--help" || x == "-h") {
        println!(
            "GEditor {} — free Rust terminal editor\nAuthors: Miodrag Gromilic and OpenAI\n\nUsage: ge [FILE|DIRECTORY]\n       geditor [FILE|DIRECTORY]\n\nCtrl+Tab: next panel (fallback: Ctrl+K, Ctrl+F)\nCtrl+E: menu  Ctrl+P: files  Ctrl+F: find  Ctrl+G: search project\nCtrl+K, Ctrl+O: settings  Ctrl+K, Ctrl+H: help\nCtrl+K Ctrl+W: Markdown preview | Ctrl+K Ctrl+L: full preview\nCtrl+Q: quit (twice to discard unsaved changes)\n\nFont size is controlled by your terminal.\nSettings: {}\nEnvironment: OPENAI_API_KEY, GEDITOR_CONFIG_DIR",
            env!("CARGO_PKG_VERSION"),
            config::path().display()
        );
        return Ok(());
    }
    if args.iter().any(|x| x == "--version" || x == "-V") {
        println!("GEditor {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    anyhow::ensure!(args.len() <= 1, "Usage: ge [FILE|DIRECTORY]");
    anyhow::ensure!(
        stdout().is_terminal(),
        "GEditor needs an interactive terminal. Run: ge ."
    );
    anyhow::ensure!(
        env::var("TERM").as_deref() != Ok("dumb"),
        "This terminal cannot run an interactive editor. Use a VT-compatible terminal."
    );
    let path = PathBuf::from(args.first().cloned().unwrap_or_else(|| ".".into()));
    let (root, file) = if path.is_dir() {
        (fs::canonicalize(&path)?, None)
    } else {
        let absolute = if path.is_absolute() {
            path
        } else {
            env::current_dir()?.join(path)
        };
        let parent = absolute.parent().context("File has no parent directory")?;
        let root = fs::canonicalize(parent)
            .with_context(|| format!("Cannot open {}", parent.display()))?;
        (
            root.clone(),
            Some(root.join(absolute.file_name().context("Invalid filename")?)),
        )
    };
    let settings_path = config::path();
    let loaded = config::Settings::load(&settings_path);
    let (settings, load_error) = match loaded {
        Ok(s) => (s, None),
        Err(e) => (config::Settings::default(), Some(e.to_string())),
    };
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore();
        hook(info)
    }));
    enable_raw_mode()?;
    if let Err(e) = execute!(
        stdout(),
        EnterAlternateScreen,
        EnableMouseCapture,
        EnableBracketedPaste,
        PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
    ) {
        restore();
        return Err(e.into());
    }
    let result = (|| -> Result<()> {
        let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
        terminal.clear()?;
        let mut app = app::App::new(root, settings, settings_path);
        if let Some(file) = file {
            app.open_file(file);
        }
        if let Some(e) = load_error {
            app.status = format!("Settings not loaded: {e}. Original settings were preserved.");
        }
        while app.running {
            app.tick();
            app.highlighter
                .apply(&mut app.docs[app.active], &app.settings);
            terminal.draw(|f| ui::draw(f, &mut app))?;
            if event::poll(Duration::from_millis(100))? {
                for _ in 0..128 {
                    match event::read()? {
                        Event::Key(k) => app.handle_key(k),
                        Event::Mouse(m) => app.handle_mouse(m),
                        Event::Paste(text) => app.paste(&text),
                        _ => {}
                    }
                    if !app.running || !event::poll(Duration::ZERO)? {
                        break;
                    }
                }
            }
        }
        Ok(())
    })();
    restore();
    result
}
