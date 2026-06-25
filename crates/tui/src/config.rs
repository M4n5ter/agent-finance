use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use agent_finance_core::paths;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct TuiLaunch {
    pub symbols: Vec<String>,
    pub config_path: Option<PathBuf>,
    pub no_persist: bool,
    pub tick_rate: Duration,
}

impl TuiLaunch {
    pub fn new(symbols: Vec<String>, config_path: Option<PathBuf>, no_persist: bool) -> Self {
        Self {
            symbols,
            config_path,
            no_persist,
            tick_rate: Duration::from_millis(250),
        }
    }

    pub fn load_config(&self) -> Result<TuiConfig> {
        let config = if let Some(path) = self.config_path.as_deref() {
            TuiConfig::load_from(path)?
        } else {
            default_config_path()
                .filter(|path| path.exists())
                .map(|path| TuiConfig::load_from(&path))
                .transpose()?
                .unwrap_or_default()
        };

        Ok(config)
    }

    pub fn runtime_config(&self, mut config: TuiConfig) -> TuiConfig {
        let symbols = normalize_symbols(&self.symbols);
        if !symbols.is_empty() {
            config.watchlist = symbols;
        }
        config.normalize();
        config
    }

    pub fn persist_config(&self, config: &TuiConfig) -> Result<()> {
        if self.no_persist {
            return Ok(());
        }

        let path = self
            .config_path
            .clone()
            .or_else(default_config_path)
            .context("could not resolve an agent-finance config directory")?;
        config.save_to(&path)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct TuiConfig {
    #[serde(default = "default_watchlist")]
    pub watchlist: Vec<String>,
    #[serde(default)]
    pub layout: LayoutConfig,
    #[serde(default)]
    pub refresh: RefreshConfig,
    #[serde(default)]
    pub providers: ProviderConfig,
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            watchlist: default_watchlist(),
            layout: LayoutConfig::default(),
            refresh: RefreshConfig::default(),
            providers: ProviderConfig::default(),
        }
    }
}

impl TuiConfig {
    pub fn load_from(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("failed to read TUI config {}", path.display()))?;
        let mut config = toml::from_str::<Self>(&content)
            .with_context(|| format!("failed to parse TUI config {}", path.display()))?;
        config.normalize();
        Ok(config)
    }

    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create config directory {}", parent.display())
            })?;
        }
        let content = toml::to_string_pretty(self).context("failed to serialize TUI config")?;
        fs::write(path, content)
            .with_context(|| format!("failed to write TUI config {}", path.display()))
    }

    pub fn normalize(&mut self) {
        self.watchlist = normalize_symbols(&self.watchlist);
        if self.watchlist.is_empty() {
            self.watchlist = default_watchlist();
        }
        self.layout.normalize();
        self.refresh.normalize();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct LayoutConfig {
    #[serde(default = "default_left_ratio")]
    pub left_ratio: u16,
    #[serde(default = "default_main_ratio")]
    pub main_ratio: u16,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            left_ratio: default_left_ratio(),
            main_ratio: default_main_ratio(),
        }
    }
}

impl LayoutConfig {
    fn normalize(&mut self) {
        self.left_ratio = self.left_ratio.clamp(15, 35);
        self.main_ratio = self.main_ratio.clamp(35, 60);
        if self.left_ratio + self.main_ratio > 85 {
            self.main_ratio = 85 - self.left_ratio;
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct RefreshConfig {
    #[serde(default = "default_price_seconds")]
    pub price_seconds: u64,
    #[serde(default = "default_research_seconds")]
    pub research_seconds: u64,
}

impl Default for RefreshConfig {
    fn default() -> Self {
        Self {
            price_seconds: default_price_seconds(),
            research_seconds: default_research_seconds(),
        }
    }
}

impl RefreshConfig {
    fn normalize(&mut self) {
        self.price_seconds = self.price_seconds.clamp(2, 300);
        self.research_seconds = self.research_seconds.clamp(60, 86_400);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct ProviderConfig {
    #[serde(default = "default_equity_provider")]
    pub equity: String,
    #[serde(default = "default_crypto_provider")]
    pub crypto: String,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            equity: default_equity_provider(),
            crypto: default_crypto_provider(),
        }
    }
}

fn default_config_path() -> Option<PathBuf> {
    paths::config_dir().ok().map(|path| path.join("tui.toml"))
}

fn normalize_symbols(symbols: &[String]) -> Vec<String> {
    let mut normalized = Vec::new();
    for symbol in symbols {
        for part in symbol.split(',') {
            let symbol = part.trim().to_ascii_uppercase();
            if !symbol.is_empty() && !normalized.contains(&symbol) {
                normalized.push(symbol);
            }
        }
    }
    normalized
}

fn default_watchlist() -> Vec<String> {
    ["AAPL", "CRDO", "BTCUSDT"]
        .into_iter()
        .map(str::to_string)
        .collect()
}

const fn default_left_ratio() -> u16 {
    24
}

const fn default_main_ratio() -> u16 {
    46
}

const fn default_price_seconds() -> u64 {
    15
}

const fn default_research_seconds() -> u64 {
    900
}

fn default_equity_provider() -> String {
    "auto".to_string()
}

fn default_crypto_provider() -> String {
    "auto".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launch_symbols_override_and_normalize_config_watchlist() {
        let launch = TuiLaunch::new(
            vec![
                " aapl, crdo ".to_string(),
                "AAPL".to_string(),
                "btcusdt".to_string(),
            ],
            None,
            true,
        );
        let config = launch.runtime_config(TuiConfig::default());

        assert_eq!(config.watchlist, ["AAPL", "CRDO", "BTCUSDT"]);
    }

    #[test]
    fn launch_symbols_do_not_mutate_persisted_config() {
        let launch = TuiLaunch::new(vec!["TSLA".to_string()], None, true);
        let persisted = TuiConfig {
            watchlist: vec!["AAPL".to_string(), "CRDO".to_string()],
            ..TuiConfig::default()
        };

        let runtime = launch.runtime_config(persisted.clone());

        assert_eq!(persisted.watchlist, ["AAPL", "CRDO"]);
        assert_eq!(runtime.watchlist, ["TSLA"]);
    }

    #[test]
    fn config_roundtrip_preserves_user_visible_preferences() {
        let mut config = TuiConfig {
            watchlist: vec!["lite".to_string(), "aaoi".to_string()],
            layout: LayoutConfig {
                left_ratio: 8,
                main_ratio: 90,
            },
            refresh: RefreshConfig {
                price_seconds: 1,
                research_seconds: 10,
            },
            providers: ProviderConfig {
                equity: "yahoo".to_string(),
                crypto: "binance".to_string(),
            },
        };
        config.normalize();

        let encoded = toml::to_string(&config).expect("encode");
        let decoded = toml::from_str::<TuiConfig>(&encoded).expect("decode");

        assert_eq!(decoded.watchlist, ["LITE", "AAOI"]);
        assert_eq!(decoded.layout.left_ratio, 15);
        assert_eq!(decoded.layout.main_ratio, 60);
        assert_eq!(decoded.refresh.price_seconds, 2);
        assert_eq!(decoded.refresh.research_seconds, 60);
        assert_eq!(decoded.providers.crypto, "binance");
    }
}
