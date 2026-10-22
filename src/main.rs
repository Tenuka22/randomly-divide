mod app;
mod avatar;
mod db;
mod pages;
mod person;
mod random;
mod store;
mod ui;

use std::path::PathBuf;
use std::sync::mpsc;

use color_eyre::Result;

use crate::app::App;

fn main() -> Result<()> {
    color_eyre::install()?;

    let runtime = tokio::runtime::Runtime::new()?;
    let (tx, rx) = mpsc::channel();

    app::spawn_input_thread(tx.clone());

    let csv_path = PathBuf::from("people.csv");
    let mut app = App::new(csv_path, runtime.handle().clone(), tx)?;

    let mut terminal = ratatui::init();
    let result = app.run(&mut terminal, &rx);
    ratatui::restore();
    result
}
