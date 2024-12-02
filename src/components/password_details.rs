use ratatui::{
    buffer::Buffer,
    crossterm::event::MouseEvent,
    layout::{Alignment, Constraint, Direction, Flex, Layout, Rect},
    style::{Style, Stylize},
    symbols,
    text::Line,
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};

use crate::{
    actions::{Action, NavigationAction, PasswordAction},
    components::{Button, MouseSupport},
    theme::Theme,
};

mod details_field;
use details_field::DetailsField;

#[derive(Debug, Default)]
pub struct PasswordDetails<'a> {
    pub show_secrets: bool,
    pub pass_id: Option<String>,
    pub line_count: Option<usize>,
    pub password: Option<String>,
    pub one_time_password: Option<String>,
    pub login: Option<String>,
    pass_id_field: DetailsField<'a>,
    lines_field: DetailsField<'a>,
    password_field: DetailsField<'a>,
    otp_field: DetailsField<'a>,
    login_field: DetailsField<'a>,
    theme: Theme,
    area: Option<Rect>,
}

impl PasswordDetails<'_> {
    pub fn new() -> Self {
        let theme = Theme::new();
        let pass_id_field = DetailsField::new(Line::from(vec![
            "Password file"
                .underlined()
                .italic()
                .bold()
                .fg(theme.details_field_fg),
            " 🗐".fg(theme.details_field_fg),
        ]))
        .button(
            Button::new("Copy".fg(theme.button_label))
                .keyboard_label("(c)".fg(theme.button_keyboard_label))
                .dimensions(10, 3)
                .padded()
                .action_on_click(Action::Password(PasswordAction::CopyPassId)),
        );
        let lines_field = DetailsField::new(Line::from(vec![
            "Number of lines"
                .underlined()
                .italic()
                .bold()
                .fg(theme.details_field_fg),
            " 🗟".fg(theme.details_field_fg),
        ]))
        .button(
            Button::new("Show file".fg(theme.button_label))
                .keyboard_label("(i)".fg(theme.button_keyboard_label))
                .dimensions(15, 3)
                .padded()
                .action_on_click(Action::Navigation(NavigationAction::File)),
        );
        let password_field = DetailsField::new(Line::from(vec![
            "Password"
                .underlined()
                .italic()
                .bold()
                .fg(theme.details_field_fg),
            " 🗝".fg(theme.details_field_fg),
        ]))
        .placeholder("********")
        .button(
            Button::new("Copy".fg(theme.button_label))
                .keyboard_label("(y)".fg(theme.button_keyboard_label))
                .dimensions(10, 3)
                .padded()
                .action_on_click(Action::Password(PasswordAction::CopyPassword)),
        );
        let otp_field = DetailsField::new(Line::from(vec![
            "One-time password (OTP)"
                .underlined()
                .italic()
                .bold()
                .fg(theme.details_field_fg),
            " 🕰".fg(theme.details_field_fg),
        ]))
        .placeholder("******")
        .button(
            Button::new("Copy".fg(theme.button_label))
                .keyboard_label("(x)".fg(theme.button_keyboard_label))
                .dimensions(10, 3)
                .padded()
                .action_on_click(Action::Password(PasswordAction::CopyOneTimePassword)),
        )
        .button(
            Button::new("Refresh".fg(theme.button_label))
                .keyboard_label("(r)".fg(theme.button_keyboard_label))
                .dimensions(13, 3)
                .padded()
                .action_on_click(Action::Password(PasswordAction::FetchOneTimePassword)),
        );
        let login_field = DetailsField::new(Line::from(vec![
            "Login"
                .underlined()
                .italic()
                .bold()
                .fg(theme.details_field_fg),
            " 🨂".fg(theme.details_field_fg),
        ]))
        .button(
            Button::new("Copy".fg(theme.button_label))
                .keyboard_label("(v)".fg(theme.button_keyboard_label))
                .dimensions(10, 3)
                .padded()
                .action_on_click(Action::Password(PasswordAction::CopyLogin)),
        );
        Self {
            show_secrets: false,
            pass_id: None,
            line_count: None,
            password: None,
            one_time_password: None,
            login: None,
            pass_id_field,
            lines_field,
            password_field,
            otp_field,
            login_field,
            theme,
            area: None,
        }
    }

    // Does not reset pass id
    pub fn clear_secrets(&mut self) {
        self.show_secrets = false;
        self.line_count = None;
        self.password = None;
        self.one_time_password = None;
        self.login = None;
    }

    pub fn reset(&mut self) {
        self.show_secrets = false;
        self.pass_id = None;
        self.line_count = None;
        self.password = None;
        self.one_time_password = None;
        self.login = None;
    }
}

impl Widget for &mut PasswordDetails<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.area = Some(area);
        if area.height < 4 {
            return;
        }

        let block = Block::new()
            .borders(Borders::TOP)
            .border_set(symbols::border::FULL)
            .border_style(Style::default().fg(self.theme.details_border))
            .bg(self.theme.standard_bg);

        // Top spacing of 1
        let mut content_area = block.inner(area);
        if content_area.height > 5 {
            content_area = Rect {
                y: content_area.y + 1,
                height: content_area.height.saturating_sub(1),
                ..content_area
            }
        };
        block.render(area, buf);

        let [left_area, right_area] = Layout::default()
            .direction(Direction::Horizontal)
            .horizontal_margin(1)
            .spacing(2)
            .constraints(Constraint::from_mins([1, 1]))
            .areas(content_area);

        let left_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![4; 3])
            .split(left_area);

        // Password file field
        if let Some(pass_id) = &self.pass_id {
            let field_area = left_layout[0];
            self.pass_id_field.set_content(pass_id);
            self.pass_id_field.render(field_area, buf);
        }

        // Number of lines field
        if let Some(number) = &self.line_count {
            if self.show_secrets {
                let field_area = left_layout[1];
                self.lines_field.set_content(&number.to_string());
                self.lines_field.render(field_area, buf);
            }
        }

        // Hint
        let hint = if self.show_secrets {
            "(←) Hide secrets  (→) Refresh"
        } else {
            "(←) View list     (→) Secrets"
        };
        Paragraph::new(vec![Line::default(), Line::from(hint.to_string())])
            .style(Style::new().fg(self.theme.details_hint_fg))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .render(left_layout[2], buf);

        // Count how many fields will be rendered
        let mut visible_fields = 1;
        if self.one_time_password.is_some() {
            visible_fields += 1;
        }
        if self.login.is_some() {
            visible_fields += 1;
        }
        let constraints = vec![4; visible_fields];

        let right_areas = Layout::vertical(Constraint::from_lengths(constraints))
            .flex(Flex::Start)
            .split(right_area);
        let mut right_areas = right_areas.iter();

        // Password field
        if self.pass_id.is_some() {
            let field_area = right_areas.next().expect("counted before");
            if !self.show_secrets {
                self.password_field.reset_content()
            } else if let Some(password) = &self.password {
                self.password_field.set_content(password);
            } else {
                self.password_field.reset_content()
            }
            self.password_field.render(*field_area, buf);
        }

        // One-time password field
        if let Some(ref otp) = self.one_time_password {
            if self.show_secrets {
                let field_area = right_areas.next().expect("counted before");
                self.otp_field.set_content(otp);
                self.otp_field.render(*field_area, buf);
            }
        }

        // Login field
        if let Some(ref login) = self.login {
            if self.show_secrets {
                let field_area = right_areas.next().expect("counted before");
                self.login_field.set_content(login);
                self.login_field.render(*field_area, buf);
            }
        }
    }
}

impl<'a> MouseSupport for PasswordDetails<'a> {
    fn handle_mouse_event(&mut self, event: MouseEvent) -> Option<Action> {
        let fields = [
            &mut self.pass_id_field,
            &mut self.lines_field,
            &mut self.otp_field,
            &mut self.password_field,
            &mut self.login_field,
        ];

        let mut action = None;
        for field in fields {
            if let Some(latest_action) = field.handle_mouse_event(event) {
                action = Some(latest_action);
            }
        }
        action
    }

    fn get_area(&self) -> Option<Rect> {
        self.area
    }
}
