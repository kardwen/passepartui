use anyhow::Result;
use ratatui::{
    buffer::Buffer,
    crossterm::event::MouseEvent,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    widgets::Widget,
};
use std::sync::mpsc::Sender;

use crate::{
    actions::{Action, NavigationAction, PasswordAction, SearchAction},
    app_state::{AppState, MainState, OverlayState, SearchState},
    components::{
        Component, FilePopup, HelpPopup, Menu, MouseSupport, PasswordDetails, PasswordTable,
        SearchField, StatusBar,
    },
};

use passepartout::{PasswordError, PasswordEvent, PasswordInfo, PasswordStore};

pub struct Dashboard<'a> {
    store: PasswordStore,
    area: Option<Rect>,
    password_subset: Vec<usize>,
    menu: Menu<'a>,
    password_table: PasswordTable<'a>,
    password_details: PasswordDetails<'a>,
    search_field: SearchField,
    help_popup: HelpPopup<'a>,
    file_popup: FilePopup<'a>,
    status_bar: StatusBar,
    pub app_state: AppState,
    render_details: bool,
}

impl<'a> Dashboard<'a> {
    pub fn new(event_tx: Sender<PasswordEvent>) -> Self {
        let store = PasswordStore::new(event_tx);
        let password_refs: Vec<&PasswordInfo> = store.passwords.iter().collect();
        let password_subset = (0..store.passwords.len()).collect();
        let search_field = SearchField::new();
        let help_popup = HelpPopup::new();
        let file_popup = FilePopup::new();
        let mut store = Self {
            area: None,
            password_table: PasswordTable::new(&password_refs),
            store,
            password_details: PasswordDetails::new(),
            password_subset,
            menu: Menu::new(),
            search_field,
            help_popup,
            file_popup,
            status_bar: StatusBar::new(),
            app_state: AppState::default(),
            render_details: true,
        };
        store.select_entry(0);
        store
    }

    pub fn next(&mut self, step: usize) {
        let i = match self.password_table.selected() {
            Some(i) => (i + step).min(self.password_subset.len() - 1),
            None => 0,
        };
        self.select_entry(i);
    }

    pub fn previous(&mut self, step: usize) {
        let i = match self.password_table.selected() {
            Some(i) => i.saturating_sub(step),
            None => 0,
        };
        self.select_entry(i);
    }

    pub fn top_row(&mut self) {
        let i = 0;
        self.select_entry(i);
    }

    pub fn bottom_row(&mut self) {
        let i = self.password_subset.len() - 1;
        self.select_entry(i);
    }

    fn select_entry(&mut self, index: usize) {
        let view_index = index.min(self.password_subset.len().saturating_sub(1));
        self.password_table.select(view_index);
        match self.get_selected_info() {
            Some(info) => {
                // Update view with infos for selected entry
                let pass_id = info.pass_id();
                if let Some(selected_pass_id) = &self.password_details.pass_id {
                    if *selected_pass_id == pass_id {
                        return;
                    }
                }
                self.status_bar.reset_status();
                self.file_popup.reset_content();
                self.password_details.reset();
                self.password_details.pass_id = Some(pass_id);
            }
            None => {
                self.status_bar.reset_status();
                self.file_popup.reset_content();
                self.password_details.reset();
            }
        }
    }

    pub fn get_selected_info(&self) -> Option<&PasswordInfo> {
        if !self.password_subset.is_empty() {
            return match self.password_table.selected() {
                Some(index) => self.store.passwords.get(self.password_subset[index]),
                None => None,
            };
        }
        None
    }

    fn filter_passwords(&mut self) {
        let pattern = self.search_field.get_content();

        // Vector of indices for matching passwords
        self.password_subset = self
            .store
            .passwords
            .iter()
            .enumerate()
            .filter(|(_, p)| p.pass_id.to_lowercase().contains(&pattern.to_lowercase()))
            .map(|(index, _)| index)
            .collect();

        // Reference vector for password table
        let filtered_passwords: Vec<&PasswordInfo> = self
            .password_subset
            .iter()
            .filter_map(|&idx| self.store.passwords.get(idx))
            .collect();

        self.password_table.highlight_pattern = Some(pattern);
        self.password_table.update_passwords(&filtered_passwords);

        // Select the first entry
        self.select_entry(0);
    }

    fn reset_password_filter(&mut self) {
        let index = if let Some(index) = self.password_table.selected() {
            self.password_subset[index]
        } else {
            0
        };
        let password_refs: Vec<&PasswordInfo> = self.store.passwords.iter().collect();
        self.password_subset = (0..self.store.passwords.len()).collect();
        self.password_table.highlight_pattern = None;
        self.password_table.update_passwords(&password_refs);
        self.select_entry(index);
    }

    fn update_pass_details(&mut self, pass_id: String, message: String) {
        match self.get_selected_info() {
            Some(info) if pass_id == info.pass_id => (),
            _ => return,
        }

        self.file_popup.set_content(&pass_id, &message.clone());
        let mut lines = message.lines();
        let mut count = 0;
        if let Some(password) = lines.next() {
            self.password_details.password = Some(password.to_string());
            count += 1;
        }
        if let Some(login) = lines.next() {
            self.password_details.login = Some(login.to_string());
            count += 1;
        }

        let mut next_line = lines.next();
        let mut has_otp = false;
        while let Some(line) = next_line {
            // One-time password (OTP)
            if line.starts_with("otpauth://") {
                has_otp = true;
            }
            count += 1;
            next_line = lines.next();
        }
        if has_otp {
            self.password_details.one_time_password = Some("*".repeat(6));
            self.store.fetch_otp(pass_id);
        }

        // let remainder = lines.fold(String::default(), |a, b| a + b);
        // if !remainder.is_empty() {}

        self.password_details.line_count = Some(count);
    }

    fn show_pass_secrets(&mut self) {
        self.password_details.show_secrets = true;
    }

    fn hide_secrets(&mut self) {
        self.password_details.clear_secrets();
        self.file_popup.reset_content();
    }
}

impl<'a> Component for Dashboard<'a> {
    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        let action = match action {
            Action::Password(action) => match action {
                PasswordAction::Fetch => {
                    if self.get_selected_info().is_some() {
                        self.status_bar
                            .set_status("⧗ (pass) Fetching password entry...".to_string());
                    }
                    if let Some(info) = self.get_selected_info() {
                        self.store.fetch_entry(info.pass_id.clone());
                    }
                    None
                }
                PasswordAction::CopyPassword => {
                    if self.get_selected_info().is_some() {
                        self.status_bar
                            .set_status("⧗ (pass) Copying password...".to_string());
                    }
                    if let Some(info) = self.get_selected_info() {
                        self.store.copy_password(info.pass_id.clone());
                    }
                    None
                }
                PasswordAction::CopyOneTimePassword => {
                    if self.get_selected_info().is_some() {
                        self.status_bar
                            .set_status("⧗ (pass) Copying one-time password...".to_string());
                    }
                    if let Some(info) = self.get_selected_info() {
                        self.store.copy_otp(info.pass_id.clone());
                    }
                    None
                }
                PasswordAction::FetchOneTimePassword => {
                    if self.get_selected_info().is_some() {
                        self.status_bar
                            .set_status("⧗ (pass) Fetching one-time password...".to_string());
                    }
                    if let Some(info) = self.get_selected_info() {
                        self.store.fetch_otp(info.pass_id.clone());
                    }
                    None
                }
                PasswordAction::CopyPassId => {
                    if let Some(info) = self.get_selected_info() {
                        match passepartout::copy_id(info.pass_id.clone()) {
                            Ok(()) => {
                                let message = "Password file ID copied to clipboard".to_string();
                                Some(Action::SetStatus(message))
                            }
                            Err(PasswordError::ClipboardError(e)) => {
                                let message = format!("✗ Failed to copy password file ID: {e:?}");
                                Some(Action::SetStatus(message))
                            }
                            Err(PasswordError::ClipboardUnavailable) => {
                                let message = String::from("✗ Clipboard not available");
                                Some(Action::SetStatus(message))
                            }
                            Err(_) => None,
                        }
                    } else {
                        None
                    }
                }
                PasswordAction::CopyLogin => {
                    if self.get_selected_info().is_some() {
                        self.status_bar
                            .set_status("⧗ (pass) Copying login...".to_string());
                    }
                    if let Some(info) = self.get_selected_info() {
                        self.store.copy_login(info.pass_id.clone());
                    }
                    None
                }
            },
            Action::Navigation(action) => {
                match action {
                    NavigationAction::Down => match self.app_state.main {
                        MainState::Secrets => {
                            self.next(1);
                            Some(Action::Navigation(NavigationAction::Preview))
                        }
                        _ => {
                            self.next(1);
                            None
                        }
                    },
                    NavigationAction::Up => match self.app_state.main {
                        MainState::Secrets => {
                            self.previous(1);
                            Some(Action::Navigation(NavigationAction::Preview))
                        }
                        _ => {
                            self.previous(1);
                            None
                        }
                    },
                    NavigationAction::PageDown => match self.app_state.main {
                        MainState::Secrets => {
                            self.next(10);
                            Some(Action::Navigation(NavigationAction::Preview))
                        }
                        _ => {
                            self.next(10);
                            None
                        }
                    },
                    NavigationAction::PageUp => match self.app_state.main {
                        MainState::Secrets => {
                            self.previous(10);
                            Some(Action::Navigation(NavigationAction::Preview))
                        }
                        _ => {
                            self.previous(10);
                            None
                        }
                    },
                    NavigationAction::Top => match self.app_state.main {
                        MainState::Secrets => {
                            self.top_row();
                            Some(Action::Navigation(NavigationAction::Preview))
                        }
                        _ => {
                            self.top_row();
                            None
                        }
                    },
                    NavigationAction::Bottom => match self.app_state.main {
                        MainState::Secrets => {
                            self.bottom_row();
                            Some(Action::Navigation(NavigationAction::Preview))
                        }
                        _ => {
                            self.bottom_row();
                            None
                        }
                    },
                    NavigationAction::Select(i) => match self.app_state.main {
                        MainState::Secrets => {
                            self.select_entry(i);
                            Some(Action::Navigation(NavigationAction::Preview))
                        }
                        _ => {
                            self.select_entry(i);
                            None
                        }
                    },
                    NavigationAction::SelectAndFetch(i) => {
                        self.app_state.main = MainState::Secrets;
                        self.show_pass_secrets();
                        self.select_entry(i);
                        Some(Action::Password(PasswordAction::Fetch))
                    }
                    NavigationAction::Preview => {
                        self.hide_secrets();
                        self.app_state.main = MainState::Preview;
                        None
                    }
                    NavigationAction::Secrets => {
                        self.app_state.main = MainState::Secrets;
                        self.show_pass_secrets();
                        Some(Action::Password(PasswordAction::Fetch))
                    }
                    // Open search popup
                    NavigationAction::Search => {
                        self.app_state.search = SearchState::Active;
                        self.search_field.resume();
                        None
                    }
                    // Open help popup
                    NavigationAction::Help => {
                        self.app_state.overlay = OverlayState::Help;
                        None
                    }
                    // Open file popup and fetch details
                    NavigationAction::File => {
                        self.app_state.overlay = OverlayState::File;
                        Some(Action::Password(PasswordAction::Fetch))
                    }
                    NavigationAction::Leave => match self.app_state {
                        AppState {
                            main: _,
                            search: SearchState::Active,
                            overlay: OverlayState::Inactive,
                        } => {
                            if self.search_field.is_empty() {
                                self.app_state.search = SearchState::Inactive;
                            } else {
                                self.search_field.suspend();
                                self.app_state.search = SearchState::Suspended;
                            }
                            None
                        }
                        AppState {
                            main: _,
                            search: SearchState::Suspended,
                            overlay: OverlayState::Inactive,
                        } => {
                            self.search_field.reset();
                            self.reset_password_filter();
                            self.app_state.search = SearchState::Inactive;
                            None
                        }
                        _ => None,
                    },
                    NavigationAction::Back => match self.app_state {
                        AppState {
                            main: MainState::Secrets,
                            search: SearchState::Inactive | SearchState::Suspended,
                            overlay: OverlayState::Inactive,
                        } => Some(Action::Navigation(NavigationAction::Preview)),
                        AppState {
                            main: MainState::Preview,
                            search: SearchState::Inactive | SearchState::Suspended,
                            overlay: OverlayState::Inactive,
                        } => {
                            self.app_state.main = MainState::Table;
                            None
                        }
                        AppState {
                            main: _,
                            search: _,
                            overlay: OverlayState::Help,
                        } => {
                            self.app_state.overlay = OverlayState::Inactive;
                            None
                        }
                        AppState {
                            main: _,
                            search: _,
                            overlay: OverlayState::File,
                        } => {
                            self.app_state.overlay = OverlayState::Inactive;
                            None
                        }
                        _ => None,
                    },
                    _ => None,
                }
            }
            Action::Search(action) => match action {
                SearchAction::Insert(character) => {
                    self.search_field.insert(character);
                    self.filter_passwords();
                    None
                }
                SearchAction::RemoveLeft => {
                    if self.search_field.remove_left() {
                        self.filter_passwords();
                    }
                    None
                }
                SearchAction::RemoveRight => {
                    if self.search_field.remove_right() {
                        self.filter_passwords();
                    }
                    None
                }
                SearchAction::MoveLeft => {
                    self.search_field.move_left();
                    None
                }
                SearchAction::MoveRight => {
                    self.search_field.move_right();
                    None
                }
                SearchAction::MoveToStart => {
                    self.search_field.move_to_start();
                    None
                }
                SearchAction::MoveToEnd => {
                    self.search_field.move_to_end();
                    None
                }
            },
            Action::SetStatus(message) => {
                self.status_bar.set_status(message);
                None
            }
            Action::ResetStatus => {
                self.status_bar.reset_status();
                None
            }
            Action::DisplaySecrets {
                pass_id,
                file_contents,
            } => {
                self.update_pass_details(pass_id, file_contents);
                None
            }
            Action::DisplayOneTimePassword {
                pass_id,
                one_time_password,
            } => match self.get_selected_info() {
                Some(info) if pass_id == info.pass_id => {
                    self.password_details.one_time_password = Some(one_time_password);
                    None
                }
                _ => None,
            },
            _ => None,
        };
        Ok(action)
    }
}

impl<'a> Widget for &mut Dashboard<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.area = Some(area);

        // Layout
        let layout = match self.app_state.main {
            MainState::Table => Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Min(1),
                    Constraint::Length(1),
                ])
                .split(area),
            MainState::Preview | MainState::Secrets => Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Min(1),
                    Constraint::Length(14),
                    Constraint::Length(1),
                ])
                .split(area),
        };

        // Menu
        self.menu.render(layout[0], buf);

        // Table
        self.password_table.render(layout[1], buf);

        // Details
        let mut status_bar_area = layout[2];
        if self.app_state.main != MainState::Table {
            if self.render_details {
                self.password_details.render(layout[2], buf);
            }
            status_bar_area = layout[3];
        }

        // Statusbar
        self.status_bar.render(status_bar_area, buf);

        // Search field
        match self.app_state.search {
            SearchState::Active | SearchState::Suspended => {
                let search_width = 35.min(area.width);
                let popup_area = Rect {
                    x: area.width.saturating_sub(search_width + 1),
                    y: 3.min(area.height),
                    width: search_width,
                    height: 3.min(area.height.saturating_sub(3)),
                };
                self.search_field.render(popup_area, buf);
            }
            SearchState::Inactive => (),
        }

        // Help popup
        if self.app_state.overlay == OverlayState::Help {
            let popup_area = area.inner(Margin::new(6, 3));
            self.help_popup.render(popup_area, buf);
        }

        // File contents popup
        if self.app_state.overlay == OverlayState::File {
            let popup_area = area.inner(Margin::new(8, 4));
            self.file_popup.render(popup_area, buf);
        }
    }
}

impl<'a> MouseSupport for Dashboard<'a> {
    fn handle_mouse_event(&mut self, event: MouseEvent) -> Option<Action> {
        // TODO: Currently this only returns the latest action
        // if components overlap, place them last
        // Should be refactored to account for current app state
        let mut action = None;
        if let Some(latest_action) = self.password_table.handle_mouse_event(event) {
            action = Some(latest_action);
        }
        match self.app_state.search {
            SearchState::Active | SearchState::Suspended => {
                if let Some(latest_action) = self.search_field.handle_mouse_event(event) {
                    action = Some(latest_action);
                }
            }
            SearchState::Inactive => (),
        }
        if let Some(latest_action) = self.password_details.handle_mouse_event(event) {
            action = Some(latest_action);
        }
        match self.app_state.overlay {
            OverlayState::File => {
                if let Some(latest_action) = self.file_popup.handle_mouse_event(event) {
                    action = Some(latest_action);
                }
            }
            OverlayState::Help => {
                if let Some(latest_action) = self.help_popup.handle_mouse_event(event) {
                    action = Some(latest_action);
                }
            }
            OverlayState::Inactive => (),
        }
        if let Some(latest_action) = self.menu.handle_mouse_event(event) {
            action = Some(latest_action);
        }
        action
    }

    fn get_area(&self) -> Option<Rect> {
        self.area
    }
}
