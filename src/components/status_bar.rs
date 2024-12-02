use crate::theme::Theme;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Style, Stylize},
    text::Line,
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

    pub fn set_status(&mut self, message: String) {
        self.text = message;
    }

    pub fn reset_status(&mut self) {
        self.text = "Ready".into();
    }
}

impl Widget for &mut StatusBar {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let theme = self.theme;
        Paragraph::new(Line::from(&*self.text))
            .style(
                Style::default()
                    .bg(theme.status_bar_bg)
                    .fg(theme.status_bar_fg),
            )
            .render(area, buf);
        Paragraph::new(Line::from("α").right_aligned().fg(theme.menu_logo_fg))
            .style(
                Style::default()
                    .bg(theme.status_bar_bg)
                    .fg(theme.status_bar_fg),
            )
            .render(area, buf);
    }
}
