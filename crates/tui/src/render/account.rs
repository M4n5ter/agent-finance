use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::{List, ListItem};

use crate::account_panel_view;
use crate::model::Panel;
use crate::mouse_target::MouseTarget;
use crate::state::AppState;

use super::widgets::panel_block;

pub(super) fn render_account(
    frame: &mut Frame<'_>,
    state: &AppState,
    area: Rect,
    mouse_target: Option<MouseTarget>,
) {
    let items =
        account_panel_view::rows_for_width(state, mouse_target, area.width.saturating_sub(2))
            .into_iter()
            .map(|row| ListItem::new(row.line));
    frame.render_widget(
        List::new(items).block(panel_block(Panel::Account, state)),
        area,
    );
}
