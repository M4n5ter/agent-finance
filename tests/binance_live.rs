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
        &["crypto", "spot", "exchange-info", SYMBOL, "--json"][..],
        &["crypto", "spot", "ticker", SYMBOL, "--json"],
        &["crypto", "spot", "ticker24h", SYMBOL, "--json"],
        &["crypto", "spot", "avg-price", SYMBOL, "--json"],
        &["crypto", "spot", "book", SYMBOL, "--limit", "5", "--json"],
        &[
            "crypto",
            "spot",
            "trades",
            SYMBOL,
            "--aggregate",
            "--limit",
            "2",
            "--json",
        ],
        &[
            "crypto",
            "spot",
            "klines",
            SYMBOL,
            "--interval",
            "1m",
            "--limit",
            "2",
            "--json",
        ],
        &["crypto", "futures", "exchange-info", "--json"],
        &["crypto", "futures", "ticker", SYMBOL, "--json"],
        &["crypto", "futures", "ticker24h", SYMBOL, "--json"],
        &[
            "crypto", "futures", "book", SYMBOL, "--limit", "5", "--json",
        ],
        &[
            "crypto", "futures", "trades", SYMBOL, "--limit", "2", "--json",
        ],
        &[
            "crypto",
            "futures",
            "klines",
            SYMBOL,
            "--interval",
            "1m",
            "--limit",
            "2",
            "--json",
        ],
        &["crypto", "futures", "mark", SYMBOL, "--json"],
        &[
            "crypto", "futures", "funding", SYMBOL, "--limit", "2", "--json",
        ],
        &["crypto", "futures", "open-interest", SYMBOL, "--json"],
        &[
            "crypto", "futures", "ratios", SYMBOL, "--limit", "2", "--json",
        ],
        &[
            "crypto", "futures", "flow", SYMBOL, "--limit", "2", "--json",
        ],
        &[
            "crypto", "futures", "basis", SYMBOL, "--limit", "2", "--json",
        ],
    ] {
        assert_endpoint(command(args));
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
        "--market",
        "usds-futures",
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

fn assert_endpoint(json: serde_json::Value) {
    assert_eq!(json["status"], 200);
    assert!(json["provider"].as_str().unwrap().starts_with("binance-"));
    assert!(
        !json["payload"].is_null(),
        "endpoint should preserve provider payload"
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
