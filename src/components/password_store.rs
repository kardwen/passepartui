use anyhow::Result;
use arboard::Clipboard;
use ratatui::{
    buffer::Buffer,
    crossterm::event::MouseEvent,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    widgets::Widget,
};
use std::{
    collections::HashMap,
    env,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc::Sender,
    thread::JoinHandle,
};

use crate::{
    actions::{Action, NavigationAction, PasswordAction, SearchAction},
    app_state::{AppState, MainState, OverlayState, SearchState},
    components::{
        Component, FilePopup, HelpPopup, Menu, MouseSupport, PasswordDetails, PasswordTable,
        SearchField, StatusBar,
    },
    events::ChannelEvent,
    utils::run_once,
};

mod password_info;

pub use password_info::PasswordInfo;

pub struct PasswordStore<'a> {
    area: Option<Rect>,
    passwords: Vec<PasswordInfo>,
    password_subset: Vec<usize>,
    menu: Menu<'a>,
    password_table: PasswordTable<'a>,
    password_details: PasswordDetails<'a>,
    search_field: SearchField,
    help_popup: HelpPopup<'a>,
    file_popup: FilePopup<'a>,
    status_bar: StatusBar,
    event_tx: Sender<ChannelEvent>,
    ops_map: HashMap<&'a str, (JoinHandle<()>, String)>,
    clipboard: Clipboard,
    pub app_state: AppState,
    render_details: bool,
}

impl<'a> PasswordStore<'a> {
    pub fn new(event_tx: Sender<ChannelEvent>) -> Self {
        let dir = Self::get_store_dir();
        let mut passwords = Self::get_password_infos(&dir);
        passwords.sort_by_key(|element| element.pass_id.clone());
        let password_refs: Vec<&PasswordInfo> = passwords.iter().collect();
        let password_subset = (0..passwords.len()).collect();
        let search_field = SearchField::new();
        let help_popup = HelpPopup::new();
        let file_popup = FilePopup::new();
        let mut store = Self {
            area: None,
            password_table: PasswordTable::new(&password_refs),
            password_details: PasswordDetails::new(),
            passwords,
            password_subset,
            menu: Menu::new(),
            search_field,
            help_popup,
            file_popup,
            status_bar: StatusBar::new(),
            event_tx,
            ops_map: HashMap::new(),
            clipboard: Clipboard::new().unwrap(),
            app_state: AppState::default(),
            render_details: true,
        };
        store.select_entry(0);
        store
    }

    pub fn get_store_dir() -> PathBuf {
        let home = dirs::home_dir().expect("could not determine home directory");
        if let Some(store_path) = env::var_os("PASSWORD_STORE_DIR") {
            let path = PathBuf::from(store_path);
            if path.is_absolute() {
                return path;
            } else if let Ok(relative_to_home) = path
                .strip_prefix("$HOME")
                .or_else(|_| path.strip_prefix("~"))
            {
                return home.join(relative_to_home);
            };
        }
        home.join(".password-store")
    }

    fn get_password_infos(store_dir: &Path) -> Vec<PasswordInfo> {
        Self::read_store_dir(store_dir)
            .unwrap_or_default()
            .iter()
            .filter_map(|path| {
                let relative_path = path.strip_prefix(store_dir).expect("prefix does exist");
                match path.metadata() {
                    Ok(metadata) => Some(PasswordInfo::new(relative_path, metadata)),
                    Err(_) => None,
                }
            })
            .collect()
    }

    fn read_store_dir(dir: &Path) -> Result<Vec<PathBuf>> {
        let mut result = Vec::new();

        fn visit_dir(dir: &Path, result: &mut Vec<PathBuf>) -> Result<()> {
            for entry in fs::read_dir(dir)? {
                let path = entry?.path();
                if path.is_dir() {
                    visit_dir(&path, result)?;
                } else if path.is_file() && path.extension().is_some_and(|ext| ext == "gpg") {
                    result.push(path);
                }
            }
            Ok(())
        }

        visit_dir(dir, &mut result)?;
        Ok(result)
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
                self.status_bar.reset_message();
                self.file_popup.reset_content();
                self.password_details.reset();
                self.password_details.pass_id = Some(pass_id);
            }
            None => {
                self.status_bar.reset_message();
                self.file_popup.reset_content();
                self.password_details.reset();
            }
        }
    }

    pub fn get_selected_info(&self) -> Option<&PasswordInfo> {
        if self.password_subset.len() > 0 {
            return match self.password_table.selected() {
                Some(index) => self.passwords.get(self.password_subset[index]),
                None => None,
            };
        }
        None
    }

    fn copy_pass_id(&mut self) {
        if let Some(info) = self.get_selected_info() {
            let pass_id = info.pass_id();
            match self.clipboard.set_text(pass_id) {
                Ok(()) => {
                    let status_text = "Password file identifier copied to clipboard".into();
                    self.status_bar.display_message(status_text);
                }
                Err(e) => {
                    let status_text = format!("Failed to copy password file identifier: {e:?}");
                    self.status_bar.display_message(status_text);
                }
            }
        } else {
            let status_text = String::from("No entry selected");
            self.status_bar.display_message(status_text);
        }
    }

    fn copy_password(&mut self) {
        if let Some(info) = self.get_selected_info() {
            let tx = self.event_tx.clone();
            let pass_id = info.pass_id();

            fn pass_fn(pass_id: String, tx: Sender<ChannelEvent>) {
                let status = Command::new("pass")
                    .arg(OsStr::new(&pass_id))
                    .arg("--clip")
                    .stderr(Stdio::null())
                    .stdout(Stdio::null())
                    .status()
                    .expect("failed to execute process");
                let message = if status.success() {
                    "Password copied to clipboard, clears after 45 seconds".to_string()
                } else {
                    format!("(pass) {status}")
                };
                let status_event = ChannelEvent::Status(message);
                tx.send(status_event).unwrap();
            }

            if run_once(
                &mut self.ops_map,
                "pass_copy_password",
                pass_id.clone(),
                move || pass_fn(pass_id, tx),
            ) {
                let status_text = String::from("⧗ (pass) Copying password...");
                self.status_bar.display_message(status_text);
            }
        } else {
            let status_text = "No entry selected".to_string();
            self.status_bar.display_message(status_text);
        }
    }

    fn copy_login(&mut self) {
        if let Some(info) = self.get_selected_info() {
            let tx = self.event_tx.clone();
            let pass_id = info.pass_id();

            fn pass_fn(pass_id: String, tx: Sender<ChannelEvent>) {
                let status = Command::new("pass")
                    .arg(OsStr::new(&pass_id))
                    .arg("--clip=2")
                    .stderr(Stdio::null())
                    .stdout(Stdio::null())
                    .status()
                    .expect("failed to execute process");
                let message = if status.success() {
                    "Login copied to clipboard, clears after 45 seconds".to_string()
                } else {
                    format!("✗ (pass) {status}")
                };
                let status_event = ChannelEvent::Status(message);
                tx.send(status_event).unwrap();
            }

            if run_once(
                &mut self.ops_map,
                "pass_copy_login",
                pass_id.clone(),
                move || pass_fn(pass_id, tx),
            ) {
                let status_text = String::from("⧗ (pass) Copying login...");
                self.status_bar.display_message(status_text);
            }
        } else {
            let status_text = "No entry selected".to_string();
            self.status_bar.display_message(status_text);
        }
    }

    fn copy_one_time_password(&mut self) {
        if let Some(info) = self.get_selected_info() {
            let tx = self.event_tx.clone();
            let pass_id = info.pass_id();

            fn pass_fn(pass_id: String, tx: Sender<ChannelEvent>) {
                let status = Command::new("pass")
                    .arg("otp")
                    .arg("code")
                    .arg(OsStr::new(&pass_id))
                    .arg("--clip")
                    .stderr(Stdio::null())
                    .stdout(Stdio::null())
                    .status()
                    .expect("failed to execute process");
                let message = if status.success() {
                    "One-time password copied to clipboard".to_string()
                } else {
                    format!("✗ (pass) {status}")
                };
                let status_event = ChannelEvent::Status(message);
                tx.send(status_event).unwrap();
            }

            if run_once(
                &mut self.ops_map,
                "pass_otp_copy",
                pass_id.clone(),
                move || pass_fn(pass_id, tx),
            ) {
                let status_text = String::from("⧗ (pass) Copying one-time password...");
                self.status_bar.display_message(status_text);
            }
        } else {
            let status_text = "No entry selected".to_string();
            self.status_bar.display_message(status_text);
        }
    }

    fn fetch_one_time_password(&mut self) {
        if let Some(info) = self.get_selected_info() {
            let tx = self.event_tx.clone();
            let pass_id = info.pass_id();

            fn pass_fn(pass_id: String, tx: Sender<ChannelEvent>) {
                let output = Command::new("pass")
                    .arg("otp")
                    .arg("code")
                    .arg(OsStr::new(&pass_id))
                    .output()
                    .expect("failed to execute process");
                if output.status.success() {
                    let one_time_password = String::from_utf8_lossy(&output.stdout).to_string();
                    tx.send(ChannelEvent::OneTimePassword {
                        pass_id,
                        one_time_password,
                    })
                    .unwrap();
                    tx.send(ChannelEvent::ResetStatus).unwrap();
                } else {
                    let message = format!("✗ (pass) {}", String::from_utf8_lossy(&output.stderr));
                    tx.send(ChannelEvent::Status(message)).unwrap();
                }
            }

            if run_once(
                &mut self.ops_map,
                "pass_otp_fetch",
                pass_id.clone(),
                move || pass_fn(pass_id, tx),
            ) {
                let status_text = String::from("⧗ (pass) Fetching one-time password...");
                self.status_bar.display_message(status_text);
            }
        } else {
            let status_text = "No entry selected".to_string();
            self.status_bar.display_message(status_text);
        }
    }

    fn fetch_pass_details(&mut self) {
        if let Some(info) = self.get_selected_info() {
            let tx = self.event_tx.clone();
            let pass_id = info.pass_id();

            fn pass_fn(pass_id: String, tx: Sender<ChannelEvent>) {
                let output = Command::new("pass")
                    .arg(OsStr::new(&pass_id))
                    .output()
                    .expect("failed to execute process");
                if output.status.success() {
                    let file_contents = String::from_utf8_lossy(&output.stdout).to_string();
                    tx.send(ChannelEvent::PasswordInfo {
                        pass_id,
                        file_contents,
                    })
                    .unwrap();
                    tx.send(ChannelEvent::ResetStatus).unwrap();
                } else {
                    let message = format!("✗ (pass) {}", String::from_utf8_lossy(&output.stderr));
                    tx.send(ChannelEvent::Status(message)).unwrap();
                };
            }

            if run_once(&mut self.ops_map, "pass_show", pass_id.clone(), move || {
                pass_fn(pass_id, tx)
            }) {
                let status_text = String::from("⧗ (pass) Fetching password info...");
                self.status_bar.display_message(status_text);
            }
        } else {
            let status_text = "No entry selected".to_string();
            self.status_bar.display_message(status_text);
        }
    }

    fn filter_passwords(&mut self) {
        let pattern = self.search_field.get_content();

        // Vector of indices for matching passwords
        self.password_subset = self
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
            .filter_map(|&idx| self.passwords.get(idx))
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
        let password_refs: Vec<&PasswordInfo> = self.passwords.iter().collect();
        self.password_subset = (0..self.passwords.len()).collect();
        self.password_table.highlight_pattern = None;
        self.password_table.update_passwords(&password_refs);
        self.select_entry(index);
    }

    fn update_pass_details(&mut self, pass_id: String, message: String) {
        if !self.password_details.show_secrets {
            return;
        }

        if pass_id != self.get_selected_info().unwrap().pass_id {
            return;
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
        while let Some(line) = next_line {
            // One-time password (OTP)
            if line.starts_with("otpauth://") {
                self.password_details.one_time_password = Some("*".repeat(6));
                self.fetch_one_time_password();
            }
            count += 1;
            next_line = lines.next();
        }

        // let remainder = lines.fold(String::default(), |a, b| a + b);
        // if !remainder.is_empty() {}

        self.password_details.number_of_lines = Some(count);
    }

    fn show_pass_secrets(&mut self) {
        self.password_details.show_secrets = true;
    }

    fn hide_secrets(&mut self) {
        self.password_details.clear_secrets();
        self.file_popup.reset_content();
    }
}

impl<'a> Component for PasswordStore<'a> {
    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        let action = match action {
            Action::Password(action) => match action {
                PasswordAction::Fetch => {
                    self.fetch_pass_details();
                    None
                }
                PasswordAction::CopyPassword => {
                    self.copy_password();
                    None
                }
                PasswordAction::CopyOneTimePassword => {
                    self.copy_one_time_password();
                    None
                }
                PasswordAction::FetchOneTimePassword => {
                    self.fetch_one_time_password();
                    None
                }
                PasswordAction::CopyPassId => {
                    self.copy_pass_id();
                    None
                }
                PasswordAction::CopyLogin => {
                    self.copy_login();
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
                        self.show_pass_secrets();
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
            Action::DisplayStatus(message) => {
                self.status_bar.display_message(message);
                None
            }
            Action::ResetStatus => {
                self.status_bar.reset_message();
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
            } if pass_id == self.get_selected_info().unwrap().pass_id => {
                self.password_details.one_time_password = Some(one_time_password);
                None
            }
            _ => None,
        };
        Ok(action)
    }
}

impl<'a> Widget for &mut PasswordStore<'a> {
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
                let search_width = 40.min(area.width);
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

impl<'a> MouseSupport for PasswordStore<'a> {
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

#[cfg(test)]
mod tests {
    // use super::*;
    // use ratatui::style::{Style, Stylize};
    //
    // #[test]
    // fn render() {
    //     // TODO: Update
    //     let app = App::default();
    //     let mut buf = Buffer::empty(Rect::new(0, 0, 50, 4));
    //
    //     app.render(buf.area, &mut buf);
    //
    //     let mut expected = Buffer::with_lines(vec![
    //         "┏━━━━━━━━━━━━━ Counter App Tutorial ━━━━━━━━━━━━━┓",
    //         "┃                    Value: 0                    ┃",
    //         "┃                                                ┃",
    //         "┗━ Decrement <Left> Increment <Right> Quit <Q> ━━┛",
    //     ]);
    //     let title_style = Style::new().bold();
    //     let counter_style = Style::new().yellow();
    //     let key_style = Style::new().blue().bold();
    //     expected.set_style(Rect::new(14, 0, 22, 1), title_style);
    //     expected.set_style(Rect::new(28, 1, 1, 1), counter_style);
    //     expected.set_style(Rect::new(13, 3, 6, 1), key_style);
    //     expected.set_style(Rect::new(30, 3, 7, 1), key_style);
    //     expected.set_style(Rect::new(43, 3, 4, 1), key_style);
    //
    //     assert_eq!(buf, expected);
    // }
    //
    // #[test]
    // fn map_key_event() -> Result<()> {
    //     let mut app = App::default();
    //     let action = app
    //         .map_key_event(KeyCode::Char('q').into())
    //         .expect("key not mapped to action");
    //     app.update(action);
    //
    //     assert!(app.exit);
    //
    //     Ok(())
    // }
}
