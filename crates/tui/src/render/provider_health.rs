use std::cmp::Reverse;

use ratatui::layout::Constraint;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Cell, Row};

use crate::provider_health::{
    ProviderHealthProvider, ProviderHealthReport, ProviderHealthSeverity, ProviderHealthTask,
};

pub(super) fn table_rows(report: ProviderHealthReport, limit: usize) -> Vec<Row<'static>> {
    display_rows(report, limit)
        .into_iter()
        .map(table_row)
        .collect()
}

pub(super) fn table_widths() -> [Constraint; 4] {
    [
        Constraint::Length(12),
        Constraint::Length(12),
        Constraint::Min(18),
        Constraint::Length(16),
    ]
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct DisplayRow {
    provider: String,
    status: &'static str,
    detail: String,
    freshness: String,
    severity: ProviderHealthSeverity,
}

fn display_rows(report: ProviderHealthReport, limit: usize) -> Vec<DisplayRow> {
    let mut rows = report
        .providers
        .into_iter()
        .map(HealthRow::Provider)
        .chain(report.tasks.into_iter().map(HealthRow::Task))
        .collect::<Vec<_>>();
    rows.sort_by_key(|row| {
        (
            Reverse(row.severity()),
            Reverse(row.is_task()),
            row.label().to_string(),
        )
    });
    rows.into_iter().map(display_row).take(limit).collect()
}

enum HealthRow {
    Provider(ProviderHealthProvider),
    Task(ProviderHealthTask),
}

impl HealthRow {
    fn severity(&self) -> ProviderHealthSeverity {
        match self {
            Self::Provider(provider) => provider.severity,
            Self::Task(task) => task.status,
        }
    }

    fn is_task(&self) -> bool {
        matches!(self, Self::Task(_))
    }

    fn label(&self) -> &str {
        match self {
            Self::Provider(provider) => provider.provider.as_str(),
            Self::Task(task) => task.source.label(),
        }
    }
}

fn display_row(row: HealthRow) -> DisplayRow {
    match row {
        HealthRow::Provider(provider) => provider_display_row(provider),
        HealthRow::Task(task) => task_display_row(task),
    }
}

fn provider_display_row(provider: ProviderHealthProvider) -> DisplayRow {
    let severity = provider.severity;
    let status = status_label(severity);
    let freshness = provider.freshness.unwrap_or_else(|| "-".to_string());
    let detail = provider
        .signals
        .iter()
        .take(2)
        .map(|signal| format!("{}={}", signal.source.label(), signal.detail))
        .collect::<Vec<_>>()
        .join("; ");
    DisplayRow {
        provider: provider.provider,
        status,
        detail,
        freshness,
        severity,
    }
}

fn task_display_row(task: ProviderHealthTask) -> DisplayRow {
    let severity = task.status;
    let status = status_label(severity);
    DisplayRow {
        provider: "task".to_string(),
        status,
        detail: format!("{} {}", task.source.label(), task.detail),
        freshness: "-".to_string(),
        severity,
    }
}

fn table_row(row: DisplayRow) -> Row<'static> {
    let style = status_style(row.severity);
    Row::new([
        Cell::from(row.provider).style(style),
        Cell::from(row.status).style(style),
        Cell::from(row.detail),
        Cell::from(row.freshness).style(Style::default().fg(Color::DarkGray)),
    ])
}

fn status_label(status: ProviderHealthSeverity) -> &'static str {
    match status {
        ProviderHealthSeverity::Ok => "ok",
        ProviderHealthSeverity::Warning => "warn",
        ProviderHealthSeverity::Loading => "load",
    }
}

fn status_style(status: ProviderHealthSeverity) -> Style {
    match status {
        ProviderHealthSeverity::Ok => Style::default().fg(Color::Green),
        ProviderHealthSeverity::Warning => Style::default().fg(Color::Yellow),
        ProviderHealthSeverity::Loading => Style::default().fg(Color::Cyan),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider_health::{ProviderHealthSignal, ProviderHealthSource, ProviderHealthTask};

    #[test]
    fn display_rows_prioritize_actionable_tasks_before_ok_providers() {
        let report = ProviderHealthReport {
            providers: vec![ProviderHealthProvider {
                provider: "yahoo".to_string(),
                severity: ProviderHealthSeverity::Ok,
                signals: vec![ProviderHealthSignal {
                    source: ProviderHealthSource::Quotes,
                    status: ProviderHealthSeverity::Ok,
                    detail: "1 priced quotes".to_string(),
                }],
                freshness: None,
            }],
            tasks: vec![ProviderHealthTask {
                source: ProviderHealthSource::History,
                status: ProviderHealthSeverity::Warning,
                detail: "timeout".to_string(),
            }],
        };

        let rows = display_rows(report, 1);

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].provider, "task");
        assert_eq!(rows[0].status, "warn");
        assert_eq!(rows[0].detail, "history timeout");
    }
}
