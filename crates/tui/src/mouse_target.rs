use crate::layout::{CockpitLayout, LayoutHit};
use crate::model::{FloatingKind, Panel, WorkspaceKind};
use crate::state::AppState;
use crate::workspace_tabs::workspace_tab_at;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct MousePosition {
    pub column: u16,
    pub row: u16,
}

impl MousePosition {
    pub const fn new(column: u16, row: u16) -> Self {
        Self { column, row }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum MouseTarget {
    WorkspaceTab(WorkspaceKind),
    Panel(Panel),
    PanelAction {
        panel: Panel,
        action: PanelMouseAction,
    },
    Floating(FloatingKind),
    FloatingAction {
        kind: FloatingKind,
        action: FloatingMouseAction,
    },
    FloatingResize(FloatingKind),
    DockedSplit,
}

impl MouseTarget {
    pub fn status_hint(self) -> String {
        match self {
            Self::WorkspaceTab(workspace) => {
                format!("mouse: click to open {} workspace", workspace.label())
            }
            Self::Panel(panel) => format!("mouse: click to focus {}", panel.title()),
            Self::PanelAction { panel, action } => {
                format!("mouse: click to {} in {}", action.label(), panel.title())
            }
            Self::Floating(kind) => format!("mouse: click to focus {}", kind.title()),
            Self::FloatingAction { kind, action } => {
                format!("mouse: click to {} in {}", action.label(), kind.title())
            }
            Self::FloatingResize(kind) => format!("mouse: drag to resize {}", kind.title()),
            Self::DockedSplit => "mouse: drag to resize docked panes".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum PanelMouseAction {
    SelectRow,
    SelectField,
    StageReadyChange,
}

impl PanelMouseAction {
    const fn label(self) -> &'static str {
        match self {
            Self::SelectRow => "select row",
            Self::SelectField => "edit field",
            Self::StageReadyChange => "stage ready change",
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum FloatingMouseAction {
    ExecuteResult,
    SelectResult,
    Confirm,
    Cancel,
}

impl FloatingMouseAction {
    const fn label(self) -> &'static str {
        match self {
            Self::ExecuteResult => "execute result",
            Self::SelectResult => "select result",
            Self::Confirm => "confirm",
            Self::Cancel => "cancel",
        }
    }
}

pub(crate) fn target_at(
    state: &AppState,
    layout: &CockpitLayout,
    position: MousePosition,
) -> Option<MouseTarget> {
    if mouse_is_blocked_by_modal(state) {
        return modal_target_at(state, layout, position);
    }

    match layout.hit_test(position.column, position.row)? {
        LayoutHit::Panel(panel) => layout
            .panel_rect(panel)
            .and_then(|area| {
                crate::panel_mouse::hover_target(state, panel, area, position.column, position.row)
            })
            .or(Some(MouseTarget::Panel(panel))),
        LayoutHit::DockedSplit(_) => Some(MouseTarget::DockedSplit),
        LayoutHit::FloatingResize(kind) => Some(MouseTarget::FloatingResize(kind)),
        LayoutHit::Floating(kind) => layout
            .floating_rect(kind)
            .and_then(|area| {
                crate::floating_input::hover_target(
                    state,
                    kind,
                    area,
                    position.column,
                    position.row,
                )
            })
            .or(Some(MouseTarget::Floating(kind))),
        LayoutHit::Status => {
            workspace_tab_at(layout.status, position.column).map(MouseTarget::WorkspaceTab)
        }
    }
}

fn modal_target_at(
    state: &AppState,
    layout: &CockpitLayout,
    position: MousePosition,
) -> Option<MouseTarget> {
    let kind = state.floating.last()?.kind;
    if !matches!(
        kind,
        FloatingKind::LiveWritesConfirmation | FloatingKind::StagedExecutionConfirmation
    ) {
        return None;
    }
    let area = layout.floating_rect(kind)?;
    layout
        .hit_test(position.column, position.row)
        .and_then(|hit| match hit {
            LayoutHit::Floating(hit_kind) | LayoutHit::FloatingResize(hit_kind)
                if hit_kind == kind =>
            {
                crate::floating_input::hover_target(
                    state,
                    kind,
                    area,
                    position.column,
                    position.row,
                )
            }
            _ => None,
        })
}

fn mouse_is_blocked_by_modal(state: &AppState) -> bool {
    crate::floating_input::live_writes_confirmation_is_top(state)
        || crate::floating_input::staged_execution_confirmation_is_top(state)
}
