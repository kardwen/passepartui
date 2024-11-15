use crate::theme::Theme;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    widgets::{Paragraph, Widget},
};

#[derive(Debug, Default, Clone)]
pub struct StatusBar {
    text: String,
    theme: Theme,
}

impl StatusBar {
    pub fn new() -> Self {
        Self {
            text: "Ready".into(),
            theme: Theme::new(),
        }
    }

    pub fn display_message(&mut self, message: String) {
        self.text = message;
    }

    pub fn reset_message(&mut self) {
        self.text = "Ready".into();
    }
}

impl Widget for &mut StatusBar {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let theme = self.theme;
        Paragraph::new(&*self.text)
            .style(
                Style::default()
                    .bg(theme.status_bar_bg)
                    .fg(theme.status_bar_fg),
            )
            .render(area, buf);
    }
}
