use crossterm::event::{KeyCode, KeyEvent};

use crate::command::ActionId;
use crate::model::FloatingKind;
use crate::state::Action;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) struct ProfileRiskActionSpec {
    pub key: char,
    pub label: &'static str,
    pub action: ActionId,
}

pub(crate) const PROFILE_RISK_ACTIONS: [ProfileRiskActionSpec; 3] = [
    ProfileRiskActionSpec {
        key: 'e',
        label: "e profile",
        action: ActionId::OpenFloating(FloatingKind::TradingProfile),
    },
    ProfileRiskActionSpec {
        key: 'v',
        label: "v validate",
        action: ActionId::RevalidateTradingProfile,
    },
    ProfileRiskActionSpec {
        key: 't',
        label: "t stage risk",
        action: ActionId::StageProfileLiveToggle,
    },
];

pub(crate) fn profile_risk_key_action(key: KeyEvent) -> Option<Action> {
    if !key.modifiers.is_empty() {
        return None;
    }
    match key.code {
        KeyCode::Char(character) => PROFILE_RISK_ACTIONS
            .into_iter()
            .find(|spec| spec.key == character)
            .map(|spec| Action::Execute(spec.action)),
        _ => None,
    }
}

pub(crate) fn profile_risk_key_hints() -> Vec<String> {
    PROFILE_RISK_ACTIONS
        .iter()
        .map(|spec| spec.label)
        .chain(["q quit"])
        .map(str::to_string)
        .collect()
}
