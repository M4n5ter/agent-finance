use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders};

use crate::model::Panel;
use crate::pane_status::{TuiPaneStatus, pane_health};
use crate::state::AppState;

pub(super) fn compact_text(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let mut output = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() {
        output.push_str("...");
    }
    output
}

pub(super) fn format_price(value: f64) -> String {
    if value.abs() >= 100.0 {
        format!("{value:.2}")
    } else {
        format!("{value:.4}")
    }
}

pub(super) fn format_volume(value: f64) -> String {
    if value.abs() >= 1_000_000_000.0 {
        format!("{:.2}B", value / 1_000_000_000.0)
    } else if value.abs() >= 1_000_000.0 {
        format!("{:.2}M", value / 1_000_000.0)
    } else if value.abs() >= 1_000.0 {
        format!("{:.2}K", value / 1_000.0)
    } else {
        format!("{value:.0}")
    }
}

pub(super) fn panel_block(panel: Panel, state: &AppState) -> Block<'static> {
    let status = pane_health(state, panel).status;
    let style = if state.panels.focused() == panel {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(status_color(status))
    };
    let title = format!("{} [{}]", panel.title(), status.label());
    Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(style)
}

fn status_color(status: TuiPaneStatus) -> Color {
    match status {
        TuiPaneStatus::Fresh => Color::Green,
        TuiPaneStatus::Loading => Color::Yellow,
        TuiPaneStatus::Partial | TuiPaneStatus::Empty | TuiPaneStatus::Stale => Color::Gray,
        TuiPaneStatus::Error => Color::Red,
    }
}
