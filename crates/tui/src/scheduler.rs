use std::cell::Cell;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread;

use agent_finance_market::{
    service::MarketRuntime,
    snapshot::{self, MarketSnapshot, PublicQuoteSnapshotRequest},
};
use anyhow::{Result, anyhow};

use crate::config::TuiLaunch;

#[derive(Debug)]
pub struct Scheduler {
    commands: Sender<SchedulerCommand>,
    events: Receiver<SchedulerEvent>,
    disconnected_reported: Cell<bool>,
}

impl Scheduler {
    pub fn start(launch: &TuiLaunch) -> Self {
        let (commands, command_rx) = mpsc::channel();
        let (event_tx, events) = mpsc::channel();
        let runtime = MarketRuntime::new(
            launch.proxy.as_deref(),
            launch.no_proxy,
            launch.timeout_seconds,
            &launch.timezone,
        );

        thread::Builder::new()
            .name("agent-finance-tui-scheduler".to_string())
            .spawn(move || run_worker(runtime, command_rx, event_tx))
            .expect("failed to spawn TUI scheduler thread");

        Self {
            commands,
            events,
            disconnected_reported: Cell::new(false),
        }
    }

    pub fn request_refresh(&self, generation: u64, symbols: Vec<String>) -> Result<()> {
        self.commands
            .send(SchedulerCommand::Refresh {
                generation,
                symbols,
            })
            .map_err(|error| anyhow!("failed to request TUI refresh: {error}"))
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
enum SchedulerCommand {
    Refresh {
        generation: u64,
        symbols: Vec<String>,
    },
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
    Fatal(String),
}

fn run_worker(
    runtime: MarketRuntime,
    commands: Receiver<SchedulerCommand>,
    events: Sender<SchedulerEvent>,
) {
    let tokio = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            let _ = events.send(SchedulerEvent::Fatal(format!(
                "failed to start scheduler runtime: {error}"
            )));
            return;
        }
    };

    while let Ok(command) = commands.recv() {
        match command {
            SchedulerCommand::Refresh {
                generation,
                symbols,
            } => {
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
    }
}

async fn fetch_snapshot(runtime: &MarketRuntime, symbols: Vec<String>) -> Result<MarketSnapshot> {
    snapshot::fetch_public_quote_snapshot(runtime, PublicQuoteSnapshotRequest { symbols }).await
}
