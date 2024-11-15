use ratatui::{
    buffer::Buffer,
    crossterm::event::MouseEvent,
    layout::{Alignment, Position, Rect},
    style::Stylize,
    text::{Line, Text},
    widgets::{Paragraph, Widget},
};

use crate::{
    actions::Action,
    components::{Button, MouseSupport},
    theme::Theme,
};

#[derive(Debug, Default, Clone)]
pub struct DetailsField<'a> {
    title: Line<'a>,
    content: Option<String>,
    placeholder: String,
    buttons: Vec<Button<'a>>,
    area: Option<Rect>,
    theme: Theme,
}

impl<'a> DetailsField<'a> {
    pub fn new<T: Into<Line<'a>>>(title: T) -> Self {
        DetailsField {
            title: title.into(),
            content: None,
            placeholder: String::default(),
            buttons: Vec::new(),
            area: None,
            theme: Theme::new(),
        }
    }

    pub fn button(mut self, button: Button<'a>) -> Self {
        self.buttons.push(button);
        self
    }

    pub fn placeholder(mut self, placeholder: &str) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn set_content(&mut self, content: &str) {
        self.content = Some(content.into());
    }

    pub fn reset_content(&mut self) {
        self.content = None;
    }

    fn in_focus(&mut self, event: MouseEvent) -> Option<Action> {
        let mut latest_action = None;
        for button in &mut self.buttons {
            if let Some(action) = button.handle_mouse_event(event) {
                latest_action = Some(action);
            }
        }
        latest_action
    }

    fn out_of_focus(&mut self) -> Option<Action> {
        for button in &mut self.buttons {
            button.reset();
        }
        None
    }
}

impl<'a> Widget for &mut DetailsField<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.area = Some(area);
        if area.height < 2 {
            return;
        }

        let theme = self.theme;

        // Draw buttons
        let mut right_offset = 0;
        for button in &mut self.buttons {
            let (width, height) = button.dimensions;
            right_offset += width;
            let button_area = Rect {
                x: (area.x + area.width).saturating_sub(right_offset),
                y: area.y + 1,
                width,
                height,
            };
            right_offset += 1; // spacing
            button.render(button_area, buf);
        }

        // Cut content string if too long
        let max_content_length = area.width.saturating_sub(right_offset);
        let content = self.content.clone().unwrap_or(self.placeholder.clone());
        let content = if content.len() > max_content_length as usize {
            let mut truncated = content
                .chars()
                .take(max_content_length.saturating_sub(1) as usize)
                .collect::<String>();
            truncated.push('…');
            truncated
        } else {
            content
        };

        Paragraph::new(Text::from(vec![
            self.title.clone(),
            Line::default(),
            content.bg(theme.standard_bg).fg(theme.standard_fg).into(),
        ]))
        .alignment(Alignment::Left)
        .render(area, buf);
    }
}

impl<'a> MouseSupport for DetailsField<'a> {
    fn handle_mouse_event(&mut self, event: MouseEvent) -> Option<Action> {
        let position = Position::new(event.column, event.row);
        match self.get_area() {
            Some(_area) if _area.contains(position) => self.in_focus(event),
            _ => self.out_of_focus(),
        }
    }

    fn get_area(&self) -> Option<Rect> {
        self.area
    }
}
