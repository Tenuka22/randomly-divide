mod app;
mod avatar;
mod db;
mod pages;
mod person;
mod random;
mod store;
mod ui;

use std::io::stdout;
use std::path::PathBuf;
use std::sync::mpsc;

use color_eyre::Result;
use crossterm::event::{DisableBracketedPaste, EnableBracketedPaste};
use crossterm::execute;

use crate::app::App;

fn main() -> Result<()> {
    let runtime = tokio::runtime::Runtime::new()?;
    let (tx, rx) = mpsc::channel();

    app::spawn_input_thread(tx.clone());

    let csv_path = PathBuf::from("people.csv");
    let db_config = db::load_config();
    let mut app = App::new(csv_path, runtime.handle().clone(), tx, db_config)?;

    let mut terminal = ratatui::init();
    // `ratatui::init()` installs its own panic hook that unconditionally
    // restores the terminal on *any* panic, on *any* thread. That's wrong
    // for this app: the Turso sync task runs on a background tokio worker
    // thread, and a panic there (we hit exactly this once — a libsql
    // misconfiguration panicked instead of returning an `Err`) would yank
    // the terminal out of raw/alt-screen mode while the main loop was still
    // mid-session and kept calling `terminal.draw()` on it, producing a
    // garbled, overlapping screen. Replace it with a hook that only
    // restores when the *main* thread panics; a background-thread panic
    // still can't take the terminal down with it.
    install_panic_hook()?;
    let _ = execute!(stdout(), EnableBracketedPaste);
    let result = app.run(&mut terminal, &rx);
    let _ = execute!(stdout(), DisableBracketedPaste);
    ratatui::restore();
    result
}

fn install_panic_hook() -> Result<()> {
    let (panic_hook, eyre_hook) = color_eyre::config::HookBuilder::default().into_hooks();
    eyre_hook.install()?;

    let panic_hook = panic_hook.into_panic_hook();
    std::panic::set_hook(Box::new(move |info| {
        if std::thread::current().name() == Some("main") {
            let _ = ratatui::try_restore();
        }
        panic_hook(info);
    }));
    Ok(())
}
