use agent_finance_core::{
    DecimalValue, DiagnosticCheck, FuturesStateChange, Market, OrderIdentifier, OrderKind,
    OrderSide, OrderSpec, TimeInForce, TransferDirection,
    submit::{SubmitIntentKind, SubmitMode},
};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub struct StagedChangeRequest {
    pub id: String,
    pub subject: StagedChangeSubject,
}

impl StagedChangeRequest {
    #[cfg(test)]
    pub fn text(id: &str, intent_kind: SubmitIntentKind, summary: &str) -> Self {
        Self {
            id: id.to_string(),
            subject: StagedChangeSubject::Text {
                intent_kind,
                summary: summary.to_string(),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StagedSubmitRequest {
    pub id: String,
    pub subject: StagedSubmitSubject,
    pub mode: SubmitMode,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum StagedSubmitSubject {
    OrderTicket(OrderTicketReview),
    Cancel(CancelReview),
    Transfer(TransferReview),
    FuturesState(FuturesStateReview),
    #[cfg(test)]
    Text {
        intent_kind: SubmitIntentKind,
        summary: String,
    },
}

impl StagedSubmitSubject {
    pub fn intent_kind(&self) -> SubmitIntentKind {
        match self {
            Self::OrderTicket(_) => SubmitIntentKind::Order,
            Self::Cancel(_) => SubmitIntentKind::Cancel,
            Self::Transfer(_) => SubmitIntentKind::Transfer,
            Self::FuturesState(_) => SubmitIntentKind::FuturesState,
            #[cfg(test)]
            Self::Text { intent_kind, .. } => *intent_kind,
        }
    }

    pub fn kind_label(&self) -> &'static str {
        match self {
            Self::OrderTicket(_) => "order",
            Self::Cancel(_) => "cancel",
            Self::Transfer(_) => "transfer",
            Self::FuturesState(_) => "futures-state",
            #[cfg(test)]
            Self::Text { .. } => "text",
        }
    }

    pub fn summary(&self) -> String {
        match self {
            Self::OrderTicket(review) => review.summary(),
            Self::Cancel(review) => review.summary(),
            Self::Transfer(review) => review.summary(),
            Self::FuturesState(review) => review.summary(),
            #[cfg(test)]
            Self::Text { summary, .. } => summary.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum StagedChangeSubject {
    OrderTicket(OrderTicketReview),
    Cancel(CancelReview),
    Transfer(TransferReview),
    FuturesState(FuturesStateReview),
    ProfileRisk(ProfileRiskReview),
    #[cfg(test)]
    Text {
        intent_kind: SubmitIntentKind,
        summary: String,
    },
}

impl StagedChangeSubject {
    pub fn kind(&self) -> StagedChangeKind {
        match self {
            Self::OrderTicket(_) => StagedChangeKind::Order,
            Self::Cancel(_) => StagedChangeKind::Cancel,
            Self::Transfer(_) => StagedChangeKind::Transfer,
            Self::FuturesState(_) => StagedChangeKind::FuturesState,
            Self::ProfileRisk(_) => StagedChangeKind::ProfileRisk,
            #[cfg(test)]
            Self::Text { .. } => StagedChangeKind::Text,
        }
    }

    pub fn submit_intent_kind(&self) -> Option<SubmitIntentKind> {
        match self {
            Self::OrderTicket(_) => Some(SubmitIntentKind::Order),
            Self::Cancel(_) => Some(SubmitIntentKind::Cancel),
            Self::Transfer(_) => Some(SubmitIntentKind::Transfer),
            Self::FuturesState(_) => Some(SubmitIntentKind::FuturesState),
            Self::ProfileRisk(_) => None,
            #[cfg(test)]
            Self::Text { intent_kind, .. } => Some(*intent_kind),
        }
    }

    pub fn summary(&self) -> String {
        match self {
            Self::OrderTicket(review) => review.summary(),
            Self::Cancel(review) => review.summary(),
            Self::Transfer(review) => review.summary(),
            Self::FuturesState(review) => review.summary(),
            Self::ProfileRisk(review) => review.summary(),
            #[cfg(test)]
            Self::Text { summary, .. } => summary.clone(),
        }
    }

    pub fn kind_label(&self) -> &'static str {
        self.kind().label()
    }

    pub fn submit_request(&self, id: String, mode: SubmitMode) -> Option<StagedSubmitRequest> {
        let subject = match self {
            Self::OrderTicket(review) => StagedSubmitSubject::OrderTicket(review.clone()),
            Self::Cancel(review) => StagedSubmitSubject::Cancel(review.clone()),
            Self::Transfer(review) => StagedSubmitSubject::Transfer(review.clone()),
            Self::FuturesState(review) => StagedSubmitSubject::FuturesState(review.clone()),
            Self::ProfileRisk(_) => return None,
            #[cfg(test)]
            Self::Text {
                intent_kind,
                summary,
            } => StagedSubmitSubject::Text {
                intent_kind: *intent_kind,
                summary: summary.clone(),
            },
        };
        Some(StagedSubmitRequest { id, subject, mode })
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum StagedChangeKind {
    Order,
    Cancel,
    Transfer,
    FuturesState,
    ProfileRisk,
    #[cfg(test)]
    Text,
}

impl StagedChangeKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Order => "order",
            Self::Cancel => "cancel",
            Self::Transfer => "transfer",
            Self::FuturesState => "futures-state",
            Self::ProfileRisk => "profile-risk",
            #[cfg(test)]
            Self::Text => "text",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ProfileRiskReview {
    pub profile: String,
    pub path: PathBuf,
    pub change: ProfileRiskChange,
    pub diff: Vec<String>,
    pub checks: Vec<DiagnosticCheck>,
    pub required_failure_count: usize,
}

impl ProfileRiskReview {
    pub fn allow_live_toggle(
        profile: &str,
        path: PathBuf,
        profile_config: &agent_finance_core::Profile,
    ) -> Self {
        let before = profile_config.risk.allow_live;
        let after = !before;
        let mut next_profile = profile_config.clone();
        next_profile.risk.allow_live = after;
        let checks = agent_finance_core::local_profile_checks(&next_profile);
        let required_failure_count = checks
            .iter()
            .filter(|check| check.required && !check.ok)
            .count();

        Self {
            profile: profile.to_string(),
            path,
            change: ProfileRiskChange::AllowLive { before, after },
            diff: vec![format!("risk.allow_live: {before} -> {after}")],
            checks,
            required_failure_count,
        }
    }

    pub fn summary(&self) -> String {
        match self.change {
            ProfileRiskChange::AllowLive { before: _, after } => {
                format!("profile-risk {} risk.allow_live -> {after}", self.profile)
            }
        }
    }

    pub fn target_value(&self) -> bool {
        match self.change {
            ProfileRiskChange::AllowLive { before: _, after } => after,
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(tag = "field", rename_all = "kebab-case")]
pub enum ProfileRiskChange {
    AllowLive { before: bool, after: bool },
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FuturesStateReview {
    pub profile: String,
    pub change: FuturesStateChange,
    pub effective_mode: SubmitMode,
}

impl FuturesStateReview {
    pub fn summary(&self) -> String {
        format!("futures-state {}", self.change.review_label())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct TransferReview {
    pub profile: String,
    pub direction: TransferDirection,
    pub asset: String,
    pub amount: String,
    pub parsed_amount: DecimalValue,
    pub effective_mode: SubmitMode,
}

impl TransferReview {
    pub fn summary(&self) -> String {
        format!("transfer {} {} {}", self.direction, self.amount, self.asset)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CancelReview {
    pub profile: String,
    pub market: Market,
    pub symbol: String,
    pub target: OrderIdentifier,
    pub effective_mode: SubmitMode,
}

impl CancelReview {
    pub fn summary(&self) -> String {
        format!(
            "cancel {} {} [{}]",
            self.market,
            self.symbol,
            self.identifier()
        )
    }

    pub fn identifier(&self) -> String {
        match &self.target {
            OrderIdentifier::OrderId { order_id } => order_id.clone(),
            OrderIdentifier::ClientOrderId { client_order_id } => client_order_id.clone(),
        }
    }

    pub fn target(&self) -> OrderIdentifier {
        self.target.clone()
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct OrderTicketReview {
    pub symbol: String,
    pub profile: String,
    pub market: Market,
    pub side: OrderSide,
    pub kind: OrderKind,
    pub quantity: String,
    pub price: Option<String>,
    pub time_in_force: TimeInForce,
    pub reduce_only: bool,
    pub parsed_quantity: DecimalValue,
    pub order_spec: OrderSpec,
    pub effective_mode: SubmitMode,
}

impl OrderTicketReview {
    pub fn summary(&self) -> String {
        format!(
            "{} {} {} {} {} @ {} {}{}",
            self.side,
            self.quantity,
            self.symbol,
            self.market,
            self.kind,
            self.price.as_deref().unwrap_or("market"),
            self.time_in_force,
            if self.reduce_only { " reduce-only" } else { "" }
        )
    }
}
