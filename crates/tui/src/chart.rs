use std::fmt;
use std::str::FromStr;

use agent_finance_market::args::HistorySession;
use agent_finance_market::is_likely_crypto_pair;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub mod series;

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub enum ChartPreset {
    #[default]
    Auto,
    OneDay,
    FiveDays,
    OneMonth,
    ThreeMonths,
    SixMonths,
    OneYear,
}

impl ChartPreset {
    pub const ALL: [Self; 7] = [
        Self::Auto,
        Self::OneDay,
        Self::FiveDays,
        Self::OneMonth,
        Self::ThreeMonths,
        Self::SixMonths,
        Self::OneYear,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::OneDay => "1d",
            Self::FiveDays => "5d",
            Self::OneMonth => "1mo",
            Self::ThreeMonths => "3mo",
            Self::SixMonths => "6mo",
            Self::OneYear => "1y",
        }
    }

    pub const fn key(self) -> char {
        match self {
            Self::Auto => '0',
            Self::OneDay => '1',
            Self::FiveDays => '2',
            Self::OneMonth => '3',
            Self::ThreeMonths => '4',
            Self::SixMonths => '5',
            Self::OneYear => '6',
        }
    }

    pub fn from_key(key: char) -> Option<Self> {
        Self::ALL.into_iter().find(|preset| preset.key() == key)
    }

    pub fn command_id(self) -> String {
        format!("chart-preset-{}", self.label())
    }

    pub fn command_title(self) -> String {
        format!("Chart preset {}", self.label().to_ascii_uppercase())
    }

    pub const fn command_description(self) -> &'static str {
        match self {
            Self::Auto => "Use the asset-aware default chart range and interval",
            Self::OneDay => "Show one trading day on the history chart",
            Self::FiveDays => "Show five trading days on the history chart",
            Self::OneMonth => "Show one month on the history chart",
            Self::ThreeMonths => "Show three months on the history chart",
            Self::SixMonths => "Show six months on the history chart",
            Self::OneYear => "Show one year on the history chart",
        }
    }

    pub fn shift(self, direction: isize) -> Self {
        let index = Self::ALL
            .iter()
            .position(|preset| *preset == self)
            .unwrap_or_default() as isize;
        let next = (index + direction).rem_euclid(Self::ALL.len() as isize) as usize;
        Self::ALL[next]
    }

    pub fn request_for(self, symbol: &str) -> ChartHistoryRequest {
        let crypto = is_likely_crypto_pair(symbol);
        match (self, crypto) {
            (Self::Auto, true) => ChartHistoryRequest::new("1d", "1m", 288),
            (Self::Auto, false) => {
                ChartHistoryRequest::new("5d", "5m", 1_000).with_session(HistorySession::Extended)
            }
            (Self::OneDay, true) => ChartHistoryRequest::new("1d", "1m", 288),
            (Self::OneDay, false) => {
                ChartHistoryRequest::new("1d", "1m", 960).with_session(HistorySession::Extended)
            }
            (Self::FiveDays, true) => ChartHistoryRequest::new("5d", "5m", 288),
            (Self::FiveDays, false) => {
                ChartHistoryRequest::new("5d", "5m", 1_000).with_session(HistorySession::Extended)
            }
            (Self::OneMonth, _) => ChartHistoryRequest::new("1mo", "1d", 31),
            (Self::ThreeMonths, _) => ChartHistoryRequest::new("3mo", "1d", 66),
            (Self::SixMonths, _) => ChartHistoryRequest::new("6mo", "1d", 132),
            (Self::OneYear, _) => ChartHistoryRequest::new("1y", "1d", 252),
        }
    }
}

impl fmt::Display for ChartPreset {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.label())
    }
}

impl FromStr for ChartPreset {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "1d" | "day" => Ok(Self::OneDay),
            "5d" | "5day" | "week" => Ok(Self::FiveDays),
            "1mo" | "1m" | "month" => Ok(Self::OneMonth),
            "3mo" | "3m" => Ok(Self::ThreeMonths),
            "6mo" | "6m" => Ok(Self::SixMonths),
            "1y" | "1yr" | "year" => Ok(Self::OneYear),
            _ => Err(format!("unknown chart preset {value}")),
        }
    }
}

impl Serialize for ChartPreset {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.label())
    }
}

impl<'de> Deserialize<'de> for ChartPreset {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ChartHistoryRequest {
    pub range: String,
    pub interval: String,
    pub limit: usize,
    pub session: HistorySession,
}

impl ChartHistoryRequest {
    fn new(range: &str, interval: &str, limit: usize) -> Self {
        Self {
            range: range.to_string(),
            interval: interval.to_string(),
            limit,
            session: HistorySession::Regular,
        }
    }

    const fn with_session(mut self, session: HistorySession) -> Self {
        self.session = session;
        self
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ChartState {
    preset: ChartPreset,
}

impl ChartState {
    pub const fn new(preset: ChartPreset) -> Self {
        Self { preset }
    }

    pub const fn preset(&self) -> ChartPreset {
        self.preset
    }

    pub fn set_preset(&mut self, preset: ChartPreset) -> bool {
        let changed = self.preset != preset;
        self.preset = preset;
        changed
    }

    pub fn shift_preset(&mut self, direction: isize) -> bool {
        self.set_preset(self.preset.shift(direction))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_chart_presets_to_asset_aware_requests() {
        let equity = ChartPreset::Auto.request_for("CRDO");
        assert_eq!(equity.range, "5d");
        assert_eq!(equity.interval, "5m");
        assert_eq!(equity.limit, 1_000);
        assert_eq!(equity.session, HistorySession::Extended);

        let day = ChartPreset::OneDay.request_for("CRDO");
        assert_eq!(day.range, "1d");
        assert_eq!(day.interval, "1m");
        assert_eq!(day.limit, 960);
        assert_eq!(day.session, HistorySession::Extended);

        let crypto = ChartPreset::Auto.request_for("BTCUSDT");
        assert_eq!(crypto.range, "1d");
        assert_eq!(crypto.interval, "1m");
        assert_eq!(crypto.session, HistorySession::Regular);

        let long = ChartPreset::OneYear.request_for("CRDO");
        assert_eq!(long.range, "1y");
        assert_eq!(long.interval, "1d");
        assert_eq!(long.limit, 252);
    }

    #[test]
    fn exposes_preset_keys_for_input_mapping() {
        for preset in ChartPreset::ALL {
            assert_eq!(ChartPreset::from_key(preset.key()), Some(preset));
        }
        assert_eq!(ChartPreset::from_key('7'), None);
    }
}
