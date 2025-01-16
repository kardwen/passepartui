use ratatui::{
    buffer::Buffer,
    crossterm::event::{MouseButton, MouseEvent, MouseEventKind},
    layout::{Position, Rect},
    style::{palette::tailwind, Color, Style},
    text::Line,
    widgets::Widget,
};

use crate::{actions::Action, components::MouseSupport};

#[derive(Debug, Default, Clone)]
pub struct Button<'a> {
    label: Line<'a>,
    keyboard_label: Line<'a>,
    mode: Mode,
    theme: Theme,
    state: State,
    pub dimensions: (u16, u16),
    inner_area: Option<Rect>,
    mouse_action: Option<Action>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum State {
    #[default]
    Normal,
    Selected,
    Active,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Horizontal,
    Vertical,
    Padded,
}

#[derive(Debug, Default, Clone, Copy)]
struct Theme {
    background: Color,
    highlight: Color,
    shadow: Color,
}

impl<'a> Button<'a> {
    pub fn new<T: Into<Line<'a>>>(label: T) -> Self {
        let color = tailwind::BLUE;
        let theme = Theme {
            background: color.c800,
            highlight: color.c700,
            shadow: color.c900,
        };
        Button {
            label: label.into(),
            keyboard_label: Line::default(),
            mode: Mode::default(),
            theme,
            state: State::Normal,
            dimensions: (10, 3),
            inner_area: None,
            mouse_action: None,
        }
    }

    pub fn keyboard_label<T: Into<Line<'a>>>(mut self, keyboard_label: T) -> Self {
        self.label.push_span(" ");
        self.keyboard_label = keyboard_label.into();
        self
    }

    pub const fn dimensions(mut self, width: u16, height: u16) -> Self {
        self.dimensions = (width, height);
        self
    }

    pub const fn horizontal_accents(mut self) -> Self {
        self.mode = Mode::Horizontal;
        self
    }

    pub const fn vertical_accents(mut self) -> Self {
        self.mode = Mode::Vertical;
        self
    }

    pub const fn padded(mut self) -> Self {
        self.mode = Mode::Padded;
        self
    }

    pub fn action_on_click(mut self, action: Action) -> Self {
        self.mouse_action = Some(action);
        self
    }

    pub const fn theme(mut self, background: Color, highlight: Color, shadow: Color) -> Self {
        self.theme = Theme {
            background,
            highlight,
            shadow,
        };
        self
    }

    pub const fn state(mut self, state: State) -> Self {
        self.state = state;
        self
    }

    const fn colors(&self) -> (Color, Color, Color) {
        let theme = self.theme;
        match self.state {
            State::Normal => (theme.background, theme.shadow, theme.highlight),
            State::Selected => (theme.highlight, theme.shadow, theme.highlight),
            State::Active => (theme.background, theme.highlight, theme.shadow),
        }
    }

    pub fn inner_area(&self) -> Option<Rect> {
        self.inner_area
    }

    pub fn select(&mut self) {
        self.state = State::Selected;
    }

    pub fn reset(&mut self) {
        self.state = State::Normal;
    }

    pub fn activate(&mut self) {
        self.state = State::Active;
    }

    fn in_focus(&mut self, event: MouseEvent) -> Option<Action> {
        match event.kind {
            MouseEventKind::Moved => {
                self.select();
                None
            }
            MouseEventKind::Down(MouseButton::Left) => {
                self.activate();
                self.mouse_action.clone()
            }
            MouseEventKind::Up(MouseButton::Left) => {
                self.select();
                None
            }
            _ => None,
        }
    }

    fn out_of_focus(&mut self) -> Option<Action> {
        self.reset();
        None
    }
}

impl Widget for &mut Button<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let (background, shadow, highlight) = self.colors();

        let inner_area = match self.mode {
            Mode::Horizontal | Mode::Vertical => area,
            Mode::Padded if area.height > 2 => Rect {
                height: area.height.saturating_sub(2),
                y: area.y + 1,
                ..area
            },
            Mode::Padded if area.height > 1 => Rect {
                height: area.height.saturating_sub(1),
                y: area.y + 1,
                ..area
            },
            _ => area,
        };
        // Clickable area
        self.inner_area = Some(inner_area);

        // Set button background
        buf.set_style(
            Rect {
                y: inner_area.y,
                height: inner_area.height,
                ..inner_area
            },
            Style::new().bg(background),
        );

        let (first_symbol, second_symbol) = match self.mode {
            Mode::Horizontal => ("▔", "▁"),
            Mode::Vertical => ("▏", "▕"),
            Mode::Padded => ("▁", "▔"),
        };

        if self.mode == Mode::Vertical {
            for y in area.y..(area.y + area.height) {
                buf.set_string(area.x, y, first_symbol, Style::new().fg(highlight));
                buf.set_string(
                    area.x + area.width.saturating_sub(1),
                    y,
                    second_symbol,
                    Style::new().fg(highlight),
                );
            }
        } else {
            // Render top line if there's enough space
            if area.height > 2 {
                buf.set_string(
                    area.x,
                    area.y,
                    first_symbol.repeat(area.width as usize),
                    Style::new().fg(highlight),
                );
            }
            // Render bottom line if there's enough space
            if area.height > 1 {
                buf.set_string(
                    area.x,
                    area.y + area.height - 1,
                    second_symbol.repeat(area.width as usize),
                    Style::new().fg(shadow),
                );
            }
        }

        // Center label and keyboard_label
        let combined_width = self.label.width() + self.keyboard_label.width();
        buf.set_line(
            area.x + (area.width.saturating_sub(combined_width as u16)) / 2,
            area.y + (area.height.saturating_sub(1)) / 2,
            &self.label,
            area.width,
        );
        buf.set_line(
            area.x
                + (area.width.saturating_sub(combined_width as u16)) / 2
                + self.label.width() as u16,
            area.y + (area.height.saturating_sub(1)) / 2,
            &self.keyboard_label,
            area.width,
        );
    }
}

impl MouseSupport for Button<'_> {
    fn handle_mouse_event(&mut self, event: MouseEvent) -> Option<Action> {
        let position = Position::new(event.column, event.row);
        match self.get_area() {
            Some(_area) if _area.contains(position) => self.in_focus(event),
            _ => self.out_of_focus(),
        }
    }

    fn get_area(&self) -> Option<Rect> {
        self.inner_area
    }
}
