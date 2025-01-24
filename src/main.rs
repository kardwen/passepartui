use anyhow::Result;
use ratatui::crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::SetTitle,
};
use std::{env, io::stdout};

mod actions;
mod app;
mod components;
mod event;
mod theme;

use app::App;

fn main() -> Result<()> {
    let tty_pinentry = env::args().any(|arg| arg == "--tty-pinentry");
    // See: https://github.com/kardwen/passepartui/issues/12#issuecomment-2612471345
    let set_title = env::args().any(|arg| arg == "--set-title");

    let mut terminal = ratatui::init();
    if set_title {
        execute!(stdout(), SetTitle("passepartui"))?;
    }
    execute!(stdout(), EnableMouseCapture)?;
    terminal.clear()?;
    let result = App::new(tty_pinentry).run(&mut terminal);
    execute!(stdout(), DisableMouseCapture)?;
    ratatui::restore();
    result?;
    Ok(())
}
