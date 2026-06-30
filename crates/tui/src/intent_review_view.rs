pub(crate) const INTENT_REVIEW_SUMMARY_ROWS: u16 = 4;
pub(crate) const INTENT_REVIEW_ACTION_ROW: usize = INTENT_REVIEW_SUMMARY_ROWS as usize - 1;

const TABLE_HEADER_ROWS: usize = 1;

pub(crate) type IntentReviewActionLine = crate::action_line_view::ActionLine<IntentReviewAction>;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum IntentReviewAction {
    ExecuteSelected,
    CloseSelected,
}

pub(crate) fn staged_change_index_at_content_row(
    visible_len: usize,
    content_row: usize,
) -> Option<usize> {
    let index = content_row.checked_sub(staged_change_content_row(0))?;
    (index < visible_len).then_some(index)
}

pub(crate) const fn staged_change_content_row(index: usize) -> usize {
    first_staged_change_content_row() + index
}

const fn first_staged_change_content_row() -> usize {
    INTENT_REVIEW_SUMMARY_ROWS as usize + TABLE_HEADER_ROWS
}

pub(crate) fn action_line(hidden: usize, width: u16) -> IntentReviewActionLine {
    let mut line = IntentReviewActionLine::new(crate::hints::intent_review_panel_hint(), width);
    line.push_visible_text("  ");
    line.push_visible_action("[execute]", IntentReviewAction::ExecuteSelected);
    line.push_visible_text("  ");
    line.push_visible_action("[close]", IntentReviewAction::CloseSelected);
    if hidden > 0 {
        line.push_visible_text(&format!("  +{hidden} hidden staged change(s)"));
    }
    line
}

pub(crate) fn action_at_content_cell(
    hidden: usize,
    width: u16,
    content_row: usize,
    content_column: u16,
) -> Option<IntentReviewAction> {
    if content_row != INTENT_REVIEW_ACTION_ROW {
        return None;
    }
    action_line(hidden, width)
        .action_at(content_column)
        .map(|span| span.action)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_row_maps_to_staged_change_index_below_summary_and_header() {
        assert_eq!(staged_change_index_at_content_row(2, 4), None);
        assert_eq!(staged_change_index_at_content_row(2, 5), Some(0));
        assert_eq!(staged_change_index_at_content_row(2, 6), Some(1));
        assert_eq!(staged_change_index_at_content_row(2, 7), None);
    }

    #[test]
    fn action_line_maps_visible_buttons_to_actions() {
        let line = action_line(3, 120);
        let execute = line
            .actions
            .iter()
            .find(|span| span.action == IntentReviewAction::ExecuteSelected)
            .expect("execute action");
        let close = line
            .actions
            .iter()
            .find(|span| span.action == IntentReviewAction::CloseSelected)
            .expect("close action");

        assert_eq!(line.action_text(execute), "[execute]");
        assert_eq!(
            action_at_content_cell(3, 120, INTENT_REVIEW_ACTION_ROW, execute.start),
            Some(IntentReviewAction::ExecuteSelected)
        );
        assert_eq!(line.action_text(close), "[close]");
        assert_eq!(
            action_at_content_cell(3, 120, INTENT_REVIEW_ACTION_ROW, close.end - 1),
            Some(IntentReviewAction::CloseSelected)
        );
        assert_eq!(action_at_content_cell(3, 120, 0, execute.start), None);
    }

    #[test]
    fn narrow_action_line_does_not_expose_hidden_actions() {
        let line = action_line(0, 40);

        assert!(unicode_width::UnicodeWidthStr::width(line.text.as_str()) <= 40);
        assert!(line.actions.is_empty());
        assert_eq!(
            action_at_content_cell(0, 40, INTENT_REVIEW_ACTION_ROW, 39),
            None
        );
    }
}
