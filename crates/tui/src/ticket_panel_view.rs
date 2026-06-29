use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::action_line_view::{ActionLine, ActionSpan};
use crate::command::ActionId;
use crate::panel_action_line_view::{PanelActionLine, PanelActionSpan};

const FIELD_PREV_LABEL: &str = "[prev]";
const FIELD_NEXT_LABEL: &str = "[next]";
const FIELD_ACTION_GAP: u16 = 1;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum TicketPanelRow {
    Header,
    Detail(usize),
    Action(usize),
    Field(usize),
    ReadyAction,
    Blocker(usize),
    Hint,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum TicketPanelClick {
    Field(usize),
    ReadyAction,
}

pub(crate) type TicketFieldActionLine = ActionLine<TicketFieldAction>;
pub(crate) type TicketFieldActionSpan = ActionSpan<TicketFieldAction>;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) struct TicketFieldAction {
    pub index: usize,
    pub direction: isize,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) struct TicketPanelAction {
    pub label: &'static str,
    pub action: ActionId,
}

impl TicketPanelAction {
    pub(crate) fn line(self, width: u16) -> PanelActionLine {
        let mut line = PanelActionLine::new("", width);
        line.push_visible_action(self.label, self.action);
        line
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct TicketPanelRows {
    pub detail_count: usize,
    pub actions: &'static [TicketPanelAction],
    pub field_count: usize,
    pub field_adjustable: Vec<bool>,
    pub ready: bool,
    pub blocker_count: usize,
}

impl TicketPanelRows {
    pub(crate) fn rows(&self) -> Vec<TicketPanelRow> {
        let mut rows = vec![TicketPanelRow::Header];
        rows.extend((0..self.detail_count).map(TicketPanelRow::Detail));
        rows.extend((0..self.field_count).map(TicketPanelRow::Field));
        if self.ready {
            rows.push(TicketPanelRow::ReadyAction);
        } else {
            rows.extend((0..self.blocker_count.min(3)).map(TicketPanelRow::Blocker));
        }
        rows.extend((0..self.actions.len()).map(TicketPanelRow::Action));
        rows.push(TicketPanelRow::Hint);
        rows
    }

    pub(crate) fn click_at(&self, content_row: usize) -> Option<TicketPanelClick> {
        match self.rows().get(content_row)? {
            TicketPanelRow::Field(index) => Some(TicketPanelClick::Field(*index)),
            TicketPanelRow::ReadyAction => Some(TicketPanelClick::ReadyAction),
            _ => None,
        }
    }

    pub(crate) fn action_at_content_cell(
        &self,
        width: u16,
        content_row: usize,
        content_column: u16,
    ) -> Option<PanelActionSpan> {
        let rows = self.rows();
        let TicketPanelRow::Action(index) = rows.get(content_row)? else {
            return None;
        };
        self.actions
            .get(*index)?
            .line(width)
            .action_at(content_column)
    }

    pub(crate) fn field_action_at_content_cell(
        &self,
        width: u16,
        content_row: usize,
        content_column: u16,
    ) -> Option<TicketFieldActionSpan> {
        let rows = self.rows();
        let TicketPanelRow::Field(index) = rows.get(content_row)? else {
            return None;
        };
        field_action_line(width, *index, "", self.field_is_adjustable(*index))
            .action_at(content_column)
    }

    fn field_is_adjustable(&self, index: usize) -> bool {
        self.field_adjustable.get(index).copied().unwrap_or(true)
    }
}

pub(crate) fn field_action_line(
    width: u16,
    index: usize,
    text: &str,
    adjustable: bool,
) -> TicketFieldActionLine {
    let prev_width = UnicodeWidthStr::width(FIELD_PREV_LABEL) as u16;
    let next_width = UnicodeWidthStr::width(FIELD_NEXT_LABEL) as u16;
    let total_width = prev_width
        .saturating_add(FIELD_ACTION_GAP)
        .saturating_add(next_width);
    let mut line = TicketFieldActionLine::new("", width);
    if !adjustable || width <= total_width {
        line.push_visible_text(text);
        return line;
    }

    let text_width = width - total_width - FIELD_ACTION_GAP;
    let visible_text = truncate_to_width(text, text_width);
    let visible_width = UnicodeWidthStr::width(visible_text.as_str()) as u16;
    line.push_visible_text(&visible_text);
    line.push_visible_text(&" ".repeat((width - total_width - visible_width) as usize));
    line.push_visible_action(
        FIELD_PREV_LABEL,
        TicketFieldAction {
            index,
            direction: -1,
        },
    );
    line.push_visible_text(" ");
    line.push_visible_action(
        FIELD_NEXT_LABEL,
        TicketFieldAction {
            index,
            direction: 1,
        },
    );
    line
}

pub(crate) fn truncate_to_width(text: &str, width: u16) -> String {
    let mut output = String::new();
    let mut used = 0u16;
    for character in text.chars() {
        let character_width = character.width().unwrap_or(0) as u16;
        if used.saturating_add(character_width) > width {
            break;
        }
        output.push(character);
        used = used.saturating_add(character_width);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_mark_fields_and_ready_action_as_clickable() {
        let rows = TicketPanelRows {
            detail_count: 1,
            actions: &[],
            field_count: 2,
            field_adjustable: vec![true, true],
            ready: true,
            blocker_count: 0,
        };

        assert_eq!(
            rows.rows(),
            vec![
                TicketPanelRow::Header,
                TicketPanelRow::Detail(0),
                TicketPanelRow::Field(0),
                TicketPanelRow::Field(1),
                TicketPanelRow::ReadyAction,
                TicketPanelRow::Hint,
            ]
        );
        assert_eq!(rows.click_at(2), Some(TicketPanelClick::Field(0)));
        assert_eq!(rows.click_at(4), Some(TicketPanelClick::ReadyAction));
        assert_eq!(rows.click_at(5), None);
    }

    #[test]
    fn blocked_rows_do_not_expose_stage_action() {
        let rows = TicketPanelRows {
            detail_count: 0,
            actions: &[],
            field_count: 1,
            field_adjustable: vec![true],
            ready: false,
            blocker_count: 4,
        };

        assert_eq!(
            rows.rows(),
            vec![
                TicketPanelRow::Header,
                TicketPanelRow::Field(0),
                TicketPanelRow::Blocker(0),
                TicketPanelRow::Blocker(1),
                TicketPanelRow::Blocker(2),
                TicketPanelRow::Hint,
            ]
        );
        assert_eq!(rows.click_at(1), Some(TicketPanelClick::Field(0)));
        assert_eq!(rows.click_at(2), None);
    }

    #[test]
    fn rows_place_ticket_actions_after_state_rows() {
        const ACTIONS: &[TicketPanelAction] = &[TicketPanelAction {
            label: "[capture price]",
            action: ActionId::CaptureOrderReferencePrice,
        }];
        let rows = TicketPanelRows {
            detail_count: 1,
            actions: ACTIONS,
            field_count: 1,
            field_adjustable: vec![true],
            ready: false,
            blocker_count: 1,
        };

        assert_eq!(
            rows.rows(),
            vec![
                TicketPanelRow::Header,
                TicketPanelRow::Detail(0),
                TicketPanelRow::Field(0),
                TicketPanelRow::Blocker(0),
                TicketPanelRow::Action(0),
                TicketPanelRow::Hint,
            ]
        );
        assert_eq!(rows.click_at(2), Some(TicketPanelClick::Field(0)));
        assert_eq!(rows.click_at(4), None);
        assert_eq!(
            rows.action_at_content_cell(80, 4, 0)
                .map(|span| (span.label, span.action)),
            Some(("[capture price]", ActionId::CaptureOrderReferencePrice))
        );
        assert_eq!(rows.action_at_content_cell(80, 4, 15), None);
        assert_eq!(rows.action_at_content_cell(3, 4, 0), None);
    }

    #[test]
    fn field_actions_are_right_aligned_and_hidden_when_narrow() {
        let rows = TicketPanelRows {
            detail_count: 0,
            actions: &[],
            field_count: 1,
            field_adjustable: vec![true],
            ready: false,
            blocker_count: 0,
        };

        let prev = rows
            .field_action_at_content_cell(30, 1, 18)
            .expect("prev action is visible");
        let next = rows
            .field_action_at_content_cell(30, 1, 25)
            .expect("next action is visible");

        assert_eq!(
            prev.action,
            TicketFieldAction {
                index: 0,
                direction: -1
            }
        );
        assert_eq!(
            next.action,
            TicketFieldAction {
                index: 0,
                direction: 1
            }
        );
        assert_eq!(rows.field_action_at_content_cell(12, 1, 0), None);
    }

    #[test]
    fn inactive_fields_do_not_expose_adjust_actions() {
        let rows = TicketPanelRows {
            detail_count: 0,
            actions: &[],
            field_count: 2,
            field_adjustable: vec![true, false],
            ready: false,
            blocker_count: 0,
        };

        assert!(rows.field_action_at_content_cell(30, 2, 18).is_none());
    }
}
