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
    color_eyre::install()?;

    let runtime = tokio::runtime::Runtime::new()?;
    let (tx, rx) = mpsc::channel();

    app::spawn_input_thread(tx.clone());

    let csv_path = PathBuf::from("people.csv");
    let mut app = App::new(csv_path, runtime.handle().clone(), tx)?;

    let mut terminal = ratatui::init();
    // Without this, a paste is delivered as a flood of synthetic keystrokes
    // — if the clipboard text contains 'q' anywhere, that would quit the
    // app mid-paste. With it, crossterm hands pasted text back as one
    // `Event::Paste(String)` instead (see `app::AppEvent::Paste`).
    let _ = execute!(stdout(), EnableBracketedPaste);
    let result = app.run(&mut terminal, &rx);
    let _ = execute!(stdout(), DisableBracketedPaste);
    ratatui::restore();
    result
}
