use ratatui::{
    buffer::Buffer,
    crossterm::event::MouseEvent,
    layout::{Alignment, Constraint, Direction, Flex, Layout, Rect},
    style::{Style, Stylize},
    symbols,
    text::Line,
    widgets::{Block, Clear, Padding, Paragraph, Widget, Wrap},
};

use crate::{
    actions::{Action, NavigationAction},
    components::{Button, MouseSupport},
    theme::Theme,
};

#[derive(Debug, Default, Clone)]
pub struct FilePopup<'a> {
    area: Option<Rect>,
    theme: Theme,
    pass_id: Option<String>,
    content: Option<String>,
    close_button: Button<'a>,
}

impl<'a> FilePopup<'a> {
    pub fn new() -> Self {
        let theme = Theme::new();
        FilePopup {
            area: None,
            theme,
            pass_id: None,
            content: None,
            close_button: Button::new("Close".fg(theme.button_label))
                .keyboard_label("(Esc)".fg(theme.button_keyboard_label))
                .dimensions(13, 3)
                .padded()
                .action_on_click(Action::Navigation(NavigationAction::Back)),
        }
    }

    pub fn set_content(&mut self, pass_id: &str, content: &str) {
        self.pass_id = Some(pass_id.into());
        self.content = Some(content.into());
    }

    pub fn reset_content(&mut self) {
        self.pass_id = None;
        self.content = None;
    }
}

impl<'a> Widget for &mut FilePopup<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.area = Some(area);
        let theme = self.theme;

        let block = Block::bordered()
            .title(Line::from("File").fg(theme.standard_fg).centered())
            .padding(Padding::horizontal(1))
            .bg(theme.standard_bg)
            .border_set(symbols::border::ROUNDED)
            .border_style(Style::new().fg(theme.popup_border));
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Min(1),
                Constraint::Length(3),
            ])
            .split(block.inner(area));

        Clear.render(area, buf);
        block.render(area, buf);

        if let Some(pass_id) = self.pass_id.clone() {
            Paragraph::new(Line::from(vec![
                "Password file ID: ".fg(theme.debug),
                pass_id.into(),
            ]))
            .alignment(Alignment::Left)
            .style(Style::new().fg(theme.standard_fg))
            .render(layout[0], buf);
        }

        if let Some(content) = self.content.clone() {
            let lines: Vec<&str> = content.lines().collect();
            let content: Vec<Line> = lines
                .iter()
                .map(|line| Line::from(line.fg(theme.standard_fg)))
                .collect();

            let content_area = layout[1];
            let content_area = Rect {
                x: content_area.x + 2,
                width: content_area.width.saturating_sub(2),
                ..content_area
            };
            Paragraph::new(content)
                .style(Style::new().fg(theme.standard_fg))
                .alignment(Alignment::Left)
                .wrap(Wrap { trim: false })
                .render(content_area, buf);
        }

        let [button_area] = Layout::horizontal([Constraint::Length(13)])
            .flex(Flex::Center)
            .areas(layout[2]);
        self.close_button.render(button_area, buf);
    }
}

impl<'a> MouseSupport for FilePopup<'a> {
    fn handle_mouse_event(&mut self, event: MouseEvent) -> Option<Action> {
        self.close_button
            .handle_mouse_event(event)
            .or(Some(Action::NoOp))
    }

    fn get_area(&self) -> Option<Rect> {
        self.area
    }
}
