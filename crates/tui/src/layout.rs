use ratatui::layout::{Constraint, Direction, Layout, Rect};

use crate::config::LayoutConfig;
use crate::state::{FloatingKind, FloatingPane, Panel};

const MIN_PANEL_WIDTH: u16 = 18;
const MIN_PANEL_HEIGHT: u16 = 4;
const STATUS_HEIGHT: u16 = 1;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CockpitLayout {
    pub watchlist: Rect,
    pub quote: Rect,
    pub history: Rect,
    pub evidence: Rect,
    pub research: Rect,
    pub provider_health: Rect,
    pub task_log: Rect,
    pub status: Rect,
    pub floating: Vec<FloatingRect>,
}

impl CockpitLayout {
    pub fn panel_rect(&self, panel: Panel) -> Rect {
        match panel {
            Panel::Watchlist => self.watchlist,
            Panel::Quote => self.quote,
            Panel::History => self.history,
            Panel::Evidence => self.evidence,
            Panel::Research => self.research,
            Panel::ProviderHealth => self.provider_health,
            Panel::TaskLog => self.task_log,
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct FloatingRect {
    pub kind: FloatingKind,
    pub rect: Rect,
    pub z_index: u16,
}

pub fn build(area: Rect, config: &LayoutConfig, floating: &[FloatingPane]) -> CockpitLayout {
    let [body, status] = split_vertical(
        area,
        [Constraint::Min(0), Constraint::Length(STATUS_HEIGHT)],
    );
    let compact = body.width < MIN_PANEL_WIDTH * 3 || body.height < MIN_PANEL_HEIGHT * 3;

    let (watchlist, quote, history, evidence, research, provider_health, task_log) = if compact {
        compact_layout(body)
    } else {
        wide_layout(body, config)
    };

    let mut floating = floating
        .iter()
        .map(|pane| FloatingRect {
            kind: pane.kind,
            rect: floating_rect(body, pane.kind),
            z_index: pane.z_index,
        })
        .collect::<Vec<_>>();
    floating.sort_by_key(|pane| pane.z_index);

    CockpitLayout {
        watchlist,
        quote,
        history,
        evidence,
        research,
        provider_health,
        task_log,
        status,
        floating,
    }
}

fn wide_layout(area: Rect, config: &LayoutConfig) -> (Rect, Rect, Rect, Rect, Rect, Rect, Rect) {
    let right_ratio = 100u16.saturating_sub(config.left_ratio + config.main_ratio);
    let [left, middle, right] = split_horizontal(
        area,
        [
            Constraint::Percentage(config.left_ratio),
            Constraint::Percentage(config.main_ratio),
            Constraint::Percentage(right_ratio.max(20)),
        ],
    );

    let [watchlist, provider_health, task_log] = split_vertical(
        left,
        [
            Constraint::Percentage(55),
            Constraint::Percentage(25),
            Constraint::Percentage(20),
        ],
    );
    let [quote, history] = split_vertical(
        middle,
        [Constraint::Length(9.min(middle.height)), Constraint::Min(0)],
    );
    let [evidence, research] = split_vertical(
        right,
        [Constraint::Percentage(48), Constraint::Percentage(52)],
    );

    (
        non_empty(watchlist),
        non_empty(quote),
        non_empty(history),
        non_empty(evidence),
        non_empty(research),
        non_empty(provider_health),
        non_empty(task_log),
    )
}

fn compact_layout(area: Rect) -> (Rect, Rect, Rect, Rect, Rect, Rect, Rect) {
    let [top, middle, bottom] = split_vertical(
        area,
        [
            Constraint::Length(8.min(area.height)),
            Constraint::Percentage(55),
            Constraint::Percentage(45),
        ],
    );
    let [watchlist, quote] = split_horizontal(
        top,
        [Constraint::Percentage(40), Constraint::Percentage(60)],
    );
    let [history, evidence] = split_horizontal(
        middle,
        [Constraint::Percentage(60), Constraint::Percentage(40)],
    );
    let [research, provider_health, task_log] = split_horizontal(
        bottom,
        [
            Constraint::Percentage(45),
            Constraint::Percentage(25),
            Constraint::Percentage(30),
        ],
    );

    (
        non_empty(watchlist),
        non_empty(quote),
        non_empty(history),
        non_empty(evidence),
        non_empty(research),
        non_empty(provider_health),
        non_empty(task_log),
    )
}

fn floating_rect(area: Rect, kind: FloatingKind) -> Rect {
    let (width_ratio, height_ratio) = match kind {
        FloatingKind::CommandPalette => (70, 40),
        FloatingKind::Help => (64, 70),
        FloatingKind::ProviderDetails => (58, 58),
    };
    let width = ((area.width as u32 * width_ratio) / 100)
        .clamp(MIN_PANEL_WIDTH as u32, area.width.max(1) as u32) as u16;
    let height = ((area.height as u32 * height_ratio) / 100)
        .clamp(MIN_PANEL_HEIGHT as u32, area.height.max(1) as u32) as u16;
    Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    }
}

fn split_horizontal<const N: usize>(area: Rect, constraints: [Constraint; N]) -> [Rect; N] {
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(area)
        .to_vec()
        .try_into()
        .unwrap_or_else(|_| [Rect::default(); N])
}

fn split_vertical<const N: usize>(area: Rect, constraints: [Constraint; N]) -> [Rect; N] {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area)
        .to_vec()
        .try_into()
        .unwrap_or_else(|_| [Rect::default(); N])
}

fn non_empty(rect: Rect) -> Rect {
    Rect {
        width: rect.width.max(1),
        height: rect.height.max(1),
        ..rect
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::FloatingPane;

    #[test]
    fn wide_layout_preserves_all_docked_panels_and_status_bar() {
        let layout = build(Rect::new(0, 0, 160, 48), &LayoutConfig::default(), &[]);

        assert_eq!(layout.status.height, 1);
        for panel in [
            Panel::Watchlist,
            Panel::Quote,
            Panel::History,
            Panel::Evidence,
            Panel::Research,
            Panel::ProviderHealth,
            Panel::TaskLog,
        ] {
            let rect = layout.panel_rect(panel);
            assert!(rect.width > 0, "{panel:?} should have width");
            assert!(rect.height > 0, "{panel:?} should have height");
        }
    }

    #[test]
    fn compact_layout_does_not_generate_zero_sized_panels() {
        let layout = build(Rect::new(0, 0, 42, 16), &LayoutConfig::default(), &[]);

        assert!(layout.history.width > 0);
        assert!(layout.evidence.height > 0);
        assert_eq!(layout.status.height, 1);
    }

    #[test]
    fn floating_rects_are_clamped_and_sorted_by_z_index() {
        let layout = build(
            Rect::new(0, 0, 100, 32),
            &LayoutConfig::default(),
            &[
                FloatingPane {
                    kind: FloatingKind::Help,
                    z_index: 4,
                },
                FloatingPane {
                    kind: FloatingKind::CommandPalette,
                    z_index: 2,
                },
            ],
        );

        assert_eq!(layout.floating[0].kind, FloatingKind::CommandPalette);
        assert_eq!(layout.floating[1].kind, FloatingKind::Help);
        for floating in layout.floating {
            assert!(floating.rect.width <= 100);
            assert!(floating.rect.height <= 31);
            assert!(floating.rect.width >= MIN_PANEL_WIDTH);
            assert!(floating.rect.height >= MIN_PANEL_HEIGHT);
        }
    }
}
