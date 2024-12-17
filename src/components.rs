use anyhow::Result;
use ratatui::{crossterm::event::MouseEvent, layout::Rect};

mod button;
mod dashboard;
mod file_popup;
mod help_popup;
mod menu;
mod password_details;
mod password_table;
mod search_field;
mod status_bar;

use crate::actions::Action;
pub use button::Button;
pub use dashboard::Dashboard;
pub use file_popup::FilePopup;
pub use help_popup::HelpPopup;
pub use menu::Menu;
pub use password_details::PasswordDetails;
pub use password_table::PasswordTable;
pub use search_field::SearchField;
pub use status_bar::StatusBar;

pub trait Component {
    fn update(&mut self, action: Action) -> Result<Option<Action>>;
}

pub trait MouseSupport {
    fn handle_mouse_event(&mut self, event: MouseEvent) -> Option<Action>;

    // TODO: can I require that self.area exists directly?
    fn get_area(&self) -> Option<Rect>;
}
