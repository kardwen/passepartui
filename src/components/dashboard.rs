use anyhow::Result;
use futures::{
    self,
    channel::oneshot,
    executor::{block_on, ThreadPool},
};
use passepartout::{PasswordInfo, PasswordStore};
use ratatui::{
    buffer::Buffer,
    crossterm::event::MouseEvent,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    widgets::Widget,
};
use std::sync::mpsc::Sender;

use crate::{
    actions::{Action, NavigationAction, PasswordAction, SearchAction},
    app::{self, MainState, OverlayState, SearchState},
    components::{
        Component, FilePopup, HelpPopup, Menu, MouseSupport, PasswordDetails, PasswordTable,
        SearchField, StatusBar,
    },
    event::PasswordEvent,
};

#[derive(Default)]
struct LastOperation {
    pass_id: String,
    class: String,
    completion_receiver: Option<oneshot::Receiver<u8>>,
}

impl LastOperation {
    /// Determines if a new operation is allowed and then updates itself and
    /// returns a sender if permitted.
    ///
    /// An operation is allowed when:
    /// - The password ID is different from the last operation
    /// - The operation is from a different class than the last operation
    /// - The last operation has completed
    pub fn allows(&mut self, pass_id: &str, class: &str) -> Option<oneshot::Sender<u8>> {
        if pass_id != self.pass_id || class != self.class {
            self.update(pass_id, class)
        } else if let Some(ref mut receiver) = self.completion_receiver {
            match receiver.try_recv() {
                Ok(None) => None,
                Ok(Some(_)) | Err(oneshot::Canceled) => self.update(pass_id, class),
            }
        } else {
            None
        }
    }

    /// Returns a new sender that can be used to send a completion signal.
    fn update(&mut self, pass_id: &str, class: &str) -> Option<oneshot::Sender<u8>> {
        self.pass_id = pass_id.to_string();
        self.class = class.to_string();
        let (sender, receiver) = oneshot::channel::<u8>();
        self.completion_receiver = Some(receiver);
        Some(sender)
    }
}

pub struct Dashboard<'a> {
    tty_pinentry: bool,
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
    pub app_state: app::State,
    render_details: bool,
    pool: ThreadPool,
    last_op: LastOperation,
    event_tx: Sender<PasswordEvent>,
}

impl Dashboard<'_> {
    pub fn new(tty_pinentry: bool, event_tx: Sender<PasswordEvent>) -> Self {
        let store = PasswordStore::new();
        let password_refs: Vec<&PasswordInfo> = store.passwords.iter().collect();
        let password_subset = (0..store.passwords.len()).collect();
        let search_field = SearchField::new();
        let help_popup = HelpPopup::new();
        let file_popup = FilePopup::new();
        let pool = ThreadPool::builder()
            .pool_size(2)
            .create()
            .expect("this should work");
        let mut dashboard = Self {
            tty_pinentry,
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
            app_state: app::State::default(),
            render_details: true,
            pool,
            last_op: LastOperation::default(),
            event_tx,
        };
        dashboard.select_entry(0);
        dashboard
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
                let pass_id = info.id.clone();
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
            .filter(|(_, info)| info.id.to_lowercase().contains(&pattern.to_lowercase()))
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

    fn update_pass_details(&mut self, pass_id: String, message: String) -> Option<Action> {
        match self.get_selected_info() {
            Some(info) if pass_id == info.id => (),
            _ => return None,
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

        // let remainder = lines.fold(String::default(), |a, b| a + b);
        // if !remainder.is_empty() {}

        self.password_details.line_count = Some(count);

        if has_otp {
            self.password_details.one_time_password = Some("*".repeat(6));
            Some(Action::Password(PasswordAction::FetchOtp))
        } else {
            None
        }
    }

    fn show_pass_secrets(&mut self) {
        self.password_details.show_secrets = true;
    }

    fn hide_secrets(&mut self) {
        self.password_details.clear_secrets();
        self.file_popup.reset_content();
    }
}

impl Component for Dashboard<'_> {
    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        let action = match action {
            Action::Password(action) => match action {
                PasswordAction::CopyPassId => {
                    if let Some(info) = self.get_selected_info() {
                        match passepartout::copy_id(info.id.clone()) {
                            Ok(()) => {
                                let message = "Password file ID copied to clipboard".to_string();
                                Some(Action::SetStatus(message))
                            }
                            Err(passepartout::Error::Clipboard(e)) => {
                                let message = format!("✗ Clipboard error: {e:?}");
                                Some(Action::SetStatus(message))
                            }
                            Err(_) => None,
                        }
                    } else {
                        None
                    }
                }
                PasswordAction::CopyPassword => {
                    if let Some(info) = self.get_selected_info() {
                        let pass_id = info.id.clone();
                        if let Some(completion_beacon) =
                            self.last_op.allows(&pass_id, "copy_password")
                        {
                            let file_path = self.store.store_dir.join(format!("{}.gpg", pass_id));
                            let event_tx = self.event_tx.clone();

                            let future = async move {
                                let event = match passepartout::copy_password(&file_path) {
                                    Ok(_) => {
                                        let status_message =
                                            "Password copied to clipboard, clears after 45 seconds"
                                                .to_string();
                                        PasswordEvent::Status(Ok(Some(status_message)))
                                    }
                                    Err(e) => PasswordEvent::Status(Err(e)),
                                };
                                event_tx.send(event).expect("receiver deallocated");
                                let _ = completion_beacon.send(1);
                            };

                            if self.tty_pinentry {
                                block_on(future);
                                Some(Action::Redraw)
                            } else {
                                self.pool.spawn_ok(future);
                                let status_message = "⧗ Copying password...".to_string();
                                Some(Action::SetStatus(status_message))
                            }
                        } else {
                            None
                        }
                    } else {
                        let status_message = "No entry selected".to_string();
                        Some(Action::SetStatus(status_message))
                    }
                }
                PasswordAction::CopyLogin => {
                    if let Some(info) = self.get_selected_info() {
                        let pass_id = info.id.clone();
                        if let Some(completion_beacon) =
                            self.last_op.allows(&pass_id, "copy_password")
                        {
                            let file_path = self.store.store_dir.join(format!("{}.gpg", pass_id));
                            let event_tx = self.event_tx.clone();

                            let future = async move {
                                let event = match passepartout::copy_login(&file_path) {
                                    Ok(_) => {
                                        let status_message =
                                            "Login copied to clipboard, clears after 45 seconds"
                                                .to_string();
                                        PasswordEvent::Status(Ok(Some(status_message)))
                                    }
                                    Err(e) => PasswordEvent::Status(Err(e)),
                                };
                                event_tx.send(event).expect("receiver deallocated");
                                let _ = completion_beacon.send(1);
                            };

                            if self.tty_pinentry {
                                block_on(future);
                                Some(Action::Redraw)
                            } else {
                                self.pool.spawn_ok(future);
                                let status_message = "⧗ Copying login...".to_string();
                                Some(Action::SetStatus(status_message))
                            }
                        } else {
                            None
                        }
                    } else {
                        let status_message = "No entry selected".to_string();
                        Some(Action::SetStatus(status_message))
                    }
                }
                PasswordAction::CopyOtp => {
                    if let Some(info) = self.get_selected_info() {
                        let pass_id = info.id.clone();
                        if let Some(completion_beacon) =
                            self.last_op.allows(&pass_id, "copy_password")
                        {
                            let file_path = self.store.store_dir.join(format!("{}.gpg", pass_id));
                            let event_tx = self.event_tx.clone();

                            let future = async move {
                                let event = match passepartout::copy_otp(&file_path) {
                                    Ok(_) => {
                                        let status_message =
                                        "One-time password copied to clipboard, clears after 45 seconds"
                                            .to_string();
                                        PasswordEvent::Status(Ok(Some(status_message)))
                                    }
                                    Err(e) => PasswordEvent::Status(Err(e)),
                                };
                                event_tx.send(event).expect("receiver deallocated");
                                let _ = completion_beacon.send(1);
                            };

                            if self.tty_pinentry {
                                block_on(future);
                                Some(Action::Redraw)
                            } else {
                                self.pool.spawn_ok(future);
                                let status_message = "⧗ Copying one-time password...".to_string();
                                Some(Action::SetStatus(status_message))
                            }
                        } else {
                            None
                        }
                    } else {
                        let status_message = "No entry selected".to_string();
                        Some(Action::SetStatus(status_message))
                    }
                }
                PasswordAction::Fetch => {
                    if let Some(info) = self.get_selected_info() {
                        let pass_id = info.id.clone();
                        if let Some(completion_beacon) =
                            self.last_op.allows(&pass_id, "decrypt_password_file")
                        {
                            let file_path = self.store.store_dir.join(format!("{}.gpg", pass_id));
                            let event_tx = self.event_tx.clone();

                            let future = async move {
                                let event = match passepartout::decrypt_password_file(&file_path) {
                                    Ok(file_contents) => PasswordEvent::PasswordFile {
                                        pass_id,
                                        file_contents,
                                    },
                                    Err(e) => PasswordEvent::Status(Err(e)),
                                };
                                event_tx.send(event).expect("receiver deallocated");
                                let _ = completion_beacon.send(1);
                            };

                            if self.tty_pinentry {
                                block_on(future);
                                Some(Action::Redraw)
                            } else {
                                self.pool.spawn_ok(future);
                                let status_message = "⧗ Fetching password entry...".to_string();
                                Some(Action::SetStatus(status_message))
                            }
                        } else {
                            None
                        }
                    } else {
                        let status_message = "No entry selected".to_string();
                        Some(Action::SetStatus(status_message))
                    }
                }
                PasswordAction::FetchOtp => {
                    if let Some(info) = self.get_selected_info() {
                        let pass_id = info.id.clone();
                        if let Some(completion_beacon) =
                            self.last_op.allows(&pass_id, "copy_password")
                        {
                            let file_path = self.store.store_dir.join(format!("{}.gpg", pass_id));
                            let event_tx = self.event_tx.clone();

                            let future = async move {
                                let event = match passepartout::generate_otp(&file_path) {
                                    Ok(otp) => PasswordEvent::OneTimePassword { pass_id, otp },
                                    Err(e) => PasswordEvent::Status(Err(e)),
                                };
                                event_tx.send(event).expect("receiver deallocated");
                                let _ = completion_beacon.send(1);
                            };

                            if self.tty_pinentry {
                                block_on(future);
                                Some(Action::Redraw)
                            } else {
                                self.pool.spawn_ok(future);
                                let status_message = "⧗ Fetching one-time password...".to_string();
                                Some(Action::SetStatus(status_message))
                            }
                        } else {
                            None
                        }
                    } else {
                        let status_message = "No entry selected".to_string();
                        Some(Action::SetStatus(status_message))
                    }
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
                        app::State {
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
                        app::State {
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
                        app::State {
                            main: MainState::Secrets,
                            search: SearchState::Inactive | SearchState::Suspended,
                            overlay: OverlayState::Inactive,
                        } => Some(Action::Navigation(NavigationAction::Preview)),
                        app::State {
                            main: MainState::Preview,
                            search: SearchState::Inactive | SearchState::Suspended,
                            overlay: OverlayState::Inactive,
                        } => {
                            self.app_state.main = MainState::Table;
                            None
                        }
                        app::State {
                            main: _,
                            search: _,
                            overlay: OverlayState::Help,
                        } => {
                            self.app_state.overlay = OverlayState::Inactive;
                            None
                        }
                        app::State {
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
                self.status_bar.reset_status();
                self.update_pass_details(pass_id, file_contents)
            }
            Action::DisplayOneTimePassword { pass_id, otp } => {
                self.status_bar.reset_status();
                match self.get_selected_info() {
                    Some(info) if pass_id == info.id => {
                        self.password_details.one_time_password = Some(otp);
                        None
                    }
                    _ => None,
                }
            }
            _ => None,
        };
        Ok(action)
    }
}

impl Widget for &mut Dashboard<'_> {
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

impl MouseSupport for Dashboard<'_> {
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
