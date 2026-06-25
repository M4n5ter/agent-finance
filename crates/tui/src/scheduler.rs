use std::cell::Cell;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;

use agent_finance_market::{
    history_snapshot::{self, HistorySnapshot, HistorySnapshotRequest},
    service::MarketRuntime,
    snapshot::{self, MarketSnapshot, PublicQuoteSnapshotRequest},
};
use anyhow::{Result, anyhow};

use crate::config::TuiLaunch;

#[derive(Debug)]
pub struct Scheduler {
    refresh_commands: Sender<RefreshCommand>,
    history_commands: Sender<HistoryCommand>,
    events: Receiver<SchedulerEvent>,
    disconnected_reported: Cell<bool>,
}

impl Scheduler {
    pub fn start(launch: &TuiLaunch) -> Self {
        let (refresh_commands, refresh_command_rx) = mpsc::channel();
        let (history_commands, history_command_rx) = mpsc::channel();
        let (event_tx, events) = mpsc::channel();
        let runtime = MarketRuntime::new(
            launch.proxy.as_deref(),
            launch.no_proxy,
            launch.timeout_seconds,
            &launch.timezone,
        );

        thread::Builder::new()
            .name("agent-finance-tui-refresh".to_string())
            .spawn({
                let runtime = runtime.clone();
                let event_tx = event_tx.clone();
                move || run_refresh_worker(runtime, refresh_command_rx, event_tx)
            })
            .expect("failed to spawn TUI refresh scheduler thread");

        thread::Builder::new()
            .name("agent-finance-tui-history".to_string())
            .spawn(move || run_history_worker(runtime, history_command_rx, event_tx))
            .expect("failed to spawn TUI history scheduler thread");

        Self {
            refresh_commands,
            history_commands,
            events,
            disconnected_reported: Cell::new(false),
        }
    }

    pub fn request_refresh(&self, generation: u64, symbols: Vec<String>) -> Result<()> {
        self.refresh_commands
            .send(RefreshCommand {
                generation,
                symbols,
            })
            .map_err(|error| anyhow!("failed to request TUI refresh: {error}"))
    }

    pub fn request_history(&self, generation: u64, symbol: String) -> Result<()> {
        self.history_commands
            .send(HistoryCommand { generation, symbol })
            .map_err(|error| anyhow!("failed to request TUI history: {error}"))
    }

    pub fn try_recv(&self) -> Option<SchedulerEvent> {
        match self.events.try_recv() {
            Ok(event) => Some(event),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) if !self.disconnected_reported.replace(true) => Some(
                SchedulerEvent::Fatal("scheduler worker stopped".to_string()),
            ),
            Err(TryRecvError::Disconnected) => None,
        }
    }
}

#[derive(Debug)]
struct RefreshCommand {
    generation: u64,
    symbols: Vec<String>,
}

#[derive(Debug)]
struct HistoryCommand {
    generation: u64,
    symbol: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SchedulerEvent {
    Snapshot {
        generation: u64,
        snapshot: MarketSnapshot,
    },
    RefreshFailed {
        generation: u64,
        error: String,
    },
    History {
        generation: u64,
        snapshot: HistorySnapshot,
    },
    HistoryFailed {
        generation: u64,
        symbol: String,
        error: String,
    },
    Fatal(String),
}

fn run_refresh_worker(
    runtime: MarketRuntime,
    commands: Receiver<RefreshCommand>,
    events: Sender<SchedulerEvent>,
) {
    let Some(tokio) = scheduler_runtime("refresh", &events) else {
        return;
    };

    while let Ok(command) = commands.recv() {
        let RefreshCommand {
            generation,
            symbols,
        } = command;
        let result = tokio.block_on(fetch_snapshot(&runtime, symbols));
        let event = match result {
            Ok(snapshot) => SchedulerEvent::Snapshot {
                generation,
                snapshot,
            },
            Err(error) => SchedulerEvent::RefreshFailed {
                generation,
                error: error.to_string(),
            },
        };
        if events.send(event).is_err() {
            break;
        }
    }
}

fn run_history_worker(
    runtime: MarketRuntime,
    commands: Receiver<HistoryCommand>,
    events: Sender<SchedulerEvent>,
) {
    let Some(tokio) = scheduler_runtime("history", &events) else {
        return;
    };

    while let Ok(command) = commands.recv() {
        let HistoryCommand { generation, symbol } = command;
        let result = tokio.block_on(fetch_history(&runtime, symbol.clone()));
        let event = match result {
            Ok(snapshot) => SchedulerEvent::History {
                generation,
                snapshot,
            },
            Err(error) => SchedulerEvent::HistoryFailed {
                generation,
                symbol,
                error: error.to_string(),
            },
        };
        if events.send(event).is_err() {
            break;
        }
    }
}

fn scheduler_runtime(
    worker_name: &str,
    events: &Sender<SchedulerEvent>,
) -> Option<tokio::runtime::Runtime> {
    match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => Some(runtime),
        Err(error) => {
            let _ = events.send(SchedulerEvent::Fatal(format!(
                "failed to start {worker_name} scheduler runtime: {error}"
            )));
            None
        }
    }
}

async fn fetch_snapshot(runtime: &MarketRuntime, symbols: Vec<String>) -> Result<MarketSnapshot> {
    snapshot::fetch_public_quote_snapshot(runtime, PublicQuoteSnapshotRequest { symbols }).await
}

async fn fetch_history(runtime: &MarketRuntime, symbol: String) -> Result<HistorySnapshot> {
    history_snapshot::fetch_history_snapshot(
        runtime,
        HistorySnapshotRequest {
            symbol,
            interval: "1d".to_string(),
            range: "6mo".to_string(),
            limit: 90,
        },
    )
    .await
}
