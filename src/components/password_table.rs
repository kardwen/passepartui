use ratatui::{
    buffer::Buffer,
    crossterm::event::{MouseButton, MouseEvent, MouseEventKind},
    layout::{Constraint, Position, Rect},
    style::{Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{
        Cell, HighlightSpacing, Row, Scrollbar, ScrollbarOrientation, ScrollbarState,
        StatefulWidget, Table, TableState, Widget,
    },
};

use crate::{
    actions::{Action, NavigationAction},
    components::password_store::PasswordInfo,
    components::MouseSupport,
    theme::Theme,
};

#[derive(Debug, Default)]
pub struct PasswordTable<'a> {
    theme: Theme,
    table: Table<'a>,
    length: usize,
    table_state: TableState,
    pub highlight_pattern: Option<String>,
    scrollbar: Scrollbar<'a>,
    scrollbar_state: ScrollbarState,
    area: Option<Rect>,
    content_area: Option<Rect>,
    scrollbar_area: Option<Rect>,
}

impl<'a> PasswordTable<'a> {
    pub fn new(passwords: &[&PasswordInfo]) -> Self {
        let theme = Theme::new();
        let rows = Self::build_rows(passwords, &theme);
        let length = rows.len();
        let table = Self::build_table(rows, &theme);
        let scrollbar_state = ScrollbarState::new(length);
        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None);
        Self {
            theme,
            table,
            length,
            table_state: TableState::new(),
            highlight_pattern: None,
            scrollbar,
            scrollbar_state,
            area: None,
            content_area: None,
            scrollbar_area: None,
        }
    }

    pub fn update_passwords(&mut self, passwords: &[&PasswordInfo]) {
        let rows = if let Some(pattern) = &self.highlight_pattern {
            passwords
                .iter()
                .enumerate()
                .map(|(i, info)| {
                    let bg_color = match i % 2 {
                        0 => self.theme.table_normal_row,
                        _ => self.theme.table_alt_row,
                    };
                    let pass_id = info.pass_id();
                    let pass_id_parts: Vec<_> = if !pattern.is_empty() {
                        let pass_id_lower = pass_id.to_lowercase();
                        let pattern_lower = pattern.to_lowercase();

                        if let Some(_first_idx) = pass_id_lower.find(&pattern_lower) {
                            let mut styled_parts = Vec::new();
                            let mut last_idx = 0;

                            pass_id_lower
                                .match_indices(&pattern_lower)
                                .for_each(|(idx, _)| {
                                    // Add non-matching part
                                    if idx > last_idx {
                                        styled_parts.push(Span::styled(
                                            pass_id[last_idx..idx].to_string(),
                                            Style::default().fg(self.theme.table_row_fg),
                                        ));
                                    }

                                    // Add matching part
                                    styled_parts.push(Span::styled(
                                        pass_id[idx..idx + pattern.len()].to_string(),
                                        Style::default()
                                            .fg(self.theme.table_row_fg)
                                            .bg(self.theme.table_pattern_highlight_bg)
                                            .add_modifier(Modifier::BOLD),
                                    ));

                                    last_idx = idx + pattern.len();
                                });

                            // Add remaining part
                            if last_idx < pass_id.len() {
                                styled_parts.push(Span::styled(
                                    pass_id[last_idx..].to_string(),
                                    Style::default().fg(self.theme.table_row_fg),
                                ));
                            }

                            styled_parts
                        } else {
                            vec![Span::styled(
                                pass_id,
                                Style::default().fg(self.theme.table_row_fg),
                            )]
                        }
                    } else {
                        vec![Span::styled(
                            pass_id,
                            Style::default().fg(self.theme.table_row_fg),
                        )]
                    };

                    Row::new(vec![
                        Cell::from(Line::from(pass_id_parts)),
                        Cell::from(info.last_modified()),
                    ])
                    .style(Style::default().bg(bg_color))
                })
                .collect()
        } else {
            Self::build_rows(passwords, &self.theme)
        };

        self.length = rows.len();
        self.table = Self::build_table(rows, &self.theme);
        self.table_state = TableState::new();
        self.scrollbar_state = ScrollbarState::new(self.length);
    }

    fn build_rows(passwords: &[&PasswordInfo], theme: &Theme) -> Vec<Row<'a>> {
        passwords
            .iter()
            .enumerate()
            .map(|(i, info)| {
                let color = match i % 2 {
                    0 => theme.table_normal_row,
                    _ => theme.table_alt_row,
                };
                Row::new(vec![info.pass_id(), info.last_modified()])
                    .style(Style::new().fg(theme.table_row_fg).bg(color))
            })
            .collect()
    }

    fn build_table(rows: Vec<Row<'a>>, theme: &Theme) -> Table<'a> {
        let header_style = Style::default()
            .fg(theme.table_header_fg)
            .bg(theme.table_header_bg);
        let selected_row_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(theme.table_selected_row_style_fg);
        let selected_col_style = Style::default().fg(theme.table_selected_column_style_fg);
        let selected_cell_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(theme.table_selected_cell_style_fg);
        let header = ["Password", "Last modified (UTC)"]
            .into_iter()
            .map(Cell::from)
            .collect::<Row>()
            .style(header_style)
            .height(1);
        let widths = [Constraint::Min(25), Constraint::Max(25)];
        Table::new(rows.clone(), widths)
            .column_spacing(1)
            .style(Style::new().white())
            .header(header)
            .row_highlight_style(Style::new().add_modifier(Modifier::REVERSED))
            .row_highlight_style(selected_row_style)
            .column_highlight_style(selected_col_style)
            .cell_highlight_style(selected_cell_style)
            .highlight_symbol(Text::from("│"))
            .bg(theme.table_buffer_bg)
            .highlight_spacing(HighlightSpacing::Always)
    }

    pub fn select(&mut self, index: usize) {
        self.table_state.select(Some(index));
        self.scrollbar_state = self.scrollbar_state.position(index);
    }

    pub fn selected(&self) -> Option<usize> {
        self.table_state.selected()
    }

    pub fn scrollbar_area(&self) -> Option<Rect> {
        self.scrollbar_area
    }
}

impl<'a> Widget for &mut PasswordTable<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.area = Some(area);

        // Exclude header (height: 1) and scrollbar_area plus tolerance from content_area
        let content_area = Rect {
            x: area.x,
            y: area.y + 1,
            width: area.width.saturating_sub(8),
            height: area.height.saturating_sub(1),
        };
        let scrollbar_area_width = 8;
        let scrollbar_area = Rect {
            x: area.width.saturating_sub(scrollbar_area_width),
            width: scrollbar_area_width,
            ..content_area
        };
        self.content_area = Some(content_area);
        self.scrollbar_area = Some(scrollbar_area);

        StatefulWidget::render(&self.table, area, buf, &mut self.table_state);

        self.scrollbar
            .clone()
            .render(scrollbar_area, buf, &mut self.scrollbar_state);
    }
}

impl<'a> MouseSupport for PasswordTable<'a> {
    fn handle_mouse_event(&mut self, event: MouseEvent) -> Option<Action> {
        let position = Position::new(event.column, event.row);

        // Mouse position on password table contents
        if let Some(area) = self.content_area {
            if area.contains(position) {
                return match event.kind {
                    MouseEventKind::Down(MouseButton::Left) => {
                        let line = position.y - area.y;
                        let i = self.table_state.offset() + line as usize;
                        Some(Action::Navigation(NavigationAction::SelectAndFetch(i)))
                    }
                    MouseEventKind::ScrollDown => Some(Action::Navigation(NavigationAction::Down)),
                    MouseEventKind::ScrollUp => Some(Action::Navigation(NavigationAction::Up)),
                    _ => None,
                };
            }
        }

        // Mouse position on the scrollbar
        if let Some(area) = self.scrollbar_area {
            if area.contains(position) {
                return match event.kind {
                    MouseEventKind::Down(MouseButton::Left)
                    | MouseEventKind::Drag(MouseButton::Left) => {
                        let line: u16 = position.y - area.y;
                        let ratio: f32 = line as f32 / (area.height - 1) as f32;
                        let i: usize = (ratio * self.length as f32) as usize;
                        Some(Action::Navigation(NavigationAction::Select(i)))
                    }
                    MouseEventKind::Up(MouseButton::Left) => None,
                    _ => None,
                };
            }
        }

        None
    }

    fn get_area(&self) -> Option<Rect> {
        self.area
    }
}
