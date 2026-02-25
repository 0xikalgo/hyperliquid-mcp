use alloy::signers::local::PrivateKeySigner;
use anyhow::{Context, Result};
use hypersdk::Address;
use hypersdk::hypercore::Chain;

pub const BUILDER_ADDRESS: &str = "0xdadcB94d61D4A14e8aD1b94Acf888120b7E807aE";
// 10 = 0.01%
pub const BUILDER_FEE: u64 = 10;

pub struct Config {
    pub wallet: Option<PrivateKeySigner>,
    pub main_wallet: Option<PrivateKeySigner>,
    pub main_address: Option<Address>,
    pub agent_address: Option<Address>,
    pub vault_address: Option<Address>,
    pub chain: Chain,
    pub realtime: bool,
}

pub const ENV_FILE_PATH: &str = ".config/hyperliquid-mcp/.env";

impl Config {
    pub fn from_env() -> Result<Self> {
        if let Some(home) = dirs::home_dir() {
            let env_path = home.join(ENV_FILE_PATH);
            match dotenvy::from_path(&env_path) {
                Ok(()) => tracing::info!(path = %env_path.display(), "Loaded .env file"),
                Err(dotenvy::Error::Io(_)) => {
                    // File doesn't exist
                }
                Err(e) => {
                    tracing::warn!(path = %env_path.display(), error = %e, "Failed to parse .env file")
                }
            }
        }

        let wallet = match std::env::var("HYPERLIQUID_AGENT_PRIVATE_KEY") {
            Ok(key) => {
                let key = key.trim().trim_start_matches("0x");
                let signer: PrivateKeySigner = key.parse().context(
                    "Failed to parse HYPERLIQUID_AGENT_PRIVATE_KEY as a valid hex private key",
                )?;
                tracing::info!(address = %signer.address(), "Loaded agent wallet key");
                Some(signer)
            }
            Err(_) => None,
        };

        let main_wallet = match std::env::var("HYPERLIQUID_PRIVATE_KEY") {
            Ok(key) => {
                let key = key.trim().trim_start_matches("0x");
                let signer: PrivateKeySigner = key.parse().context(
                    "Failed to parse HYPERLIQUID_PRIVATE_KEY as a valid hex private key",
                )?;
                tracing::info!(address = %signer.address(), "Loaded main wallet key (for setup)");
                Some(signer)
            }
            Err(_) => None,
        };

        // Priority: main_wallet key > HYPERLIQUID_WALLET_ADDRESS env var > agent wallet (fallback)
        let main_address = main_wallet
            .as_ref()
            .map(|w| w.address())
            .or_else(|| {
                std::env::var("HYPERLIQUID_WALLET_ADDRESS")
                    .ok()
                    .and_then(|addr| {
                        let addr = addr.trim().to_string();
                        addr.parse::<Address>()
                            .inspect_err(|e| {
                                tracing::warn!(error = %e, "Failed to parse HYPERLIQUID_WALLET_ADDRESS");
                            })
                            .ok()
                    })
            });

        if let Some(addr) = main_address {
            tracing::info!(address = %addr, "Main wallet address for account queries");
        }

        if wallet.is_none() && main_wallet.is_none() {
            tracing::warn!(
                "HYPERLIQUID_AGENT_PRIVATE_KEY not set — running in read-only mode (market data only)"
            );
        }

        let chain = match std::env::var("HYPERLIQUID_NETWORK")
            .unwrap_or_else(|_| "mainnet".to_string())
            .to_lowercase()
            .as_str()
        {
            "testnet" | "test" => Chain::Testnet,
            _ => Chain::Mainnet,
        };
        let network_name = match chain {
            Chain::Testnet => "testnet",
            Chain::Mainnet => "mainnet",
        };
        tracing::info!(network = network_name, "Selected network");

        let realtime = std::env::var("REALTIME_ENABLED")
            .map(|v| !matches!(v.to_lowercase().as_str(), "false" | "0" | "no" | "off"))
            .unwrap_or(true);
        if !realtime {
            tracing::info!("WebSocket real-time cache disabled");
        }

        let agent_address = wallet.as_ref().map(|w| w.address());

        let vault_address = std::env::var("HYPERLIQUID_VAULT_ADDRESS")
            .ok()
            .and_then(|addr| {
                addr.trim()
                    .parse::<Address>()
                    .inspect_err(|e| {
                        tracing::warn!(error = %e, "Failed to parse HYPERLIQUID_VAULT_ADDRESS");
                    })
                    .ok()
            });

        if let Some(vault) = vault_address {
            tracing::info!(vault = %vault, "Vault mode enabled — trading as vault leader");
        }

        Ok(Config {
            wallet,
            main_wallet,
            main_address,
            agent_address,
            vault_address,
            chain,
            realtime,
        })
    }
}
