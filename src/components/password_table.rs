use passepartout::PasswordInfo;
use ratatui::{
    buffer::Buffer,
    crossterm::event::{MouseButton, MouseEvent, MouseEventKind},
    layout::{Constraint, Layout, Position, Rect},
    style::{Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{
        Cell, HighlightSpacing, Row, Scrollbar, ScrollbarOrientation, ScrollbarState,
        StatefulWidget, Table, TableState, Widget,
    },
};

use crate::{
    actions::{Action, NavigationAction},
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
    scrollbar_state: ScrollbarState,
    area: Option<Rect>,
    mouse_content_area: Option<Rect>,
    mouse_track_area: Option<Rect>,
}

impl<'a> PasswordTable<'a> {
    pub fn new(passwords: &[&PasswordInfo]) -> Self {
        let theme = Theme::new();
        let rows = Self::build_rows(passwords, &theme);
        let length = rows.len();
        let table = Self::build_table(rows, &theme);
        let scrollbar_state = ScrollbarState::new(length);
        Self {
            theme,
            table,
            length,
            table_state: TableState::new(),
            highlight_pattern: None,
            scrollbar_state,
            area: None,
            mouse_content_area: None,
            mouse_track_area: None,
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
                    let pass_id = info.id.clone();
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
                    .style(Style::default().fg(self.theme.table_row_fg).bg(bg_color))
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
                Row::new(vec![info.id.clone(), info.last_modified()])
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
        let header = ["Password file", "Last modified (UTC)"]
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
}

impl Widget for &mut PasswordTable<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.area = Some(area);
        let theme = self.theme;

        let [table_area, right_area] =
            Layout::horizontal([Constraint::Min(1), Constraint::Length(1)]).areas(area);
        let [above_track_area, track_area] =
            Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).areas(right_area);
        buf.set_style(above_track_area, Style::new().bg(theme.table_header_bg));
        // Draw handle event when no row is displayed
        buf.set_style(track_area, Style::new().bg(theme.standard_fg));

        // Calculate areas for mouse interaction
        let mouse_content_area = Rect {
            x: area.x,
            y: area.y + 1,
            width: area.width.saturating_sub(8),
            height: area.height.saturating_sub(1),
        };
        let mouse_track_area = Rect {
            x: area.width.saturating_sub(8),
            width: 8,
            ..mouse_content_area
        };
        self.mouse_content_area = Some(mouse_content_area);
        self.mouse_track_area = Some(mouse_track_area);

        StatefulWidget::render(&self.table, table_area, buf, &mut self.table_state);

        Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .track_style(
                Style::new()
                    .fg(theme.table_track_fg)
                    .bg(theme.table_track_bg),
            )
            .thumb_style(Style::new().fg(theme.standard_fg).bg(theme.standard_bg))
            .begin_symbol(None)
            .end_symbol(None)
            .render(track_area, buf, &mut self.scrollbar_state);
    }
}

impl MouseSupport for PasswordTable<'_> {
    fn handle_mouse_event(&mut self, event: MouseEvent) -> Option<Action> {
        let position = Position::new(event.column, event.row);

        // Mouse position on password table contents
        if let Some(area) = self.mouse_content_area {
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
        if let Some(area) = self.mouse_track_area {
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
