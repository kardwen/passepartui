use crate::{
    actions::{Action, NavigationAction},
    components::{Button, MouseSupport},
    theme::Theme,
};
use ratatui::{
    buffer::Buffer, crossterm::event::MouseEvent, layout::Rect, style::Stylize, widgets::Widget,
};

#[derive(Debug, Default, Clone)]
pub struct Menu<'a> {
    theme: Theme,
    area: Option<Rect>,
    search_button: Button<'a>,
    help_button: Button<'a>,
    quit_button: Button<'a>,
}

impl<'a> Menu<'a> {
    pub fn new() -> Self {
        let theme = Theme::new();
        let search_button = Button::new("Search".fg(theme.menu_button_label))
            .keyboard_label("(/)".fg(theme.menu_button_keyboard_label))
            .vertical_accents()
            .theme(
                theme.menu_button_background,
                theme.menu_button_highlight,
                theme.menu_button_shadow,
            )
            .action_on_click(Action::Navigation(NavigationAction::Search));
        let help_button = Button::new("Help".fg(theme.menu_button_label))
            .keyboard_label("(F1)".fg(theme.menu_button_keyboard_label))
            .vertical_accents()
            .theme(
                theme.menu_button_background,
                theme.menu_button_highlight,
                theme.menu_button_shadow,
            )
            .action_on_click(Action::Navigation(NavigationAction::Help));
        let quit_button = Button::new("Quit".fg(theme.menu_button_label))
            .keyboard_label("(q)".fg(theme.menu_button_keyboard_label))
            .vertical_accents()
            .theme(
                theme.menu_button_background,
                theme.menu_button_highlight,
                theme.menu_button_shadow,
            )
            .action_on_click(Action::Navigation(NavigationAction::Quit));
        Menu {
            theme,
            area: None,
            help_button,
            search_button,
            quit_button,
        }
    }
}

impl<'a> Widget for &mut Menu<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.area = Some(area);

        // Title bar and menu
        let title = "passepartui α  "
            .bold()
            .into_right_aligned_line()
            .fg(self.theme.menu_logo_fg)
            .bg(self.theme.menu_bg);
        title.render(area, buf);

        // Search button
        let button_area = Rect {
            x: 0,
            y: 0,
            width: 12,
            height: 1,
        };
        self.search_button.render(button_area, buf);
        // Help button
        let button_area = Rect {
            x: 12,
            y: 0,
            width: 11,
            height: 1,
        };
        self.help_button.render(button_area, buf);
        // Quit button
        let button_area = Rect {
            x: 23,
            y: 0,
            width: 10,
            height: 1,
        };
        self.quit_button.render(button_area, buf);
    }
}

impl<'a> MouseSupport for Menu<'a> {
    fn handle_mouse_event(&mut self, event: MouseEvent) -> Option<Action> {
        let buttons = vec![
            &mut self.search_button,
            &mut self.help_button,
            &mut self.quit_button,
        ];
        // TODO: Currently this only returns the latest actions
        // since buttons shouldn't overlap
        let mut action = None;
        for button in buttons {
            if let Some(latest_action) = button.handle_mouse_event(event) {
                action = Some(latest_action);
            }
        }
        action
    }

    fn get_area(&self) -> Option<Rect> {
        self.area
    }
}
