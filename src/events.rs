// use ratatui::crossterm::event::Event as TerminalEvent;

// #[derive(Debug, Clone, PartialEq)]
// pub enum Event {
//     Terminal(TerminalEvent),
//     Channel(ChannelEvent),
// }

#[derive(Debug, Clone, PartialEq)]
pub enum ChannelEvent {
    Status(String),
    ResetStatus,
    PasswordInfo {
        pass_id: String,
        file_contents: String,
    },
    OneTimePassword {
        pass_id: String,
        one_time_password: String,
    },
}
