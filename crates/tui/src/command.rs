#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct CommandPaletteState {
    pub selected: usize,
}

impl CommandPaletteState {
    pub fn shift(&mut self, direction: isize) {
        let len = ACTION_SPECS.len() as isize;
        let selected = self.selected as isize;
        self.selected = (selected + direction).rem_euclid(len) as usize;
    }

    pub fn selected_command(&self) -> CommandSpec {
        ACTION_SPECS[self.selected]
    }

    pub fn selected_action(&self) -> ActionId {
        self.selected_command().action
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct CommandSpec {
    pub title: &'static str,
    pub description: &'static str,
    pub action: ActionId,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ActionId {
    OpenFloating(FloatingKind),
    ResetLayout,
    CloseFocusedPanel,
    RestorePanels,
    ShiftWorkspace(isize),
    SetWorkspace(WorkspaceKind),
    FocusPanel(Panel),
    TogglePanel(Panel),
    CloseCommandPalette,
}

pub const ACTION_SPECS: [CommandSpec; 28] = [
    CommandSpec {
        title: "Open help",
        description: "Show cockpit shortcuts and interaction model",
        action: ActionId::OpenFloating(FloatingKind::Help),
    },
    CommandSpec {
        title: "Open provider details",
        description: "Inspect provider capability coverage",
        action: ActionId::OpenFloating(FloatingKind::ProviderDetails),
    },
    CommandSpec {
        title: "Reset layout",
        description: "Restore default docked columns and close overlays",
        action: ActionId::ResetLayout,
    },
    CommandSpec {
        title: "Close focused panel",
        description: "Hide the focused docked panel and move focus to another open panel",
        action: ActionId::CloseFocusedPanel,
    },
    CommandSpec {
        title: "Restore all panels",
        description: "Reopen every docked panel without changing the current symbol",
        action: ActionId::RestorePanels,
    },
    CommandSpec {
        title: "Next workspace",
        description: "Move to the next workspace tab",
        action: ActionId::ShiftWorkspace(1),
    },
    CommandSpec {
        title: "Previous workspace",
        description: "Move to the previous workspace tab",
        action: ActionId::ShiftWorkspace(-1),
    },
    CommandSpec {
        title: "Workspace overview",
        description: "Show the overview cockpit workspace",
        action: ActionId::SetWorkspace(WorkspaceKind::Overview),
    },
    CommandSpec {
        title: "Workspace research",
        description: "Show news, research, and prediction-market context",
        action: ActionId::SetWorkspace(WorkspaceKind::Research),
    },
    CommandSpec {
        title: "Workspace crypto",
        description: "Show crypto evidence and market context",
        action: ActionId::SetWorkspace(WorkspaceKind::Crypto),
    },
    CommandSpec {
        title: "Workspace providers",
        description: "Show provider health and runtime task status",
        action: ActionId::SetWorkspace(WorkspaceKind::Providers),
    },
    CommandSpec {
        title: "Focus watchlist",
        description: "Move keyboard focus to the symbol list",
        action: ActionId::FocusPanel(Panel::Watchlist),
    },
    CommandSpec {
        title: "Focus quote",
        description: "Move keyboard focus to quote and session summary",
        action: ActionId::FocusPanel(Panel::Quote),
    },
    CommandSpec {
        title: "Focus history",
        description: "Move keyboard focus to historical price chart",
        action: ActionId::FocusPanel(Panel::History),
    },
    CommandSpec {
        title: "Focus crypto evidence",
        description: "Move keyboard focus to crypto provider evidence",
        action: ActionId::FocusPanel(Panel::Evidence),
    },
    CommandSpec {
        title: "Focus Polymarket",
        description: "Move keyboard focus to prediction market signals",
        action: ActionId::FocusPanel(Panel::Polymarket),
    },
    CommandSpec {
        title: "Focus research",
        description: "Move keyboard focus to news and research highlights",
        action: ActionId::FocusPanel(Panel::Research),
    },
    CommandSpec {
        title: "Focus provider health",
        description: "Move keyboard focus to provider health",
        action: ActionId::FocusPanel(Panel::ProviderHealth),
    },
    CommandSpec {
        title: "Focus task log",
        description: "Move keyboard focus to runtime task log",
        action: ActionId::FocusPanel(Panel::TaskLog),
    },
    CommandSpec {
        title: "Toggle watchlist",
        description: "Show or hide the symbol list panel",
        action: ActionId::TogglePanel(Panel::Watchlist),
    },
    CommandSpec {
        title: "Toggle quote",
        description: "Show or hide quote and session summary",
        action: ActionId::TogglePanel(Panel::Quote),
    },
    CommandSpec {
        title: "Toggle history",
        description: "Show or hide the historical price chart",
        action: ActionId::TogglePanel(Panel::History),
    },
    CommandSpec {
        title: "Toggle crypto evidence",
        description: "Show or hide crypto provider evidence",
        action: ActionId::TogglePanel(Panel::Evidence),
    },
    CommandSpec {
        title: "Toggle Polymarket",
        description: "Show or hide prediction market signals",
        action: ActionId::TogglePanel(Panel::Polymarket),
    },
    CommandSpec {
        title: "Toggle research",
        description: "Show or hide news and research highlights",
        action: ActionId::TogglePanel(Panel::Research),
    },
    CommandSpec {
        title: "Toggle provider health",
        description: "Show or hide provider capability coverage",
        action: ActionId::TogglePanel(Panel::ProviderHealth),
    },
    CommandSpec {
        title: "Toggle task log",
        description: "Show or hide the runtime task log",
        action: ActionId::TogglePanel(Panel::TaskLog),
    },
    CommandSpec {
        title: "Close command palette",
        description: "Dismiss this command palette without changing docked panels",
        action: ActionId::CloseCommandPalette,
    },
];
use crate::model::{FloatingKind, Panel, WorkspaceKind};
