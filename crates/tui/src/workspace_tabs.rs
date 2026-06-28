use ratatui::layout::Rect;

use crate::model::WorkspaceKind;

const TAB_HORIZONTAL_PADDING: u16 = 2;
const TAB_DIVIDER_WIDTH: u16 = 1;

pub(crate) fn workspace_index(current: WorkspaceKind) -> usize {
    WorkspaceKind::ALL
        .iter()
        .position(|workspace| *workspace == current)
        .unwrap_or(0)
}

pub(crate) fn workspace_tabs_width() -> u16 {
    WorkspaceKind::ALL
        .iter()
        .copied()
        .enumerate()
        .map(|(index, workspace)| workspace_tab_width(workspace) + divider_width_after(index))
        .sum()
}

pub(crate) fn workspace_tab_at(area: Rect, column: u16) -> Option<WorkspaceKind> {
    let tab_width = workspace_tabs_width().min(area.width);
    if column >= area.x.saturating_add(tab_width) {
        return None;
    }

    let relative = column.saturating_sub(area.x);
    let mut cursor = 0;
    for (index, workspace) in WorkspaceKind::ALL.iter().copied().enumerate() {
        let start = cursor;
        let end = start + workspace_tab_width(workspace);
        if (start..end).contains(&relative) {
            return Some(workspace);
        }
        cursor = end + divider_width_after(index);
    }
    None
}

fn workspace_tab_width(workspace: WorkspaceKind) -> u16 {
    workspace.title().len() as u16 + TAB_HORIZONTAL_PADDING
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
        let mut cursor = area.x;

        for (index, workspace) in WorkspaceKind::ALL.iter().copied().enumerate() {
            assert_eq!(workspace_tab_at(area, cursor), Some(workspace));
            assert_eq!(
                workspace_tab_at(area, cursor + workspace_tab_width(workspace) - 1),
                Some(workspace)
            );
            cursor += workspace_tab_width(workspace);

            if index + 1 < WorkspaceKind::ALL.len() {
                assert_eq!(workspace_tab_at(area, cursor), None);
                cursor += TAB_DIVIDER_WIDTH;
            }
        }

        assert_eq!(workspace_tab_at(area, cursor), None);
    }
}
