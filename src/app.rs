use anyhow::Result;
use ratatui::{
    crossterm::event::{self, Event as TerminalEvent, KeyCode, KeyEvent, KeyEventKind, MouseEvent},
    DefaultTerminal,
};
use std::{
    sync::mpsc::{self, Receiver},
    time::Duration,
};

mod state;

use crate::{
    actions::{Action, NavigationAction, PasswordAction, SearchAction},
    components::{Component, Dashboard, MouseSupport},
    event::PasswordEvent,
};
pub use state::{MainState, OverlayState, SearchState, State};

pub struct App<'a> {
    running: bool,
    complete_redraw: bool,
    tick_rate: Duration,
    event_rx: Receiver<PasswordEvent>,
    dashboard: Dashboard<'a>,
}

impl<'a> App<'a> {
    pub fn new(tty_pinentry: bool) -> Self {
        let (event_tx, event_rx) = mpsc::channel();
        Self {
            dashboard: Dashboard::new(tty_pinentry, event_tx),
            running: false,
            complete_redraw: false,
            tick_rate: Duration::from_millis(80),
            event_rx,
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        self.running = true;
        // Application loop
        while self.running {
            if self.complete_redraw {
                let _ = terminal.clear();
                self.complete_redraw = false;
            }
            terminal.draw(|frame| frame.render_widget(&mut self.dashboard, frame.area()))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn handle_events(&mut self) -> Result<()> {
        if event::poll(self.tick_rate)? {
            if let Ok(terminal_event) = event::read() {
                match terminal_event {
                    TerminalEvent::Key(event) if event.kind == KeyEventKind::Press => {
                        if let Some(action) = self.handle_key_event(event) {
                            self.dispatch_action(action)?;
                        }
                    }
                    TerminalEvent::Mouse(mouse_event) => {
                        if let Some(action) = self.handle_mouse_event(mouse_event) {
                            self.dispatch_action(action)?;
                        }
                    }
                    TerminalEvent::Resize(_, _) => (),
                    _ => (),
                }
            }
        }
        while let Ok(event) = self.event_rx.try_recv() {
            if let Some(action) = self.handle_channel_event(event) {
                self.dispatch_action(action)?;
            }
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) -> Option<Action> {
        match self.dashboard.app_state {
            State {
                main: MainState::Preview | MainState::Secrets,
                search: SearchState::Inactive | SearchState::Suspended,
                overlay: OverlayState::Inactive,
            } => match key_event.code {
                KeyCode::Char('j') | KeyCode::Down => {
                    Some(Action::Navigation(NavigationAction::Down))
                }
                KeyCode::Char('k') | KeyCode::Up => Some(Action::Navigation(NavigationAction::Up)),
                KeyCode::PageDown | KeyCode::Char('f') => {
                    Some(Action::Navigation(NavigationAction::PageDown))
                }
                KeyCode::PageUp | KeyCode::Char('b') => {
                    Some(Action::Navigation(NavigationAction::PageUp))
                }
                KeyCode::Char('g') | KeyCode::Home => {
                    Some(Action::Navigation(NavigationAction::Top))
                }
                KeyCode::Char('G') | KeyCode::End => {
                    Some(Action::Navigation(NavigationAction::Bottom))
                }
                KeyCode::Char('y') => Some(Action::Password(PasswordAction::CopyPassword)),
                KeyCode::Char('h') | KeyCode::Left => {
                    Some(Action::Navigation(NavigationAction::Back))
                }
                KeyCode::Char('l') | KeyCode::Right | KeyCode::Enter => {
                    Some(Action::Navigation(NavigationAction::Secrets))
                }
                KeyCode::Char('/') => Some(Action::Navigation(NavigationAction::Search)),
                KeyCode::F(1) => Some(Action::Navigation(NavigationAction::Help)),
                KeyCode::Char('i') => Some(Action::Navigation(NavigationAction::File)),
                KeyCode::Char('r') => Some(Action::Password(PasswordAction::FetchOtp)),
                KeyCode::Char('x') => Some(Action::Password(PasswordAction::CopyOtp)),
                KeyCode::Char('c') => Some(Action::Password(PasswordAction::CopyPassId)),
                KeyCode::Char('v') => Some(Action::Password(PasswordAction::CopyLogin)),
                KeyCode::Esc => Some(Action::Navigation(NavigationAction::Leave)),
                KeyCode::Char('q') | KeyCode::Char('Q') => {
                    Some(Action::Navigation(NavigationAction::Quit))
                }
                _ => None,
            },
            State {
                main: MainState::Table,
                search: SearchState::Inactive | SearchState::Suspended,
                overlay: OverlayState::Inactive,
            } => match key_event.code {
                KeyCode::Char('j') | KeyCode::Down => {
                    Some(Action::Navigation(NavigationAction::Down))
                }
                KeyCode::Char('k') | KeyCode::Up => Some(Action::Navigation(NavigationAction::Up)),
                KeyCode::PageDown | KeyCode::Char('f') => {
                    Some(Action::Navigation(NavigationAction::PageDown))
                }
                KeyCode::PageUp | KeyCode::Char('b') => {
                    Some(Action::Navigation(NavigationAction::PageUp))
                }
                KeyCode::Char('g') | KeyCode::Home => {
                    Some(Action::Navigation(NavigationAction::Top))
                }
                KeyCode::Char('G') | KeyCode::End => {
                    Some(Action::Navigation(NavigationAction::Bottom))
                }
                KeyCode::Char('y') => Some(Action::Password(PasswordAction::CopyPassword)),
                KeyCode::Char('l') | KeyCode::Right | KeyCode::Enter => {
                    Some(Action::Navigation(NavigationAction::Preview))
                }
                KeyCode::Char('/') => Some(Action::Navigation(NavigationAction::Search)),
                KeyCode::F(1) => Some(Action::Navigation(NavigationAction::Help)),
                KeyCode::Char('i') => Some(Action::Navigation(NavigationAction::File)),
                KeyCode::Char('x') => Some(Action::Password(PasswordAction::CopyOtp)),
                KeyCode::Char('c') => Some(Action::Password(PasswordAction::CopyPassId)),
                KeyCode::Char('v') => Some(Action::Password(PasswordAction::CopyLogin)),
                KeyCode::Esc => Some(Action::Navigation(NavigationAction::Leave)),
                KeyCode::Char('q') | KeyCode::Char('Q') => {
                    Some(Action::Navigation(NavigationAction::Quit))
                }
                _ => None,
            },
            State {
                main: _,
                search: SearchState::Active,
                overlay: OverlayState::Inactive,
            } => match key_event.code {
                KeyCode::Esc | KeyCode::Enter => Some(Action::Navigation(NavigationAction::Leave)),
                KeyCode::Down => Some(Action::Navigation(NavigationAction::Down)),
                KeyCode::Up => Some(Action::Navigation(NavigationAction::Up)),
                KeyCode::PageDown => Some(Action::Navigation(NavigationAction::PageDown)),
                KeyCode::PageUp => Some(Action::Navigation(NavigationAction::PageUp)),
                KeyCode::F(1) => Some(Action::Navigation(NavigationAction::Help)),
                KeyCode::Char(key) => Some(Action::Search(SearchAction::Insert(key))),
                KeyCode::Backspace => Some(Action::Search(SearchAction::RemoveLeft)),
                KeyCode::Delete => Some(Action::Search(SearchAction::RemoveRight)),
                KeyCode::Left => Some(Action::Search(SearchAction::MoveLeft)),
                KeyCode::Right => Some(Action::Search(SearchAction::MoveRight)),
                KeyCode::Home => Some(Action::Search(SearchAction::MoveToStart)),
                KeyCode::End => Some(Action::Search(SearchAction::MoveToEnd)),
                _ => None,
            },
            State {
                main: _,
                search: _,
                overlay: OverlayState::Help,
            } => match key_event.code {
                KeyCode::Esc | KeyCode::F(1) => Some(Action::Navigation(NavigationAction::Back)),
                _ => None,
            },
            State {
                main: _,
                search: _,
                overlay: OverlayState::File,
            } => match key_event.code {
                KeyCode::Esc | KeyCode::Char('i') => {
                    Some(Action::Navigation(NavigationAction::Back))
                }
                KeyCode::F(1) => Some(Action::Navigation(NavigationAction::Help)),
                _ => None,
            },
        }
    }

    fn handle_mouse_event(&mut self, event: MouseEvent) -> Option<Action> {
        self.dashboard.handle_mouse_event(event)
    }

    fn handle_channel_event(&mut self, event: PasswordEvent) -> Option<Action> {
        match event {
            PasswordEvent::Status(Ok(None)) => Some(Action::ResetStatus),
            PasswordEvent::Status(Ok(Some(message))) => Some(Action::SetStatus(message)),
            PasswordEvent::Status(Err(passepartout::Error::Pass(e))) => {
                Some(Action::SetStatus(format!("✗ (pass) {e:?}")))
            }
            PasswordEvent::Status(Err(passepartout::Error::Clipboard(e))) => {
                Some(Action::SetStatus(format!("✗ Clipboard error: {e:?}")))
            }
            PasswordEvent::Status(Err(e)) => Some(Action::SetStatus(format!("✗ {e:?}"))),
            PasswordEvent::PasswordFile {
                pass_id,
                file_contents,
            } => Some(Action::DisplaySecrets {
                pass_id,
                file_contents,
            }),
            PasswordEvent::OneTimePassword { pass_id, otp } => {
                Some(Action::DisplayOneTimePassword { pass_id, otp })
            }
        }
    }

    fn dispatch_action(&mut self, action: Action) -> Result<()> {
        let mut current_action = action;
        loop {
            // Actions from App take precedence
            if let Some(next) = self.update(current_action.clone())? {
                current_action = next;
                continue;
            }

            if let Some(next) = self.dashboard.update(current_action.clone())? {
                current_action = next;
                continue;
            }

            break;
        }
        Ok(())
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Navigation(NavigationAction::Quit) => self.quit(),
            Action::Redraw => self.request_redraw(),
            _ => (),
        }
        Ok(None)
    }

    fn request_redraw(&mut self) {
        self.complete_redraw = true;
    }

    fn quit(&mut self) {
        self.running = false;
    }
}
