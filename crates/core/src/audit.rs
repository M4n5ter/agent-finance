use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::paths::data_dir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub timestamp_utc: DateTime<Utc>,
    pub profile: String,
    pub provider: String,
    pub environment: String,
    pub intent_id: Option<String>,
    pub kind: AuditEventKind,
    pub summary: String,
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AuditEventKind {
    IntentCreated,
    DryRun,
    TestSubmit,
    LiveSubmit,
    Cancel,
    Transfer,
    Error,
}

pub fn append_audit_event(event: &AuditEvent) -> Result<PathBuf> {
    let path = audit_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("failed to open audit log {}", path.display()))?;
    writeln!(file, "{}", serde_json::to_string(event)?)
        .with_context(|| format!("failed to append audit log {}", path.display()))?;
    Ok(path)
}

pub fn read_audit_events(limit: usize) -> Result<Vec<AuditEvent>> {
    let path = audit_path()?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file = fs::File::open(&path)
        .with_context(|| format!("failed to open audit log {}", path.display()))?;
    let mut events = BufReader::new(file)
        .lines()
        .map(|line| -> Result<AuditEvent> { Ok(serde_json::from_str(&line?)?) })
        .collect::<Result<Vec<_>>>()?;
    if events.len() > limit {
        events = events.split_off(events.len() - limit);
    }
    Ok(events)
}

fn audit_path() -> Result<PathBuf> {
    Ok(data_dir()?.join("audit").join("events.jsonl"))
}
