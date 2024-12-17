use anyhow::Result;
use ratatui::crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
};
use std::io::stdout;

mod actions;
mod app;
mod components;
mod theme;

use app::App;

fn main() -> Result<()> {
    let mut terminal = ratatui::init();
    execute!(stdout(), EnableMouseCapture)?;
    terminal.clear()?;
    let result = App::new().run(&mut terminal);
    execute!(stdout(), DisableMouseCapture)?;
    ratatui::restore();
    result?;
    Ok(())
}
