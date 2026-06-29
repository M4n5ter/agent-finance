use ratatui::layout::Rect;

use crate::model::WorkspaceKind;

const TAB_HORIZONTAL_PADDING: u16 = 2;
const TAB_DIVIDER_WIDTH: u16 = 1;

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct WorkspaceTabSegment {
    pub workspace: WorkspaceKind,
    pub label: String,
    pub start: u16,
    pub end: u16,
    pub has_divider_after: bool,
}

pub(crate) fn workspace_tabs_width() -> u16 {
    WorkspaceKind::ALL
        .iter()
        .copied()
        .enumerate()
        .map(|(index, workspace)| {
            workspace_tab_label(workspace).len() as u16 + divider_width_after(index)
        })
        .sum()
}

pub(crate) fn workspace_tab_at(area: Rect, column: u16) -> Option<WorkspaceKind> {
    workspace_tab_segments(area)
        .into_iter()
        .find(|segment| (segment.start..segment.end).contains(&column))
        .map(|segment| segment.workspace)
}

pub(crate) fn workspace_tab_segments(area: Rect) -> Vec<WorkspaceTabSegment> {
    let visible_right = area.x.saturating_add(area.width);
    let mut cursor = area.x;
    WorkspaceKind::ALL
        .iter()
        .copied()
        .enumerate()
        .filter_map(|(index, workspace)| {
            let label = workspace_tab_label(workspace);
            let start = cursor;
            let end = start.saturating_add(label.len() as u16).min(visible_right);
            let has_divider_after = index + 1 < WorkspaceKind::ALL.len();
            cursor = start
                .saturating_add(label.len() as u16)
                .saturating_add(divider_width_after(index));

            (start < end).then_some(WorkspaceTabSegment {
                workspace,
                label,
                start,
                end,
                has_divider_after,
            })
        })
        .collect()
}

fn workspace_tab_label(workspace: WorkspaceKind) -> String {
    let side_padding = " ".repeat((TAB_HORIZONTAL_PADDING / 2) as usize);
    format!("{side_padding}{}{side_padding}", workspace.title())
}

fn divider_width_after(index: usize) -> u16 {
    if index + 1 < WorkspaceKind::ALL.len() {
        TAB_DIVIDER_WIDTH
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn width_tracks_workspace_titles_and_chrome_padding() {
        let title_width = WorkspaceKind::ALL
            .iter()
            .map(|workspace| workspace.title().len() as u16)
            .sum::<u16>();

        assert_eq!(
            workspace_tabs_width(),
            title_width
                + WorkspaceKind::ALL.len() as u16 * TAB_HORIZONTAL_PADDING
                + WorkspaceKind::ALL.len().saturating_sub(1) as u16 * TAB_DIVIDER_WIDTH
        );
        assert!(workspace_tabs_width() < 80);
    }

    #[test]
    fn hit_testing_maps_rendered_tab_ranges() {
        let area = Rect::new(4, 10, 120, 1);
        let segments = workspace_tab_segments(area);

        for segment in &segments {
            assert_eq!(
                workspace_tab_at(area, segment.start),
                Some(segment.workspace)
            );
            assert_eq!(
                workspace_tab_at(area, segment.end - 1),
                Some(segment.workspace)
            );
            assert_eq!(segment.label, workspace_tab_label(segment.workspace));
        }

        let first_divider = segments
            .iter()
            .find(|segment| segment.has_divider_after)
            .expect("at least one divider")
            .end;
        assert_eq!(workspace_tab_at(area, first_divider), None);
        assert_eq!(
            workspace_tab_at(area, area.x + workspace_tabs_width()),
            None
        );
    }
}
