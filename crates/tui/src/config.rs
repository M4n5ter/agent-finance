use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use agent_finance_core::paths;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::model::{DockedPanels, FloatingPane, FloatingSize, Panel};

pub const MIN_LEFT_RATIO: u16 = 15;
pub const MAX_LEFT_RATIO: u16 = 35;
pub const MIN_MAIN_RATIO: u16 = 35;
pub const MAX_MAIN_RATIO: u16 = 60;
pub const MIN_RIGHT_RATIO: u16 = 20;
pub const MAX_LEFT_MAIN_RATIO: u16 = 100 - MIN_RIGHT_RATIO;

#[derive(Debug, Clone)]
pub struct TuiLaunch {
    pub symbols: Vec<String>,
    pub config_path: Option<PathBuf>,
    pub no_persist: bool,
    pub tick_rate: Duration,
    pub proxy: Option<String>,
    pub no_proxy: bool,
    pub timeout_seconds: u64,
    pub timezone: String,
}

impl TuiLaunch {
    pub fn new(symbols: Vec<String>, config_path: Option<PathBuf>, no_persist: bool) -> Self {
        Self::with_market_runtime(symbols, config_path, no_persist, None, false, 10, "UTC")
    }

    pub fn with_market_runtime(
        symbols: Vec<String>,
        config_path: Option<PathBuf>,
        no_persist: bool,
        proxy: Option<&str>,
        no_proxy: bool,
        timeout_seconds: u64,
        timezone: &str,
    ) -> Self {
        Self {
            symbols,
            config_path,
            no_persist,
            tick_rate: Duration::from_millis(250),
            proxy: proxy.map(ToString::to_string),
            no_proxy,
            timeout_seconds,
            timezone: timezone.to_string(),
        }
    }

    pub fn load_config(&self) -> Result<TuiConfig> {
        let config = if let Some(path) = self.config_path.as_deref() {
            if path.exists() {
                TuiConfig::load_from(path)?
            } else {
                TuiConfig::default()
            }
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
    pub panels: PanelConfig,
    #[serde(default)]
    pub floating: FloatingConfig,
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
            panels: PanelConfig::default(),
            floating: FloatingConfig::default(),
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
        self.panels.normalize();
        self.floating.normalize();
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
    pub fn normalize(&mut self) {
        self.left_ratio = self.left_ratio.clamp(MIN_LEFT_RATIO, MAX_LEFT_RATIO);
        self.main_ratio = self.main_ratio.clamp(MIN_MAIN_RATIO, MAX_MAIN_RATIO);
        if self.left_ratio + self.main_ratio > MAX_LEFT_MAIN_RATIO {
            self.main_ratio = MAX_LEFT_MAIN_RATIO - self.left_ratio;
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct PanelConfig {
    #[serde(default = "default_open_panels")]
    pub open: Vec<Panel>,
    #[serde(default = "default_focused_panel")]
    pub focused: Panel,
}

impl Default for PanelConfig {
    fn default() -> Self {
        Self {
            open: default_open_panels(),
            focused: default_focused_panel(),
        }
    }
}

impl PanelConfig {
    pub fn normalize(&mut self) {
        let (open, focused) =
            DockedPanels::from_open_focused(self.open.clone(), self.focused).into_parts();
        self.open = open;
        self.focused = focused;
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Eq, PartialEq)]
pub struct FloatingConfig {
    #[serde(default)]
    pub panes: Vec<FloatingPane>,
}

impl FloatingConfig {
    pub fn normalize(&mut self) {
        let mut normalized = Vec::new();
        for pane in &self.panes {
            if !pane.kind.persistent() {
                continue;
            }
            if normalized
                .iter()
                .any(|existing: &FloatingPane| existing.kind == pane.kind)
            {
                continue;
            }
            normalized.push(FloatingPane {
                kind: pane.kind,
                size: FloatingSize::resized(pane.size.width_ratio, pane.size.height_ratio),
            });
        }
        self.panes = normalized;
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

fn default_open_panels() -> Vec<Panel> {
    Panel::ALL.to_vec()
}

const fn default_focused_panel() -> Panel {
    Panel::Watchlist
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
    use crate::model::FloatingKind;
    use std::time::{SystemTime, UNIX_EPOCH};

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
            panels: PanelConfig {
                open: vec![Panel::Research, Panel::Watchlist, Panel::Research],
                focused: Panel::ProviderHealth,
            },
            floating: FloatingConfig {
                panes: vec![
                    FloatingPane {
                        kind: FloatingKind::Help,
                        size: FloatingSize::resized(99, 5),
                    },
                    FloatingPane {
                        kind: FloatingKind::Help,
                        size: FloatingSize::resized(40, 40),
                    },
                ],
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
        assert_eq!(decoded.panels.open, [Panel::Watchlist, Panel::Research]);
        assert_eq!(decoded.panels.focused, Panel::Watchlist);
        assert_eq!(decoded.floating.panes.len(), 1);
        assert_eq!(
            decoded.floating.panes[0].size,
            FloatingSize::resized(95, 20)
        );
        assert_eq!(decoded.refresh.price_seconds, 2);
        assert_eq!(decoded.refresh.research_seconds, 60);
        assert_eq!(decoded.providers.crypto, "binance");
    }

    #[test]
    fn no_persist_launch_does_not_write_config_file() {
        let path = unique_temp_config_path("no-persist");
        let launch = TuiLaunch::new(Vec::new(), Some(path.clone()), true);

        launch
            .persist_config(&TuiConfig::default())
            .expect("no-persist should be a no-op");

        assert!(!path.exists());
    }

    #[test]
    fn explicit_missing_config_path_starts_from_default_config() {
        let path = unique_temp_config_path("missing-config");
        let launch = TuiLaunch::new(Vec::new(), Some(path.clone()), true);

        let config = launch.load_config().expect("missing config should default");

        assert_eq!(config, TuiConfig::default());
        assert!(!path.exists());
    }

    #[test]
    fn persist_then_load_roundtrips_runtime_layout_config() {
        let path = unique_temp_config_path("persist-roundtrip");
        let launch = TuiLaunch::new(Vec::new(), Some(path.clone()), false);
        let mut config = TuiConfig {
            watchlist: vec!["crdo".to_string(), "lite".to_string()],
            layout: LayoutConfig {
                left_ratio: 30,
                main_ratio: 42,
            },
            panels: PanelConfig {
                open: vec![Panel::Watchlist, Panel::History],
                focused: Panel::History,
            },
            floating: FloatingConfig {
                panes: vec![
                    FloatingPane {
                        kind: FloatingKind::CommandPalette,
                        size: FloatingSize::resized(70, 40),
                    },
                    FloatingPane {
                        kind: FloatingKind::ProviderDetails,
                        size: FloatingSize::resized(61, 62),
                    },
                ],
            },
            ..TuiConfig::default()
        };
        config.normalize();

        launch.persist_config(&config).expect("persist config");
        let loaded = launch.load_config().expect("load config");
        let _ = fs::remove_file(&path);

        assert_eq!(loaded.watchlist, ["CRDO", "LITE"]);
        assert_eq!(loaded.layout.left_ratio, 30);
        assert_eq!(loaded.layout.main_ratio, 42);
        assert_eq!(loaded.panels.open, [Panel::Watchlist, Panel::History]);
        assert_eq!(loaded.panels.focused, Panel::History);
        assert_eq!(loaded.floating.panes.len(), 1);
        assert_eq!(loaded.floating.panes[0].kind, FloatingKind::ProviderDetails);
        assert_eq!(loaded.floating.panes[0].size, FloatingSize::resized(61, 62));
    }

    fn unique_temp_config_path(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("agent-finance-tui-{name}-{nanos}.toml"))
    }
}
