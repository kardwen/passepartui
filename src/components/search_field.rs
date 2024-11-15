use ratatui::{
    buffer::Buffer,
    crossterm::event::{MouseButton, MouseEvent, MouseEventKind},
    layout::{Alignment, Position, Rect},
    style::{Style, Stylize},
    symbols,
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph, Widget},
};

use crate::{
    actions::{Action, NavigationAction},
    components::MouseSupport,
    theme::Theme,
};

#[derive(Debug, Default, Clone)]
pub struct SearchField {
    area: Option<Rect>,
    characters: Vec<char>,
    cursor_position: usize,
    suspended: bool,
    theme: Theme,
}

impl SearchField {
    pub fn new() -> Self {
        let theme = Theme::new();
        SearchField {
            area: None,
            characters: Vec::new(),
            cursor_position: 0,
            suspended: false,
            theme,
        }
    }

    pub fn insert(&mut self, character: char) {
        self.characters.insert(self.cursor_position, character);
        self.cursor_position += 1;
    }

    /// Return true if a letter was removed
    pub fn remove_left(&mut self) -> bool {
        if self.cursor_position > 0 {
            let _ = self
                .characters
                .remove(self.cursor_position.saturating_sub(1));
            self.cursor_position = self.cursor_position.saturating_sub(1);
            return true;
        }
        false
    }

    /// Return true if a letter was removed
    pub fn remove_right(&mut self) -> bool {
        if self.cursor_position < self.characters.len() {
            let _ = self.characters.remove(self.cursor_position);
            return true;
        }
        false
    }

    pub fn move_left(&mut self) {
        self.cursor_position = self.cursor_position.saturating_sub(1);
    }

    pub fn move_right(&mut self) {
        self.cursor_position = self.characters.len().min(self.cursor_position + 1);
    }

    pub fn move_to_start(&mut self) {
        self.cursor_position = 0;
    }

    pub fn move_to_end(&mut self) {
        self.cursor_position = self.characters.len();
    }

    pub fn reset(&mut self) {
        self.characters = Vec::new();
        self.cursor_position = 0;
        self.suspended = false;
    }

    pub fn suspend(&mut self) {
        self.suspended = true;
    }

    pub fn resume(&mut self) {
        self.suspended = false;
    }

    pub fn is_empty(&mut self) -> bool {
        self.characters.len() == 0
    }

    pub fn get_content(&self) -> String {
        String::from_iter(&self.characters)
    }

    fn in_focus(&mut self, event: MouseEvent) -> Option<Action> {
        match event.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                Some(Action::Navigation(NavigationAction::Search))
            }
            _ => None,
        }
    }

    fn out_of_focus(&mut self, event: MouseEvent) -> Option<Action> {
        match event.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if !self.suspended {
                    return Some(Action::Navigation(NavigationAction::Leave));
                }
                None
            }
            _ => None,
        }
    }
}

impl Widget for &mut SearchField {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.area = Some(area);
        let theme = self.theme;

        let block = Block::bordered()
            .title(Line::from("Search").fg(theme.standard_fg).left_aligned())
            .bg(theme.search_bg)
            .border_set(symbols::border::ROUNDED)
            .border_style(Style::new().fg(theme.search_border));
        let content_area = block.inner(area);
        Clear.render(area, buf);

        block.render(area, buf);

        let content = if self.cursor_position < self.characters.len() {
            // Underline char at cursor position
            let left: String = self.characters[..self.cursor_position].iter().collect();
            let middle = self.characters[self.cursor_position].to_string();
            let right: String = self.characters[self.cursor_position + 1..].iter().collect();

            if self.suspended {
                Line::from(vec![
                    " ⧸ ".into(),
                    Span::from(left),
                    Span::from(middle).underlined(),
                    Span::from(right),
                ])
                .dim()
            } else {
                Line::from(vec![
                    " ⧸ ".into(),
                    Span::from(left),
                    Span::from(middle).underlined().slow_blink(),
                    Span::from(right),
                ])
            }
        } else if self.suspended {
            Line::from(vec![
                " ⧸ ".into(),
                Span::from(self.get_content()).dim(),
                "_".into(),
            ])
            .dim()
        } else {
            Line::from(vec![
                " ⧸ ".into(),
                Span::from(self.get_content()),
                "_".slow_blink(),
            ])
        };

        Paragraph::new(content)
            .style(Style::new().fg(theme.standard_fg))
            .alignment(Alignment::Left)
            .render(content_area, buf);
    }
}

impl MouseSupport for SearchField {
    fn handle_mouse_event(&mut self, event: MouseEvent) -> Option<Action> {
        let position = Position::new(event.column, event.row);
        match self.get_area() {
            Some(_area) if _area.contains(position) => self.in_focus(event),
            _ => self.out_of_focus(event),
        }
    }

    fn get_area(&self) -> Option<Rect> {
        self.area
    }
}
