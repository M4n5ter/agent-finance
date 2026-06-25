use std::io::{self, Stdout};
use std::time::Instant;

use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::config::TuiLaunch;
use crate::render;
use crate::state::{Action, AppState, FloatingKind, Panel};

type TuiTerminal = Terminal<CrosstermBackend<Stdout>>;

pub fn run(launch: TuiLaunch) -> Result<()> {
    let persisted_config = launch.load_config()?;
    let runtime_config = launch.runtime_config(persisted_config.clone());
    let mut state = AppState::from_config(runtime_config.clone());
    state.reduce(Action::Log("cockpit initialized".to_string()));

    let mut terminal = TerminalGuard::enter().context("failed to initialize terminal UI")?;
    let result = run_loop(
        terminal.terminal_mut()?,
        &mut state,
        &runtime_config.layout,
        &launch,
    );
    let restore_result = terminal.leave();

    result.and(restore_result)
}

fn run_loop(
    terminal: &mut TuiTerminal,
    state: &mut AppState,
    layout: &crate::config::LayoutConfig,
    launch: &TuiLaunch,
) -> Result<()> {
    let mut last_tick = Instant::now();
    loop {
        terminal.draw(|frame| render::render(frame, state, layout))?;

        let timeout = launch
            .tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_default();
        if event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) if should_quit(key) => break,
                Event::Key(key) => {
                    if let Some(action) = key_action(key) {
                        state.reduce(action);
                    }
                }
                _ => {}
            }
        }

        if last_tick.elapsed() >= launch.tick_rate {
            last_tick = Instant::now();
        }
    }
    Ok(())
}

fn key_action(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => Some(Action::SelectNextSymbol),
        KeyCode::Char('k') | KeyCode::Up => Some(Action::SelectPreviousSymbol),
        KeyCode::Char('h') | KeyCode::F(1) => Some(Action::ToggleFloating(FloatingKind::Help)),
        KeyCode::Char(':') => Some(Action::ToggleFloating(FloatingKind::CommandPalette)),
        KeyCode::Char('p') => Some(Action::ToggleFloating(FloatingKind::ProviderDetails)),
        KeyCode::Char('r') => Some(Action::ResetLayout),
        KeyCode::Esc => Some(Action::CloseFocusedFloating),
        KeyCode::Char('1') => Some(Action::Focus(Panel::Watchlist)),
        KeyCode::Char('2') => Some(Action::Focus(Panel::Quote)),
        KeyCode::Char('3') => Some(Action::Focus(Panel::History)),
        _ => None,
    }
}

fn should_quit(key: KeyEvent) -> bool {
    matches!(key.code, KeyCode::Char('q'))
        || (matches!(key.code, KeyCode::Char('c')) && key.modifiers.contains(KeyModifiers::CONTROL))
}

struct TerminalGuard {
    terminal: Option<TuiTerminal>,
    raw_mode: bool,
    alternate_screen: bool,
}

impl TerminalGuard {
    fn enter() -> Result<Self> {
        let mut guard = Self {
            terminal: None,
            raw_mode: false,
            alternate_screen: false,
        };

        enable_raw_mode()?;
        guard.raw_mode = true;

        if let Err(error) = execute!(io::stdout(), EnterAlternateScreen) {
            let _ = guard.cleanup();
            return Err(error.into());
        }
        guard.alternate_screen = true;

        let backend = CrosstermBackend::new(io::stdout());
        match Terminal::new(backend) {
            Ok(terminal) => {
                guard.terminal = Some(terminal);
                Ok(guard)
            }
            Err(error) => {
                let _ = guard.cleanup();
                Err(error.into())
            }
        }
    }

    fn terminal_mut(&mut self) -> Result<&mut TuiTerminal> {
        self.terminal
            .as_mut()
            .context("terminal UI was not initialized")
    }

    fn leave(&mut self) -> Result<()> {
        self.cleanup()
    }

    fn cleanup(&mut self) -> Result<()> {
        let mut first_error = None;

        if self.alternate_screen {
            let result = if let Some(terminal) = self.terminal.as_mut() {
                execute!(terminal.backend_mut(), LeaveAlternateScreen)
            } else {
                execute!(io::stdout(), LeaveAlternateScreen)
            };
            if let Err(error) = result {
                first_error.get_or_insert_with(|| anyhow::Error::from(error));
            }
            self.alternate_screen = false;
        }

        if let Some(terminal) = self.terminal.as_mut()
            && let Err(error) = terminal.show_cursor()
        {
            first_error.get_or_insert_with(|| anyhow::Error::from(error));
        }

        if self.raw_mode {
            if let Err(error) = disable_raw_mode() {
                first_error.get_or_insert_with(|| anyhow::Error::from(error));
            }
            self.raw_mode = false;
        }

        if let Some(error) = first_error {
            Err(error)
        } else {
            Ok(())
        }
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = self.leave();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_router_maps_navigation_and_overlays_to_actions() {
        assert_eq!(
            key_action(KeyEvent::from(KeyCode::Char('j'))),
            Some(Action::SelectNextSymbol)
        );
        assert_eq!(
            key_action(KeyEvent::from(KeyCode::Char(':'))),
            Some(Action::ToggleFloating(FloatingKind::CommandPalette))
        );
        assert_eq!(
            key_action(KeyEvent::from(KeyCode::Esc)),
            Some(Action::CloseFocusedFloating)
        );
    }

    #[test]
    fn quit_router_accepts_q_and_control_c_only() {
        assert!(should_quit(KeyEvent::from(KeyCode::Char('q'))));
        assert!(should_quit(KeyEvent::new(
            KeyCode::Char('c'),
            KeyModifiers::CONTROL
        )));
        assert!(!should_quit(KeyEvent::from(KeyCode::Char('c'))));
    }
}
