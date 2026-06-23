use agent_finance_core::{Capability, ProviderCapability};

pub fn profile_template(name: &str) -> String {
    format!(
        r#"name = "{name}"

[provider]
provider = "binance"
environment = "testnet"
api_key_env = "BINANCE_API_KEY"
api_secret_env = "BINANCE_API_SECRET"
spot_base_url = "https://testnet.binance.vision"
usds_futures_base_url = "https://testnet.binancefuture.com"

[risk]
allow_live = false
allowed_transfers = []

[risk.allowed_symbols.BTCUSDT]
markets = ["spot", "usds-futures"]
order_kinds = ["market", "limit"]
max_order_notional_usdt = "25"
"#
    )
}

pub fn provider_capability() -> ProviderCapability {
    ProviderCapability::new(
        "binance",
        vec![
            Capability::new(
                "market-data",
                "no-key/read-only",
                strings(["spot", "usds-futures"]),
                strings(["Existing public market-data commands remain available."]),
            ),
            Capability::new(
                "account",
                "signed/read-only",
                strings(["spot", "usds-futures"]),
                strings(["Uses HMAC signed USER_DATA endpoints."]),
            ),
            Capability::new(
                "orders",
                "signed/write-gated",
                strings(["spot", "usds-futures"]),
                strings(["Intent-first; live submit requires profile policy and --live."]),
            ),
            Capability::new(
                "transfers",
                "signed/write-gated",
                strings(["spot<->usds-futures"]),
                strings(["Universal transfer only; withdrawals are intentionally unsupported."]),
            ),
        ],
    )
}

fn strings<const N: usize>(values: [&str; N]) -> Vec<String> {
    values.into_iter().map(str::to_string).collect()
}
