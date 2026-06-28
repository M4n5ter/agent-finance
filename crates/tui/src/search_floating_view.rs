use std::ops::Range;

use ratatui::layout::{Constraint, Direction, Layout, Rect};

const INPUT_BLOCK_HEIGHT: u16 = 3;
const LIST_BLOCK_CHROME_HEIGHT: u16 = 2;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SearchFloatingLayout {
    pub input_area: Rect,
    pub list_area: Rect,
    window: SearchFloatingWindow,
}

impl SearchFloatingLayout {
    pub fn new(area: Rect, total: usize, selected: usize) -> Self {
        let [input_area, list_area] = split_vertical(
            area,
            [Constraint::Length(INPUT_BLOCK_HEIGHT), Constraint::Min(0)],
        );
        Self {
            input_area,
            list_area,
            window: SearchFloatingWindow::new(total, selected, list_area),
        }
    }

    pub fn window(&self) -> &SearchFloatingWindow {
        &self.window
    }

    pub fn item_at_point(&self, column: u16, row: u16) -> Option<usize> {
        if column <= self.list_area.x
            || column >= self.list_area.right().saturating_sub(1)
            || row <= self.list_area.y
            || row >= self.list_area.bottom().saturating_sub(1)
        {
            return None;
        }
        let list_content_row = row.saturating_sub(self.list_area.y).saturating_sub(1) as usize;
        self.window.item_at_list_content_row(list_content_row)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SearchFloatingWindow {
    visible: Range<usize>,
}

impl SearchFloatingWindow {
    fn new(total: usize, selected: usize, list_area: Rect) -> Self {
        let capacity = list_capacity(list_area);
        Self {
            visible: visible_window(total, selected, capacity),
        }
    }

    pub fn visible(&self) -> Range<usize> {
        self.visible.clone()
    }

    pub fn start(&self) -> usize {
        self.visible.start
    }

    pub fn has_hidden_before(&self) -> bool {
        self.visible.start > 0
    }

    pub fn has_hidden_after(&self, total: usize) -> bool {
        self.visible.end < total
    }

    fn item_at_list_content_row(&self, list_content_row: usize) -> Option<usize> {
        self.visible
            .start
            .checked_add(list_content_row)
            .and_then(|index| {
                (self.visible.start..self.visible.end)
                    .contains(&index)
                    .then_some(index)
            })
    }
}

fn list_capacity(area: Rect) -> usize {
    area.height.saturating_sub(LIST_BLOCK_CHROME_HEIGHT).into()
}

fn visible_window(total: usize, selected: usize, capacity: usize) -> Range<usize> {
    if total == 0 || capacity == 0 {
        return 0..0;
    }

    let selected = selected.min(total - 1);
    let capacity = capacity.min(total);
    let start = selected.saturating_add(1).saturating_sub(capacity);
    let end = (start + capacity).min(total);
    start..end
}

fn split_vertical<const N: usize>(area: Rect, constraints: [Constraint; N]) -> [Rect; N] {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area)
        .as_ref()
        .try_into()
        .unwrap_or([Rect::default(); N])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_selected_entry_visible() {
        let area = Rect::new(0, 0, 40, 12);

        assert_eq!(
            SearchFloatingLayout::new(area, 11, 0).window().visible(),
            0..7
        );
        assert_eq!(
            SearchFloatingLayout::new(area, 11, 6).window().visible(),
            0..7
        );
        assert_eq!(
            SearchFloatingLayout::new(area, 11, 10).window().visible(),
            4..11
        );
    }

    #[test]
    fn maps_points_to_visible_entries() {
        let area = Rect::new(0, 0, 40, 12);
        let layout = SearchFloatingLayout::new(area, 11, 10);

        assert_eq!(layout.item_at_point(1, 4), Some(4));
        assert_eq!(layout.item_at_point(1, 10), Some(10));
        assert_eq!(layout.item_at_point(1, 2), None);
        assert_eq!(layout.item_at_point(1, 11), None);
    }
}
