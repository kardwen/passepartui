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
pub struct HelpPopup<'a> {
    area: Option<Rect>,
    theme: Theme,
    close_button: Button<'a>,
}

impl<'a> HelpPopup<'a> {
    pub fn new() -> Self {
        let theme = Theme::new();
        HelpPopup {
            area: None,
            theme,
            close_button: Button::new("Close".fg(theme.button_label))
                .keyboard_label("(Esc)".fg(theme.button_keyboard_label))
                .dimensions(13, 3)
                .padded()
                .action_on_click(Action::Navigation(NavigationAction::Back)),
        }
    }
}

impl<'a> Widget for &mut HelpPopup<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.area = Some(area);
        let theme = self.theme;

        let block = Block::bordered()
            .title(Line::from("Help").fg(theme.standard_fg).centered())
            .padding(Padding {
                left: 1,
                right: 1,
                top: 1,
                bottom: 0,
            })
            .bg(theme.standard_bg)
            .border_set(symbols::border::ROUNDED)
            .border_style(Style::new().fg(theme.popup_border));
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(3)])
            .split(block.inner(area));
        Clear.render(area, buf);
        block.render(area, buf);

        let text = vec![
            Line::from("Navigation".fg(theme.debug).italic()),
            Line::default(),
            Line::from("(↓), (↑), (j), (k) Select list entry".fg(theme.standard_fg)),
            Line::from("(⇣), (⇡), (b), (f) Skip list entries".fg(theme.standard_fg)),
            Line::from("(⇱), (g) Select first entry in list".fg(theme.standard_fg)),
            Line::from("(⇲), (G) Select last entry in list".fg(theme.standard_fg)),
            Line::default(),
            Line::from("(←) (h) (→) (l) (↵) Switch between view modes".fg(theme.standard_fg)),
            Line::from("for password list, preview and secrets".fg(theme.standard_fg)),
            Line::default(),
            Line::from(
                "Keyboard shortcuts are mapped in all view modes."
                    .fg(theme.standard_fg)
                    .italic(),
            ),
            Line::default(),
            Line::from("Search".fg(theme.debug).italic()),
            Line::default(),
            Line::from("(Esc), (↵) Suspend search".fg(theme.standard_fg)),
            Line::from(
                "Pressing (Esc) a second time clears the search and resets the filter."
                    .fg(theme.standard_fg),
            ),
            Line::from("(↓) and (↑) work as usual to select a result.".fg(theme.standard_fg)),
        ];
        Paragraph::new(text)
            .style(Style::new().fg(theme.standard_fg))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .render(layout[0], buf);

        let [button_area] = Layout::horizontal([Constraint::Length(13)])
            .flex(Flex::Center)
            .areas(layout[1]);
        self.close_button.render(button_area, buf);
    }
}

impl<'a> MouseSupport for HelpPopup<'a> {
    fn handle_mouse_event(&mut self, event: MouseEvent) -> Option<Action> {
        self.close_button
            .handle_mouse_event(event)
            .or(Some(Action::NoOp))
    }

    fn get_area(&self) -> Option<Rect> {
        self.area
    }
}
