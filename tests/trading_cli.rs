use std::fs;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

#[test]
fn order_intent_can_be_risk_checked_and_dry_run_repeatedly() {
    let env = default_env("order-flow");
    let order = create_limit_order(&env);
    assert_eq!(order["risk"]["allowed"], true);
    let order_id = order["intent"]["id"].as_str().expect("order intent id");

    let risk = env.json(command(&[
        "risk",
        "check",
        order_id,
        "--profile",
        "default",
        "--json",
    ]));
    assert_eq!(risk["allowed"], true);

    let submit = env.json(command(&[
        "order",
        "submit",
        order_id,
        "--profile",
        "default",
        "--json",
    ]));
    assert_eq!(submit["response"]["dry_run"], true);
    assert_eq!(submit["response"]["request"]["method"], "POST");

    let second_plan = env.json(command(&[
        "order",
        "submit",
        order_id,
        "--profile",
        "default",
        "--json",
    ]));
    assert_eq!(second_plan["response"]["dry_run"], true);

    let audit = env.json(command(&["audit", "tail", "--limit", "10", "--json"]));
    let events = audit.as_array().expect("audit events");
    assert!(
        events.iter().any(|event| event["kind"] == "intent-created"),
        "audit should include intent-created events"
    );
    assert!(
        events.iter().any(|event| event["kind"] == "dry-run"),
        "audit should include dry-run events"
    );
}

#[test]
fn cancel_test_failure_does_not_consume_intent() {
    let env = default_env("cancel-flow");
    let cancel = env.json(command(&[
        "order",
        "cancel-intent",
        "BTCUSDT",
        "--profile",
        "default",
        "--market",
        "spot",
        "--client-order-id",
        "af-test",
        "--json",
    ]));
    assert_eq!(cancel["risk"]["allowed"], true);
    let cancel_id = cancel["intent"]["id"].as_str().expect("cancel intent id");

    let cancel_submit = env.json(command(&[
        "order",
        "submit",
        cancel_id,
        "--profile",
        "default",
        "--json",
    ]));
    assert_eq!(cancel_submit["response"]["request"]["method"], "DELETE");

    let cancel_test = env.output(command(&[
        "order",
        "submit",
        cancel_id,
        "--profile",
        "default",
        "--test",
        "--json",
    ]));
    assert!(
        !cancel_test.status.success(),
        "cancel intent should not have an exchange test mode"
    );
    let cancel_submit_after_test_failure = env.json(command(&[
        "order",
        "submit",
        cancel_id,
        "--profile",
        "default",
        "--json",
    ]));
    assert_eq!(
        cancel_submit_after_test_failure["response"]["request"]["method"],
        "DELETE"
    );
}

#[test]
fn invalid_and_risk_blocked_orders_are_rejected_at_the_right_boundary() {
    let env = default_env("risk-boundaries");
    let blocked = env.output(command(&[
        "order",
        "intent",
        "BTCUSDT",
        "--profile",
        "default",
        "--market",
        "spot",
        "--side",
        "buy",
        "--kind",
        "market",
        "--quantity",
        "1",
        "--valuation-price",
        "50000",
        "--json",
    ]));
    assert!(
        blocked.status.success(),
        "risk-blocked intent can be created"
    );
    let blocked_json: Value = serde_json::from_slice(&blocked.stdout).expect("blocked intent json");
    let blocked_id = blocked_json["intent"]["id"]
        .as_str()
        .expect("blocked intent id");
    let blocked_submit = env.output(command(&[
        "order",
        "submit",
        blocked_id,
        "--profile",
        "default",
        "--json",
    ]));
    assert!(
        !blocked_submit.status.success(),
        "risk-blocked intent should not be submitted even as dry-run"
    );

    let invalid_limit = env.output(command(&[
        "order",
        "intent",
        "BTCUSDT",
        "--profile",
        "default",
        "--market",
        "spot",
        "--side",
        "buy",
        "--kind",
        "limit",
        "--quantity",
        "1",
        "--price",
        "50000",
        "--valuation-price",
        "1",
        "--time-in-force",
        "gtc",
        "--json",
    ]));
    assert!(
        !invalid_limit.status.success(),
        "limit order must not accept a separate valuation price"
    );
}

#[test]
fn market_order_uses_valuation_only_for_risk_and_test_is_non_consuming() {
    let env = default_env("market-order");
    let market = env.json(command(&[
        "order",
        "intent",
        "BTCUSDT",
        "--profile",
        "default",
        "--market",
        "spot",
        "--side",
        "buy",
        "--kind",
        "market",
        "--quantity",
        "0.0001",
        "--valuation-price",
        "50000",
        "--json",
    ]));
    assert_eq!(market["risk"]["allowed"], true);
    let market_id = market["intent"]["id"].as_str().expect("market intent id");
    let market_submit = env.json(command(&[
        "order",
        "submit",
        market_id,
        "--profile",
        "default",
        "--json",
    ]));
    assert!(
        !market_submit["response"]["request"]["params"]
            .as_array()
            .expect("request params")
            .iter()
            .any(|param| param[0] == "price"),
        "market dry-run should not send an exchange price"
    );

    let test_failure = env.output(command(&[
        "order",
        "submit",
        market_id,
        "--profile",
        "default",
        "--test",
        "--json",
    ]));
    assert!(
        !test_failure.status.success(),
        "test submit without credentials should fail"
    );
    let after_failed_test = env.json(command(&[
        "order",
        "submit",
        market_id,
        "--profile",
        "default",
        "--json",
    ]));
    assert_eq!(after_failed_test["response"]["dry_run"], true);
}

#[test]
fn profile_and_command_boundaries_are_enforced() {
    let env = default_env("profile-boundaries");
    let order = create_limit_order(&env);
    let order_id = order["intent"]["id"].as_str().expect("order intent id");

    env.write_profile("other");
    let profile_mismatch = env.json(command(&[
        "risk",
        "check",
        order_id,
        "--profile",
        "other",
        "--json",
    ]));
    assert_eq!(profile_mismatch["allowed"], false);
    assert!(
        profile_mismatch["findings"]
            .as_array()
            .expect("findings")
            .iter()
            .any(|finding| finding["code"] == "profile-mismatch")
    );

    let transfer = env.json(command(&[
        "transfer",
        "intent",
        "USDT",
        "--profile",
        "default",
        "--direction",
        "spot-to-usds-futures",
        "--amount",
        "1",
        "--json",
    ]));
    assert_eq!(transfer["risk"]["allowed"], false);
    assert!(
        transfer["risk"]["findings"]
            .as_array()
            .expect("findings")
            .iter()
            .any(|finding| finding["code"] == "transfer-not-allowed")
    );
    let transfer_id = transfer["intent"]["id"]
        .as_str()
        .expect("transfer intent id");
    let wrong_submit = env.output(command(&[
        "order",
        "submit",
        transfer_id,
        "--profile",
        "default",
        "--json",
    ]));
    assert!(
        !wrong_submit.status.success(),
        "order submit must reject transfer intents"
    );
    let wrong_live_submit = env.output(command(&[
        "order",
        "submit",
        transfer_id,
        "--profile",
        "default",
        "--live",
        "--json",
    ]));
    assert!(
        !wrong_live_submit.status.success(),
        "wrong live submit must be rejected"
    );
    let correct_transfer_submit = env.output(command(&[
        "transfer",
        "submit",
        transfer_id,
        "--profile",
        "default",
        "--json",
    ]));
    let stderr = String::from_utf8_lossy(&correct_transfer_submit.stderr);
    assert!(
        stderr.contains("risk policy blocked intent submit"),
        "wrong live submit should not consume the transfer intent; stderr={stderr}"
    );
}

fn command(args: &[&str]) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_agent-finance"));
    command.args(args);
    command
}

fn default_env(name: &str) -> TestEnv {
    let env = TestEnv::new(name);
    env.write_profile("default");
    env
}

fn create_limit_order(env: &TestEnv) -> Value {
    env.json(command(&[
        "order",
        "intent",
        "BTCUSDT",
        "--profile",
        "default",
        "--market",
        "spot",
        "--side",
        "buy",
        "--kind",
        "limit",
        "--quantity",
        "0.0001",
        "--price",
        "50000",
        "--time-in-force",
        "gtc",
        "--json",
    ]))
}

struct TestEnv {
    root: std::path::PathBuf,
}

impl TestEnv {
    fn new(name: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("agent-finance-{name}-{nanos}"));
        fs::create_dir_all(&root).expect("test root");
        Self { root }
    }

    fn write_profile(&self, name: &str) {
        let profile_dir = self.root.join("config/agent-finance/profiles");
        fs::create_dir_all(&profile_dir).expect("profile dir");
        let output = self.output(command(&["profile", "template", "--profile", name]));
        assert!(output.status.success(), "profile template should succeed");
        fs::write(profile_dir.join(format!("{name}.toml")), output.stdout).expect("profile write");
    }

    fn json(&self, command: Command) -> Value {
        let output = self.output(command);
        assert!(
            output.status.success(),
            "command failed\nstdout={}\nstderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).expect("json stdout")
    }

    fn output(&self, mut command: Command) -> Output {
        let config_home = self.root.join("config");
        let data_home = self.root.join("data");
        command
            .env("XDG_CONFIG_HOME", config_home)
            .env("XDG_DATA_HOME", data_home)
            .output()
            .expect("agent-finance command should start")
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
