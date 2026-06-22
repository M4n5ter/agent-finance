use clap::{Parser, Subcommand, ValueEnum};

const CRYPTO_INTERVAL_HELP: &str = "Crypto candle interval. Binance: 1m/3m/5m/15m/30m/1h/2h/4h/6h/8h/12h/1d/3d/1w/1M; Coinbase: 1m/5m/15m/1h/6h/1d; OKX: 1m/3m/5m/15m/30m/1h/2h/4h/6h/12h/1d/2d/3d.";

#[derive(Parser, Debug)]
pub struct CryptoArgs {
    #[command(subcommand)]
    pub command: CryptoCommand,
}

#[derive(Subcommand, Debug)]
pub enum CryptoCommand {
    /// Aggregate Binance spot and USD-M futures state for one symbol.
    Snapshot(CryptoSymbolArgs),
    /// Aggregate Binance USD-M funding, open interest, long/short, taker flow, and basis signals.
    Sentiment(CryptoSymbolArgs),
    /// Stream selected Binance WebSocket market events.
    Stream(CryptoStreamArgs),
    /// Fetch quote evidence across Binance, Coinbase, OKX, and CoinGecko.
    Quote(CryptoEvidenceSymbolArgs),
    /// Fetch order-book depth across providers where available.
    Book(CryptoEvidenceBookArgs),
    /// Fetch recent trade evidence across providers where available.
    Trades(CryptoEvidenceTradesArgs),
    /// Fetch OHLCV or OHLC candle evidence across providers where available.
    Candles(CryptoEvidenceKlinesArgs),
    /// Fetch derivatives funding-rate evidence across providers where available.
    Funding(CryptoEvidenceFundingArgs),
    /// Fetch derivatives open-interest evidence across providers where available.
    OpenInterest(CryptoEvidenceOpenInterestArgs),
    /// Discover provider markets, metadata, trending, global, or exchange data.
    Discover(CryptoDiscoverArgs),
}

#[derive(Parser, Debug)]
pub struct CryptoSymbolArgs {
    pub symbol: String,

    #[arg(long)]
    pub raw: bool,

    #[arg(long)]
    pub json: bool,
}

#[derive(Parser, Debug)]
pub struct CryptoStreamArgs {
    pub symbol: String,

    #[arg(long, value_enum, default_value_t = CryptoInstrument::Auto)]
    pub instrument: CryptoInstrument,

    #[arg(long, value_enum, default_value_t = CryptoStreamKind::Trade)]
    pub kind: CryptoStreamKind,

    #[arg(long, default_value = "1m", help = CRYPTO_INTERVAL_HELP)]
    pub interval: String,

    #[arg(long, default_value_t = 5)]
    pub messages: usize,

    #[arg(long)]
    pub json: bool,
}

#[derive(Parser, Debug)]
pub struct CryptoEvidenceSymbolArgs {
    pub symbol: String,

    #[arg(long, value_enum, default_value_t = CryptoProvider::Auto)]
    pub provider: CryptoProvider,

    #[arg(long, value_enum, default_value_t = CryptoInstrument::Auto)]
    pub instrument: CryptoInstrument,

    #[arg(long)]
    pub raw: bool,

    #[arg(long)]
    pub json: bool,
}

#[derive(Parser, Debug)]
pub struct CryptoEvidenceBookArgs {
    pub symbol: String,

    #[arg(long, value_enum, default_value_t = CryptoProvider::Auto)]
    pub provider: CryptoProvider,

    #[arg(long, value_enum, default_value_t = CryptoInstrument::Auto)]
    pub instrument: CryptoInstrument,

    #[arg(long, default_value_t = 20)]
    pub limit: usize,

    #[arg(long)]
    pub raw: bool,

    #[arg(long)]
    pub json: bool,
}

#[derive(Parser, Debug)]
pub struct CryptoEvidenceTradesArgs {
    pub symbol: String,

    #[arg(long, value_enum, default_value_t = CryptoProvider::Auto)]
    pub provider: CryptoProvider,

    #[arg(long, value_enum, default_value_t = CryptoInstrument::Auto)]
    pub instrument: CryptoInstrument,

    #[arg(long, default_value_t = 20)]
    pub limit: usize,

    #[arg(long)]
    pub aggregate: bool,

    #[arg(long)]
    pub raw: bool,

    #[arg(long)]
    pub json: bool,
}

#[derive(Parser, Debug)]
pub struct CryptoEvidenceKlinesArgs {
    pub symbol: String,

    #[arg(long, value_enum, default_value_t = CryptoProvider::Auto)]
    pub provider: CryptoProvider,

    #[arg(long, value_enum, default_value_t = CryptoInstrument::Auto)]
    pub instrument: CryptoInstrument,

    #[arg(long, default_value = "1m", help = CRYPTO_INTERVAL_HELP)]
    pub interval: String,

    #[arg(long, default_value_t = 60)]
    pub limit: usize,

    #[arg(long)]
    pub raw: bool,

    #[arg(long)]
    pub json: bool,
}

#[derive(Parser, Debug)]
pub struct CryptoEvidenceFundingArgs {
    pub symbol: String,

    #[arg(long, value_enum, default_value_t = CryptoProvider::Auto)]
    pub provider: CryptoProvider,

    #[arg(long, value_enum, default_value_t = CryptoInstrument::Auto)]
    pub instrument: CryptoInstrument,

    #[arg(long, default_value_t = 8)]
    pub limit: usize,

    #[arg(long)]
    pub raw: bool,

    #[arg(long)]
    pub json: bool,
}

#[derive(Parser, Debug)]
pub struct CryptoEvidenceOpenInterestArgs {
    pub symbol: String,

    #[arg(long, value_enum, default_value_t = CryptoProvider::Auto)]
    pub provider: CryptoProvider,

    #[arg(long, value_enum, default_value_t = CryptoInstrument::Auto)]
    pub instrument: CryptoInstrument,

    #[arg(long)]
    pub raw: bool,

    #[arg(long)]
    pub json: bool,
}

#[derive(Parser, Debug)]
pub struct CryptoDiscoverArgs {
    #[arg(long, value_enum, default_value_t = CryptoProvider::Auto)]
    pub provider: CryptoProvider,

    #[arg(long, value_enum, default_value_t = CryptoDiscoverKind::Markets)]
    pub kind: CryptoDiscoverKind,

    #[arg(long, value_enum, default_value_t = CryptoInstrument::Auto)]
    pub instrument: CryptoInstrument,

    #[arg(long, default_value = "usd")]
    pub vs_currency: String,

    #[arg(long, default_value_t = 100)]
    pub limit: usize,

    #[arg(long)]
    pub raw: bool,

    #[arg(long)]
    pub json: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum CryptoDiscoverKind {
    Markets,
    Instruments,
    Tickers,
    Trending,
    Global,
    Exchanges,
    Derivatives,
    DerivativesExchanges,
    VolumeSummary,
    CoinsList,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum CryptoProvider {
    Auto,
    Binance,
    Coinbase,
    Okx,
    Coingecko,
}

impl CryptoProvider {
    pub const fn label(self) -> &'static str {
        match self {
            CryptoProvider::Auto => "auto",
            CryptoProvider::Binance => "binance",
            CryptoProvider::Coinbase => "coinbase",
            CryptoProvider::Okx => "okx",
            CryptoProvider::Coingecko => "coingecko",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum CryptoMarket {
    Auto,
    Spot,
    UsdsFutures,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum CryptoInstrument {
    Auto,
    Spot,
    Swap,
    Futures,
    Option,
}

impl CryptoInstrument {
    pub const fn label(self) -> &'static str {
        match self {
            CryptoInstrument::Auto => "auto",
            CryptoInstrument::Spot => "spot",
            CryptoInstrument::Swap => "swap",
            CryptoInstrument::Futures => "futures",
            CryptoInstrument::Option => "option",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum CryptoStreamKind {
    Trade,
    Kline,
    Depth,
    BookTicker,
    MarkPrice,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum FuturesPeriod {
    #[value(name = "5m")]
    FiveMin,
    #[value(name = "15m")]
    FifteenMin,
    #[value(name = "30m")]
    ThirtyMin,
    #[value(name = "1h")]
    OneHour,
    #[value(name = "2h")]
    TwoHour,
    #[value(name = "4h")]
    FourHour,
    #[value(name = "6h")]
    SixHour,
    #[value(name = "12h")]
    TwelveHour,
    #[value(name = "1d")]
    OneDay,
}
