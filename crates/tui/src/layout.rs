use ratatui::layout::{Constraint, Direction, Layout, Rect};

use crate::config::{
    LayoutConfig, MAX_LEFT_MAIN_RATIO, MAX_LEFT_RATIO, MAX_MAIN_RATIO, MIN_LEFT_RATIO,
    MIN_MAIN_RATIO, MIN_RIGHT_RATIO,
};
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
    columns: Option<DockedColumns>,
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

    pub fn panel_at(&self, x: u16, y: u16) -> Option<Panel> {
        [
            Panel::Watchlist,
            Panel::Quote,
            Panel::History,
            Panel::Evidence,
            Panel::Research,
            Panel::ProviderHealth,
            Panel::TaskLog,
        ]
        .into_iter()
        .find(|panel| contains(self.panel_rect(*panel), x, y))
    }

    pub fn hit_test(&self, x: u16, y: u16) -> Option<LayoutHit> {
        if let Some(floating) = self
            .floating
            .iter()
            .rev()
            .find(|floating| contains(floating.rect, x, y))
        {
            return Some(LayoutHit::Floating(floating.kind));
        }
        if contains(self.status, x, y) {
            return Some(LayoutHit::Status);
        }
        if let Some(split) = self.docked_split_at(x, y) {
            return Some(LayoutHit::DockedSplit(split));
        }
        self.panel_at(x, y).map(LayoutHit::Panel)
    }

    fn docked_split_at(&self, x: u16, y: u16) -> Option<DockedColumnSplit> {
        let columns = self.columns.as_ref()?;
        if !contains(columns.left, x, y)
            && !contains(columns.middle, x, y)
            && !contains(columns.right, x, y)
        {
            return None;
        }

        let left_boundary = columns.left.x.saturating_add(columns.left.width);
        let right_boundary = columns.middle.x.saturating_add(columns.middle.width);
        if near_column_boundary(x, left_boundary) {
            Some(DockedColumnSplit::LeftMain)
        } else if near_column_boundary(x, right_boundary) {
            Some(DockedColumnSplit::MainRight)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct FloatingRect {
    pub kind: FloatingKind,
    pub rect: Rect,
    pub z_index: u16,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct DockedColumns {
    left: Rect,
    middle: Rect,
    right: Rect,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum DockedColumnSplit {
    LeftMain,
    MainRight,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum LayoutHit {
    Floating(FloatingKind),
    DockedSplit(DockedColumnSplit),
    Panel(Panel),
    Status,
}

pub fn build(area: Rect, config: &LayoutConfig, floating: &[FloatingPane]) -> CockpitLayout {
    let [body, status] = split_vertical(
        area,
        [Constraint::Min(0), Constraint::Length(STATUS_HEIGHT)],
    );
    let compact = body.width < MIN_PANEL_WIDTH * 3 || body.height < MIN_PANEL_HEIGHT * 3;

    let (watchlist, quote, history, evidence, research, provider_health, task_log, columns) =
        if compact {
            let (watchlist, quote, history, evidence, research, provider_health, task_log) =
                compact_layout(body);
            (
                watchlist,
                quote,
                history,
                evidence,
                research,
                provider_health,
                task_log,
                None,
            )
        } else {
            let columns = docked_columns(body, config);
            let (watchlist, quote, history, evidence, research, provider_health, task_log) =
                wide_layout(columns);
            (
                watchlist,
                quote,
                history,
                evidence,
                research,
                provider_health,
                task_log,
                Some(columns),
            )
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
        columns,
    }
}

pub fn resize_docked_columns(
    area: Rect,
    split: DockedColumnSplit,
    x: u16,
    config: &LayoutConfig,
) -> LayoutConfig {
    let [body, _status] = split_vertical(
        area,
        [Constraint::Min(0), Constraint::Length(STATUS_HEIGHT)],
    );
    if body.width == 0 {
        return config.clone();
    }

    let pointer_ratio =
        (((u32::from(x.saturating_sub(body.x)) * 100) / u32::from(body.width)).min(100)) as u16;
    let mut next = config.clone();
    match split {
        DockedColumnSplit::LeftMain => {
            let left_and_main = config.left_ratio.saturating_add(config.main_ratio);
            let max_left = MAX_LEFT_RATIO.min(left_and_main.saturating_sub(MIN_MAIN_RATIO));
            next.left_ratio = pointer_ratio.clamp(MIN_LEFT_RATIO, max_left);
            next.main_ratio = left_and_main.saturating_sub(next.left_ratio);
        }
        DockedColumnSplit::MainRight => {
            let max_main = MAX_MAIN_RATIO.min(MAX_LEFT_MAIN_RATIO.saturating_sub(next.left_ratio));
            next.main_ratio = pointer_ratio
                .saturating_sub(next.left_ratio)
                .clamp(MIN_MAIN_RATIO, max_main);
        }
    }
    next.normalize();
    next
}

fn docked_columns(area: Rect, config: &LayoutConfig) -> DockedColumns {
    let right_ratio = 100u16.saturating_sub(config.left_ratio + config.main_ratio);
    let [left, middle, right] = split_horizontal(
        area,
        [
            Constraint::Percentage(config.left_ratio),
            Constraint::Percentage(config.main_ratio),
            Constraint::Percentage(right_ratio.max(MIN_RIGHT_RATIO)),
        ],
    );
    DockedColumns {
        left,
        middle,
        right,
    }
}

fn wide_layout(columns: DockedColumns) -> (Rect, Rect, Rect, Rect, Rect, Rect, Rect) {
    let [watchlist, provider_health, task_log] = split_vertical(
        columns.left,
        [
            Constraint::Percentage(55),
            Constraint::Percentage(25),
            Constraint::Percentage(20),
        ],
    );
    let [quote, history] = split_vertical(
        columns.middle,
        [
            Constraint::Length(9.min(columns.middle.height)),
            Constraint::Min(0),
        ],
    );
    let [evidence, research] = split_vertical(
        columns.right,
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

fn contains(rect: Rect, x: u16, y: u16) -> bool {
    x >= rect.x
        && x < rect.x.saturating_add(rect.width)
        && y >= rect.y
        && y < rect.y.saturating_add(rect.height)
}

fn near_column_boundary(x: u16, boundary: u16) -> bool {
    x.abs_diff(boundary) <= 1
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

    #[test]
    fn wide_layout_maps_points_to_panels_and_split_handles() {
        let area = Rect::new(0, 0, 160, 48);
        let config = LayoutConfig::default();
        let layout = build(area, &config, &[]);

        assert_eq!(layout.panel_at(2, 2), Some(Panel::Watchlist));
        assert_eq!(layout.panel_at(80, 2), Some(Panel::Quote));
        assert_eq!(layout.panel_at(150, 36), Some(Panel::Research));

        assert_eq!(
            layout.hit_test(layout.watchlist.x + layout.watchlist.width, 2),
            Some(LayoutHit::DockedSplit(DockedColumnSplit::LeftMain))
        );
        assert_eq!(
            layout.hit_test(layout.quote.x + layout.quote.width, 2),
            Some(LayoutHit::DockedSplit(DockedColumnSplit::MainRight))
        );
    }

    #[test]
    fn resizing_docked_columns_clamps_to_usable_ratios() {
        let area = Rect::new(0, 0, 160, 48);
        let config = LayoutConfig::default();

        let narrow_left = resize_docked_columns(area, DockedColumnSplit::LeftMain, 4, &config);
        assert_eq!(narrow_left.left_ratio, 15);

        let wide_main = resize_docked_columns(area, DockedColumnSplit::MainRight, 150, &config);
        assert_eq!(wide_main.main_ratio, 56);
        assert!(wide_main.left_ratio + wide_main.main_ratio <= MAX_LEFT_MAIN_RATIO);
    }

    #[test]
    fn resizing_left_main_preserves_right_column_share() {
        let area = Rect::new(0, 0, 160, 48);
        let config = LayoutConfig::default();
        let initial_right = 100 - config.left_ratio - config.main_ratio;

        let resized = resize_docked_columns(area, DockedColumnSplit::LeftMain, 56, &config);

        assert_eq!(100 - resized.left_ratio - resized.main_ratio, initial_right);
        assert_eq!(resized.left_ratio, 35);
        assert_eq!(resized.main_ratio, 35);
    }

    #[test]
    fn resizing_left_main_does_not_borrow_from_right_when_main_is_minimum() {
        let area = Rect::new(0, 0, 160, 48);
        let config = LayoutConfig {
            left_ratio: 24,
            main_ratio: 35,
        };
        let initial_right = 100 - config.left_ratio - config.main_ratio;

        let resized = resize_docked_columns(area, DockedColumnSplit::LeftMain, 56, &config);

        assert_eq!(100 - resized.left_ratio - resized.main_ratio, initial_right);
        assert_eq!(resized.left_ratio, 24);
        assert_eq!(resized.main_ratio, 35);
    }

    #[test]
    fn floating_hit_test_blocks_docked_panel_passthrough() {
        let layout = build(
            Rect::new(0, 0, 160, 48),
            &LayoutConfig::default(),
            &[FloatingPane {
                kind: FloatingKind::Help,
                z_index: 1,
            }],
        );
        let floating = layout.floating[0];

        assert_eq!(
            layout.hit_test(floating.rect.x + 1, floating.rect.y + 1),
            Some(LayoutHit::Floating(FloatingKind::Help))
        );
    }

    #[test]
    fn resizing_docked_columns_uses_full_wide_terminal_range() {
        let area = Rect::new(0, 0, 1_000, 48);
        let config = LayoutConfig::default();

        let resized = resize_docked_columns(area, DockedColumnSplit::MainRight, 920, &config);

        assert_eq!(resized.main_ratio, 56);
    }
}
