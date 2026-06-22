use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Command;
use std::thread;

const SYMBOL: &str = "BTCUSDT";

#[test]
fn crypto_watch_json_reports_errors_and_fails() {
    let base_url = one_shot_error_server();
    let output = Command::new(env!("CARGO_BIN_EXE_agent-finance"))
        .env("BINANCE_SPOT_BASE_URL", base_url)
        .args([
            "--no-proxy",
            "watch",
            "BAD",
            "--asset",
            "crypto",
            "--iterations",
            "1",
            "--json",
        ])
        .output()
        .expect("agent-finance command should start");

    assert!(
        !output.status.success(),
        "watch should fail when every crypto quote fails: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("watch should print JSON");
    assert!(
        json["points"].as_array().unwrap().is_empty(),
        "failed watch should not fabricate price points"
    );
    assert!(
        json["errors"]["BAD"]
            .as_str()
            .unwrap()
            .contains("status=400"),
        "failed watch should expose the provider error: {json}"
    );
}

#[test]
#[ignore = "requires AGENT_FINANCE_LIVE_BINANCE=1 and live Binance network access"]
fn binance_live_cli_surface_is_usable() {
    if std::env::var("AGENT_FINANCE_LIVE_BINANCE").ok().as_deref() != Some("1") {
        eprintln!("skipping live Binance test; set AGENT_FINANCE_LIVE_BINANCE=1");
        return;
    }

    assert_aggregate(
        command(&["crypto", "snapshot", SYMBOL, "--json"]),
        "spot.ticker",
    );
    assert_aggregate(command(&["crypto", "sentiment", SYMBOL, "--json"]), "mark");

    for args in [
        &[
            "crypto",
            "quote",
            SYMBOL,
            "--provider",
            "binance",
            "--instrument",
            "spot",
            "--json",
        ][..],
        &[
            "crypto",
            "book",
            SYMBOL,
            "--provider",
            "binance",
            "--instrument",
            "spot",
            "--limit",
            "5",
            "--json",
        ],
        &[
            "crypto",
            "trades",
            SYMBOL,
            "--provider",
            "binance",
            "--instrument",
            "spot",
            "--limit",
            "2",
            "--json",
        ],
        &[
            "crypto",
            "candles",
            SYMBOL,
            "--provider",
            "binance",
            "--instrument",
            "spot",
            "--interval",
            "1m",
            "--limit",
            "2",
            "--json",
        ],
        &[
            "crypto",
            "quote",
            SYMBOL,
            "--provider",
            "binance",
            "--instrument",
            "swap",
            "--json",
        ],
        &[
            "crypto",
            "book",
            SYMBOL,
            "--provider",
            "binance",
            "--instrument",
            "swap",
            "--limit",
            "5",
            "--json",
        ],
        &[
            "crypto",
            "trades",
            SYMBOL,
            "--provider",
            "binance",
            "--instrument",
            "swap",
            "--limit",
            "2",
            "--json",
        ],
        &[
            "crypto",
            "candles",
            SYMBOL,
            "--provider",
            "binance",
            "--instrument",
            "swap",
            "--interval",
            "1m",
            "--limit",
            "2",
            "--json",
        ],
        &[
            "crypto",
            "funding",
            SYMBOL,
            "--provider",
            "binance",
            "--instrument",
            "swap",
            "--limit",
            "2",
            "--json",
        ],
        &[
            "crypto",
            "open-interest",
            SYMBOL,
            "--provider",
            "binance",
            "--instrument",
            "swap",
            "--json",
        ],
    ] {
        assert_evidence(command(args), "binance");
    }

    assert_stream(command(&[
        "crypto",
        "stream",
        SYMBOL,
        "--kind",
        "trade",
        "--messages",
        "1",
        "--json",
    ]));
    assert_stream(command(&[
        "crypto",
        "stream",
        SYMBOL,
        "--instrument",
        "swap",
        "--kind",
        "mark-price",
        "--messages",
        "1",
        "--json",
    ]));
    assert_price(command(&["price", SYMBOL, "--asset", "crypto", "--json"]));
    assert_history(command(&[
        "history",
        SYMBOL,
        "--asset",
        "crypto",
        "--interval",
        "1m",
        "--limit",
        "2",
        "--json",
    ]));
}

#[test]
#[ignore = "requires AGENT_FINANCE_LIVE_CRYPTO_PROVIDERS=1 and live Coinbase/OKX/CoinGecko network access"]
fn crypto_provider_live_cli_surface_is_usable() {
    if std::env::var("AGENT_FINANCE_LIVE_CRYPTO_PROVIDERS")
        .ok()
        .as_deref()
        != Some("1")
    {
        eprintln!(
            "skipping live multi-provider crypto test; set AGENT_FINANCE_LIVE_CRYPTO_PROVIDERS=1"
        );
        return;
    }

    assert_evidence(
        command(&[
            "crypto",
            "quote",
            "BTC-USD",
            "--provider",
            "coinbase",
            "--instrument",
            "spot",
            "--json",
        ]),
        "coinbase",
    );
    assert_evidence(
        command(&[
            "crypto",
            "quote",
            "BTC/USDT",
            "--provider",
            "okx",
            "--instrument",
            "swap",
            "--json",
        ]),
        "okx",
    );
    assert_evidence(
        command(&[
            "crypto",
            "quote",
            "bitcoin",
            "--provider",
            "coingecko",
            "--instrument",
            "spot",
            "--json",
        ]),
        "coingecko",
    );

    assert_payload_len_at_most(
        command(&[
            "crypto",
            "candles",
            "BTC-USD",
            "--provider",
            "coinbase",
            "--instrument",
            "spot",
            "--interval",
            "1m",
            "--limit",
            "2",
            "--json",
        ]),
        "candles",
        2,
    );
    assert_payload_len_at_most(
        command(&[
            "crypto",
            "candles",
            "bitcoin",
            "--provider",
            "coingecko",
            "--instrument",
            "spot",
            "--interval",
            "1d",
            "--limit",
            "2",
            "--json",
        ]),
        "ohlc",
        2,
    );

    let human = command_text(&["crypto", "quote", "BTC-USD", "--provider", "coinbase"]);
    assert!(
        human.lines().count() < 40,
        "human output should summarize payloads instead of dumping JSON: {human}"
    );
    assert!(
        human.contains("payload: object fields="),
        "human output should describe payload shape: {human}"
    );

    let raw = command_text(&[
        "crypto",
        "quote",
        "BTC-USD",
        "--provider",
        "coinbase",
        "--raw",
    ]);
    assert!(
        raw.lines().count() > human.lines().count(),
        "raw output should include provider payloads"
    );
}

fn command(args: &[&str]) -> serde_json::Value {
    let output = Command::new(env!("CARGO_BIN_EXE_agent-finance"))
        .args(args)
        .output()
        .expect("agent-finance command should start");
    assert!(
        output.status.success(),
        "command failed: args={args:?} stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("command should print JSON")
}

fn assert_aggregate(json: serde_json::Value, required_key: &str) {
    assert_eq!(json["symbol"], SYMBOL);
    assert!(
        json["spot"].get(required_key).is_some() || json["futures"].get(required_key).is_some(),
        "aggregate should include required key {required_key}: {json}"
    );
}

fn assert_evidence(json: serde_json::Value, provider: &str) {
    let results = json["results"].as_array().unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["provider"], provider);
    assert!(
        results[0]["ok"].as_bool().unwrap(),
        "provider evidence should be successful: {json}"
    );
}

fn assert_payload_len_at_most(json: serde_json::Value, endpoint: &str, limit: usize) {
    let endpoints = json["results"][0]["endpoints"].as_array().unwrap();
    let payload = endpoints
        .iter()
        .find(|value| value["endpoint"] == endpoint)
        .and_then(|value| value["payload"].as_array())
        .unwrap_or_else(|| panic!("missing array payload for endpoint {endpoint}: {json}"));
    assert!(
        payload.len() <= limit,
        "payload should honor limit={limit}: {json}"
    );
}

fn assert_stream(json: serde_json::Value) {
    assert_eq!(json["symbol"], SYMBOL);
    assert!(
        !json["messages"].as_array().unwrap().is_empty(),
        "stream should contain at least one message"
    );
}

fn assert_price(json: serde_json::Value) {
    assert!(json["errors"].as_object().unwrap().is_empty());
    let quote = &json["points"].as_array().unwrap()[0];
    assert_eq!(quote["symbol"], SYMBOL);
    assert_eq!(quote["provider"], "binance-spot");
    assert!(quote["price"].as_f64().unwrap() > 0.0);
}

fn assert_history(json: serde_json::Value) {
    assert_eq!(json["symbol"], SYMBOL);
    assert_eq!(json["provider"], "binance-spot");
    assert_eq!(json["bars"].as_array().unwrap().len(), 2);
}

fn command_text(args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_agent-finance"))
        .args(args)
        .output()
        .expect("agent-finance command should start");
    assert!(
        output.status.success(),
        "command failed: args={args:?} stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("command should print UTF-8")
}

fn one_shot_error_server() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buffer = [0; 4096];
        let read = stream.read(&mut buffer).unwrap();
        let request = String::from_utf8_lossy(&buffer[..read]);
        assert!(
            request.starts_with("GET /api/v3/ticker/price?symbol=BAD "),
            "request was {request:?}"
        );
        let body = r#"{"code":-1121,"msg":"Invalid symbol."}"#;
        let response = format!(
            "HTTP/1.1 400 Bad Request\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(response.as_bytes()).unwrap();
    });
    format!("http://{address}")
}
