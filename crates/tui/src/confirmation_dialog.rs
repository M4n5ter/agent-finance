use crate::model::FloatingKind;
use crate::state::{StagedExecution, StagedExecutionRequest};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum ConfirmationButtonAction {
    Primary,
    Cancel,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) enum ConfirmationRow {
    Text(String),
    Blank,
    Buttons(ConfirmationButtons),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct ConfirmationButtons {
    pub primary: Option<String>,
    pub cancel: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct ConfirmationButtonSegment {
    pub text: String,
    pub action: Option<ConfirmationButtonAction>,
    pub start: usize,
    pub end: usize,
}

pub(crate) fn rows_for(
    kind: FloatingKind,
    pending_staged_confirmation: Option<&StagedExecutionRequest>,
    content_width: usize,
) -> Vec<ConfirmationRow> {
    let rows = match kind {
        FloatingKind::LiveWritesConfirmation => live_writes_rows(),
        FloatingKind::StagedExecutionConfirmation => {
            staged_execution_rows(pending_staged_confirmation)
        }
        _ => Vec::new(),
    };
    materialize_visual_rows(rows, content_width)
}

pub(crate) fn click_action_at(
    rows: &[ConfirmationRow],
    content_column: usize,
    content_row: usize,
) -> Option<ConfirmationButtonAction> {
    let ConfirmationRow::Buttons(buttons) = rows.get(content_row)? else {
        return None;
    };
    button_segments(buttons)
        .into_iter()
        .find(|segment| (segment.start..segment.end).contains(&content_column))
        .and_then(|segment| segment.action)
}

pub(crate) fn button_segments(buttons: &ConfirmationButtons) -> Vec<ConfirmationButtonSegment> {
    let mut segments = Vec::new();
    let mut cursor = 0;
    if let Some(primary) = buttons.primary.as_deref() {
        push_button_segment(
            &mut segments,
            &mut cursor,
            format!("[{primary}]"),
            Some(ConfirmationButtonAction::Primary),
        );
        push_button_segment(&mut segments, &mut cursor, "  ".to_string(), None);
    }
    push_button_segment(
        &mut segments,
        &mut cursor,
        format!("[{}]", buttons.cancel),
        Some(ConfirmationButtonAction::Cancel),
    );
    segments
}

fn push_button_segment(
    segments: &mut Vec<ConfirmationButtonSegment>,
    cursor: &mut usize,
    text: String,
    action: Option<ConfirmationButtonAction>,
) {
    let start = *cursor;
    *cursor += text.chars().count();
    segments.push(ConfirmationButtonSegment {
        text,
        action,
        start,
        end: *cursor,
    });
}

fn live_writes_rows() -> Vec<ConfirmationRow> {
    vec![
        ConfirmationRow::Text("Live writes are disabled by default for every TUI session.".into()),
        ConfirmationRow::Blank,
        ConfirmationRow::Text(
            "Enabling live writes allows staged orders, cancels, transfers, and futures state changes to reach live providers after their own review and risk gates.".into(),
        ),
        ConfirmationRow::Blank,
        ConfirmationRow::Buttons(ConfirmationButtons {
            primary: Some("Enable live writes".into()),
            cancel: "Keep disabled".into(),
        }),
    ]
}

fn staged_execution_rows(request: Option<&StagedExecutionRequest>) -> Vec<ConfirmationRow> {
    let Some(request) = request else {
        return vec![
            ConfirmationRow::Text("No staged execution is waiting for confirmation.".into()),
            ConfirmationRow::Blank,
            ConfirmationRow::Buttons(ConfirmationButtons {
                primary: None,
                cancel: "Close".into(),
            }),
        ];
    };

    let mut rows = vec![
        ConfirmationRow::Text("Review the selected staged change before executing it.".into()),
        ConfirmationRow::Blank,
        ConfirmationRow::Text(format!("kind: {}", request.kind_label())),
        ConfirmationRow::Text(format!("id: {}", request.id)),
        ConfirmationRow::Text(format!("summary: {}", request.summary())),
        ConfirmationRow::Blank,
    ];
    match &request.execution {
        StagedExecution::Submit { mode, .. } => {
            rows.push(ConfirmationRow::Text(format!("mode: {mode}")));
            rows.push(ConfirmationRow::Blank);
            rows.push(ConfirmationRow::Text(
                "This creates an intent and runs the trading runtime gates.".into(),
            ));
            rows.push(ConfirmationRow::Text(
                "Live mode still requires profile permissions, risk policy, intent claim lock, and audit logging.".into(),
            ));
            rows.push(ConfirmationRow::Blank);
            rows.push(ConfirmationRow::Buttons(ConfirmationButtons {
                primary: Some("Confirm submit".into()),
                cancel: "Cancel".into(),
            }));
        }
        StagedExecution::LocalCommit { .. } => {
            rows.push(ConfirmationRow::Text(
                "This writes the profile file through the core profile store.".into(),
            ));
            rows.push(ConfirmationRow::Text(
                "A backup is created before replacing an existing profile.".into(),
            ));
            rows.push(ConfirmationRow::Text(
                "The write fails if the profile changes before commit.".into(),
            ));
            rows.push(ConfirmationRow::Blank);
            rows.push(ConfirmationRow::Buttons(ConfirmationButtons {
                primary: Some("Confirm local write".into()),
                cancel: "Cancel".into(),
            }));
        }
    }
    rows
}

fn materialize_visual_rows(
    rows: Vec<ConfirmationRow>,
    content_width: usize,
) -> Vec<ConfirmationRow> {
    let width = content_width.max(1);
    rows.into_iter()
        .flat_map(|row| match row {
            ConfirmationRow::Text(text) => wrap_text(&text, width)
                .into_iter()
                .map(ConfirmationRow::Text)
                .collect::<Vec<_>>(),
            row => vec![row],
        })
        .collect()
}

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let separator = usize::from(!current.is_empty());
        if !current.is_empty() && current.chars().count() + separator + word.chars().count() > width
        {
            lines.push(current);
            current = String::new();
        }
        if word.chars().count() > width {
            if !current.is_empty() {
                lines.push(current);
                current = String::new();
            }
            lines.extend(split_long_word(word, width));
            continue;
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn split_long_word(word: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for character in word.chars() {
        if current.chars().count() == width {
            lines.push(current);
            current = String::new();
        }
        current.push(character);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_writes_buttons_hit_primary_and_cancel_ranges() {
        let rows = rows_for(FloatingKind::LiveWritesConfirmation, None, 80);
        let button_row = rows
            .iter()
            .position(|row| matches!(row, ConfirmationRow::Buttons(_)))
            .expect("button row is present");
        assert_eq!(
            click_action_at(&rows, 1, button_row),
            Some(ConfirmationButtonAction::Primary)
        );
        assert_eq!(
            click_action_at(&rows, 24, button_row),
            Some(ConfirmationButtonAction::Cancel)
        );
        assert_eq!(click_action_at(&rows, 20, button_row), None);
        assert_eq!(click_action_at(&rows, 1, 0), None);
    }

    #[test]
    fn close_only_dialog_exposes_only_cancel_action() {
        let rows = rows_for(FloatingKind::StagedExecutionConfirmation, None, 80);
        let button_row = rows
            .iter()
            .position(|row| matches!(row, ConfirmationRow::Buttons(_)))
            .expect("button row is present");
        assert_eq!(
            click_action_at(&rows, 1, button_row),
            Some(ConfirmationButtonAction::Cancel)
        );
        assert_eq!(click_action_at(&rows, 8, button_row), None);
    }

    #[test]
    fn text_rows_wrap_before_render_and_hit_test() {
        let width = 40;
        let rows = rows_for(FloatingKind::LiveWritesConfirmation, None, width);

        assert!(rows.iter().all(|row| match row {
            ConfirmationRow::Text(text) => text.chars().count() <= width,
            _ => true,
        }));
        assert_eq!(
            rows.iter()
                .filter(|row| matches!(row, ConfirmationRow::Buttons(_)))
                .count(),
            1
        );
    }
}
