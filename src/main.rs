#[tokio::main]
async fn main() -> anyhow::Result<()> {
    agent_finance_cli::app::run().await
}
