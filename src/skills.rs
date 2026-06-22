pub struct BuiltinSkill {
    pub name: &'static str,
    pub description: &'static str,
}

const SKILLS: &[BuiltinSkill] = &[
    BuiltinSkill {
        name: "core",
        description: "Entry guide for price, sessions, history, research, providers, proxy data, and safe source handling.",
    },
    BuiltinSkill {
        name: "price",
        description: "Fetch current price summaries, regular-market basis, pre/post/overnight sessions, streams, and proxy prices.",
    },
    BuiltinSkill {
        name: "research-data",
        description: "Fetch no-key Yahoo/SEC EDGAR/Robinhood/CNBC research data and read URL text with fallback readers.",
    },
    BuiltinSkill {
        name: "providers",
        description: "Understand Yahoo, SEC EDGAR, CNBC, Robinhood, Stooq, Binance futures, Polymarket, and proxy quote capabilities.",
    },
    BuiltinSkill {
        name: "prediction-markets",
        description: "Use Polymarket as quantifiable sentiment and event-probability evidence via official read-only APIs.",
    },
    BuiltinSkill {
        name: "history-indicators",
        description: "Fetch OHLCV, understand intervals, adjustments, repair, actions, and local technical indicators.",
    },
    BuiltinSkill {
        name: "futures",
        description: "Use Binance USD-M futures / TradFi perps for proxy price, funding, open interest, and mark price.",
    },
];

pub fn print_list() {
    for skill in SKILLS {
        println!("{:<20} {}", skill.name, skill.description);
    }
}

pub fn get(name: &str, full: bool) -> Option<&'static str> {
    match (name, full) {
        ("core", false) => Some(CORE),
        ("core", true) => Some(CORE_FULL),
        ("price", _) => Some(PRICE),
        ("research-data", _) => Some(RESEARCH_DATA),
        ("providers", _) => Some(PROVIDERS),
        ("prediction-markets", _) => Some(PREDICTION_MARKETS),
        ("history-indicators", _) => Some(HISTORY_INDICATORS),
        ("futures", _) => Some(FUTURES),
        _ => None,
    }
}

const CORE: &str = include_str!("../skills/core.md");
const CORE_FULL: &str = include_str!("../skills/core-full.md");
const PRICE: &str = include_str!("../skills/price.md");
const RESEARCH_DATA: &str = include_str!("../skills/research-data.md");
const PROVIDERS: &str = include_str!("../skills/providers.md");
const PREDICTION_MARKETS: &str = include_str!("../skills/prediction-markets.md");
const HISTORY_INDICATORS: &str = include_str!("../skills/history-indicators.md");
const FUTURES: &str = include_str!("../skills/futures.md");
