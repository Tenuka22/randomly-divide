mod app;
mod pages;
mod person;
mod random;
mod store;
mod ui;

use std::path::PathBuf;

use color_eyre::Result;

use crate::app::App;

fn main() -> Result<()> {
    color_eyre::install()?;
    let csv_path = PathBuf::from("people.csv");
    let mut app = App::new(csv_path)?;
    let mut terminal = ratatui::init();
    let result = app.run(&mut terminal);
    ratatui::restore();
    result
}
