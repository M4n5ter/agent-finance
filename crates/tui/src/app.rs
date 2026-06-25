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
use crate::scheduler::{Scheduler, SchedulerEvent};
use crate::state::{Action, AppState, FloatingKind, Panel};

type TuiTerminal = Terminal<CrosstermBackend<Stdout>>;

pub fn run(launch: TuiLaunch) -> Result<()> {
    let persisted_config = launch.load_config()?;
    let runtime_config = launch.runtime_config(persisted_config.clone());
    let mut state = AppState::from_config(runtime_config.clone());
    state.reduce(Action::Log("cockpit initialized".to_string()));

    let mut terminal = TerminalGuard::enter().context("failed to initialize terminal UI")?;
    let scheduler = Scheduler::start(&launch);
    let mut next_refresh_generation = 1;
    request_refresh(&scheduler, &mut state, &mut next_refresh_generation);

    let result = run_loop(
        terminal.terminal_mut()?,
        &mut state,
        &runtime_config.layout,
        runtime_config.refresh.price_seconds,
        &scheduler,
        &mut next_refresh_generation,
        &launch,
    );
    let restore_result = terminal.leave();

    result.and(restore_result)
}

fn run_loop(
    terminal: &mut TuiTerminal,
    state: &mut AppState,
    layout: &crate::config::LayoutConfig,
    refresh_seconds: u64,
    scheduler: &Scheduler,
    next_refresh_generation: &mut u64,
    launch: &TuiLaunch,
) -> Result<()> {
    let mut last_tick = Instant::now();
    let mut last_refresh = Instant::now();
    let refresh_interval = refresh_seconds.max(2);
    loop {
        terminal.draw(|frame| render::render(frame, state, layout))?;

        while let Some(event) = scheduler.try_recv() {
            match event {
                SchedulerEvent::Snapshot {
                    generation,
                    snapshot,
                } => state.reduce(Action::SnapshotLoaded {
                    generation,
                    snapshot,
                }),
                SchedulerEvent::RefreshFailed { generation, error } => {
                    state.reduce(Action::RefreshFailed { generation, error })
                }
                SchedulerEvent::Fatal(error) => state.reduce(Action::SchedulerFailed(error)),
            }
        }

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

        if last_refresh.elapsed().as_secs() >= refresh_interval {
            request_refresh(scheduler, state, next_refresh_generation);
            last_refresh = Instant::now();
        }
    }
    Ok(())
}

fn request_refresh(scheduler: &Scheduler, state: &mut AppState, next_generation: &mut u64) {
    let Some(request) = prepare_refresh_request(state, next_generation) else {
        return;
    };

    if let Err(error) = scheduler.request_refresh(request.generation, request.symbols) {
        state.reduce(Action::SchedulerFailed(error.to_string()));
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct RefreshRequest {
    generation: u64,
    symbols: Vec<String>,
}

fn prepare_refresh_request(
    state: &mut AppState,
    next_generation: &mut u64,
) -> Option<RefreshRequest> {
    if state.refreshing || state.scheduler_error.is_some() {
        return None;
    }

    let generation = *next_generation;
    *next_generation = next_generation.saturating_add(1);
    state.reduce(Action::RefreshStarted(generation));
    Some(RefreshRequest {
        generation,
        symbols: state.watchlist.clone(),
    })
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

    #[test]
    fn refresh_request_does_not_enqueue_while_previous_refresh_is_in_flight() {
        let mut state = AppState::from_config(crate::config::TuiConfig::default());
        let mut next_generation = 1;

        let first = prepare_refresh_request(&mut state, &mut next_generation);
        assert_eq!(
            first,
            Some(RefreshRequest {
                generation: 1,
                symbols: state.watchlist.clone(),
            })
        );
        assert!(state.refreshing);
        assert_eq!(next_generation, 2);

        let second = prepare_refresh_request(&mut state, &mut next_generation);
        assert_eq!(second, None);
        assert_eq!(state.refresh_generation, 1);
        assert_eq!(next_generation, 2);
    }

    #[test]
    fn refresh_request_does_not_enqueue_after_scheduler_fatal_failure() {
        let mut state = AppState::from_config(crate::config::TuiConfig::default());
        let mut next_generation = 1;

        state.reduce(Action::SchedulerFailed("scheduler failed".to_string()));

        assert_eq!(
            prepare_refresh_request(&mut state, &mut next_generation),
            None
        );
        assert_eq!(next_generation, 1);
    }
}
