use crate::command::ActionId;
use crate::model::{FloatingKind, Panel, WorkspaceKind};
use crate::state::{Action, AppState};

impl AppState {
    pub(super) fn execute(&mut self, action: ActionId) {
        match action {
            ActionId::SelectSymbolBy(direction) => {
                self.close_text_input_floatings();
                self.shift_symbol(direction);
            }
            ActionId::OpenFloating(kind) => {
                if kind.text_input() {
                    self.close_text_input_floatings_except(kind);
                } else {
                    self.close_text_input_floatings();
                }
                self.open_floating(kind);
            }
            ActionId::CloseFocusedFloating => {
                self.reduce(Action::CloseFocusedFloating);
            }
            ActionId::ResetLayout => {
                self.reduce(Action::ResetLayout);
            }
            ActionId::FocusPanel(panel) => self.focus_panel_from_command(panel),
            ActionId::TogglePanel(panel) => self.toggle_panel_from_command(panel),
            ActionId::ShiftWorkspace(direction) => {
                self.close_text_input_floatings();
                self.reduce(Action::ShiftWorkspace(direction));
            }
            ActionId::SetWorkspace(workspace) => self.set_workspace_from_command(workspace),
            ActionId::CloseFocusedPanel => {
                self.close_text_input_floatings();
                self.reduce(Action::CloseFocusedPanel);
            }
            ActionId::RestorePanels => {
                self.close_text_input_floatings();
                self.reduce(Action::RestorePanels);
            }
            ActionId::FocusPanelBy(direction) => {
                self.close_text_input_floatings();
                self.reduce(Action::FocusPanelBy(direction));
            }
            ActionId::ToggleFocusedZoom => {
                self.close_text_input_floatings();
                self.reduce(Action::ToggleFocusedZoom);
            }
            ActionId::ToggleLiveWrites => {
                self.close_text_input_floatings();
                if self.live_writes_enabled {
                    self.reduce(Action::SetLiveWritesEnabled(false));
                } else {
                    self.open_floating(FloatingKind::LiveWritesConfirmation);
                }
            }
            ActionId::StageOrderTicket => {
                self.close_text_input_floatings();
                self.reduce(Action::StageOrderTicket);
            }
            ActionId::CloseCommandPalette => {
                self.close_floating(FloatingKind::CommandPalette);
            }
        }
    }

    fn focus_panel_from_command(&mut self, panel: Panel) {
        self.close_text_input_floatings();
        self.reduce(Action::Focus(panel));
    }

    fn toggle_panel_from_command(&mut self, panel: Panel) {
        self.close_text_input_floatings();
        self.toggle_panel(panel);
    }

    fn set_workspace_from_command(&mut self, workspace: WorkspaceKind) {
        self.close_text_input_floatings();
        self.reduce(Action::SetWorkspace(workspace));
    }
}
