use clap::{Parser, Subcommand, ValueEnum};

const CRYPTO_INTERVAL_HELP: &str =
    "Binance kline interval. Values: 1m/3m/5m/15m/30m/1h/2h/4h/6h/8h/12h/1d/3d/1w/1M.";

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
    /// Inspect Binance Spot public market data.
    Spot(CryptoSpotArgs),
    /// Inspect Binance USD-M Futures public market data.
    Futures(CryptoFuturesArgs),
    /// Stream selected Binance WebSocket market events.
    Stream(CryptoStreamArgs),
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
pub struct CryptoSpotArgs {
    #[command(subcommand)]
    pub command: CryptoSpotCommand,
}

#[derive(Subcommand, Debug)]
pub enum CryptoSpotCommand {
    /// Binance Spot exchange information and symbol filters.
    ExchangeInfo(CryptoExchangeInfoArgs),
    /// Latest Binance Spot price ticker.
    Ticker(CryptoSymbolArgs),
    /// Binance Spot 24h ticker statistics.
    Ticker24h(CryptoSymbolArgs),
    /// Binance Spot current average price.
    AvgPrice(CryptoSymbolArgs),
    /// Binance Spot order book snapshot.
    Book(CryptoBookArgs),
    /// Binance Spot recent or aggregate trades.
    Trades(CryptoTradesArgs),
    /// Binance Spot OHLCV klines.
    Klines(CryptoKlinesArgs),
}

#[derive(Parser, Debug)]
pub struct CryptoFuturesArgs {
    #[command(subcommand)]
    pub command: CryptoFuturesCommand,
}

#[derive(Subcommand, Debug)]
pub enum CryptoFuturesCommand {
    /// Binance USD-M Futures exchange information and symbol filters.
    ExchangeInfo(CryptoRawArgs),
    /// Latest Binance USD-M Futures price ticker.
    Ticker(CryptoSymbolArgs),
    /// Binance USD-M Futures 24h ticker statistics.
    Ticker24h(CryptoSymbolArgs),
    /// Binance USD-M Futures order book snapshot.
    Book(CryptoBookArgs),
    /// Binance USD-M Futures aggregate trades.
    Trades(CryptoTradesArgs),
    /// Binance USD-M Futures OHLCV klines.
    Klines(CryptoKlinesArgs),
    /// Binance USD-M mark price, index price, and funding reference.
    Mark(CryptoSymbolArgs),
    /// Binance USD-M funding rate history.
    Funding(CryptoLimitArgs),
    /// Binance USD-M open interest.
    OpenInterest(CryptoSymbolArgs),
    /// Binance USD-M global and top-trader long/short ratios.
    Ratios(CryptoPeriodArgs),
    /// Binance USD-M taker buy/sell volume.
    Flow(CryptoPeriodArgs),
    /// Binance USD-M futures basis.
    Basis(CryptoPeriodArgs),
}

#[derive(Parser, Debug)]
pub struct CryptoExchangeInfoArgs {
    pub symbol: Option<String>,

    #[arg(long)]
    pub raw: bool,

    #[arg(long)]
    pub json: bool,
}

#[derive(Parser, Debug)]
pub struct CryptoRawArgs {
    #[arg(long)]
    pub raw: bool,

    #[arg(long)]
    pub json: bool,
}

#[derive(Parser, Debug)]
pub struct CryptoBookArgs {
    pub symbol: String,

    #[arg(long, default_value_t = 20)]
    pub limit: usize,

    #[arg(long)]
    pub raw: bool,

    #[arg(long)]
    pub json: bool,
}

#[derive(Parser, Debug)]
pub struct CryptoTradesArgs {
    pub symbol: String,

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
pub struct CryptoKlinesArgs {
    pub symbol: String,

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
pub struct CryptoLimitArgs {
    pub symbol: String,

    #[arg(long, default_value_t = 8)]
    pub limit: usize,

    #[arg(long)]
    pub raw: bool,

    #[arg(long)]
    pub json: bool,
}

#[derive(Parser, Debug)]
pub struct CryptoPeriodArgs {
    pub symbol: String,

    #[arg(long, value_enum, default_value_t = FuturesPeriod::FiveMin)]
    pub period: FuturesPeriod,

    #[arg(long, default_value_t = 30)]
    pub limit: usize,

    #[arg(long)]
    pub raw: bool,

    #[arg(long)]
    pub json: bool,
}

#[derive(Parser, Debug)]
pub struct CryptoStreamArgs {
    pub symbol: String,

    #[arg(long, value_enum, default_value_t = CryptoMarket::Auto)]
    pub market: CryptoMarket,

    #[arg(long, value_enum, default_value_t = CryptoStreamKind::Trade)]
    pub kind: CryptoStreamKind,

    #[arg(long, default_value = "1m", help = CRYPTO_INTERVAL_HELP)]
    pub interval: String,

    #[arg(long, default_value_t = 5)]
    pub messages: usize,

    #[arg(long)]
    pub json: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum CryptoMarket {
    Auto,
    Spot,
    UsdsFutures,
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
