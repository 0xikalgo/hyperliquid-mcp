use anyhow::Result;
use rmcp::{ServiceExt, transport::stdio};
use tracing_subscriber::{self, EnvFilter};

pub mod agent;
mod cache;
mod config;
mod hyperliquid;
mod server;
mod state;
mod tools;
mod ws;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    tracing::info!("Starting Hyperliquid MCP server");

    let mut config = config::Config::from_env()?;

    if config.main_wallet.is_some() && config.wallet.is_none() && config.vault_address.is_none() {
        run_setup(&mut config).await?;
    }

    if config.main_wallet.is_some() && config.wallet.is_some() && config.vault_address.is_none() {
        tracing::info!(
            "Both HYPERLIQUID_PRIVATE_KEY and HYPERLIQUID_AGENT_PRIVATE_KEY are set. \
             Using AGENT_PRIVATE_KEY for trading."
        );
    }

    let state = state::ServerState::new(config).await?;

    // Check builder fee approval status at startup
    if state.user_address.is_some() {
        let approved = state.check_and_cache_builder_approval().await;
        if approved {
            tracing::info!("Builder fees approved for this account");
        } else {
            tracing::warn!(
                "Builder fees not approved for this account. \
                 Fees are configured but will not be charged until approved. \
                 Set HYPERLIQUID_PRIVATE_KEY in your .env and restart to approve automatically."
            );
        }
    }

    let server = server::HyperliquidMcp::new(state);

    let service = server.serve(stdio()).await?;
    service.waiting().await?;

    Ok(())
}

async fn run_setup(config: &mut config::Config) -> Result<()> {
    use hypersdk::hypercore::{HttpClient, NonceHandler};

    let main_wallet = config
        .main_wallet
        .clone()
        .expect("main_wallet checked before calling run_setup");
    let main_address = main_wallet.address();

    let is_mainnet = matches!(config.chain, hypersdk::hypercore::Chain::Mainnet);
    let network_name = if is_mainnet { "mainnet" } else { "testnet" };

    tracing::info!(address = %main_address, network = network_name, "Running first-time setup");

    let client = HttpClient::new(config.chain);
    let http = reqwest::Client::new();
    let nonce = NonceHandler::default();

    tracing::info!("Creating agent wallet...");
    let agent_key_hex = agent::create_agent_wallet(&client, &main_wallet, nonce.next()).await?;
    let display_name = agent::agent_name_today();
    tracing::info!(name = display_name, "Agent wallet created");

    let (agent_wallet, agent_address) = agent::wallet_and_address(&agent_key_hex)?;
    tracing::info!(address = %agent_address, "Agent wallet address");

    tracing::info!("Approving builder fees...");
    let builder_addr: hypersdk::Address = config::BUILDER_ADDRESS
        .parse()
        .map_err(|e| anyhow::anyhow!("Invalid builder address: {e}"))?;
    let fee_status = hyperliquid::approve_builder_fee(
        &http,
        config.chain,
        &main_wallet,
        builder_addr,
        "0.01%",
        nonce.next(),
    )
    .await?;
    tracing::info!(status = ?fee_status, "Builder fee approval");

    let env_path = agent::save_agent_key_to_env(&agent_key_hex)?;
    tracing::info!(path = %env_path.display(), "Setup complete — agent key saved");

    config.agent_address = Some(agent_address);
    config.wallet = Some(agent_wallet);

    Ok(())
}
