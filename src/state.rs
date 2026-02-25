use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use alloy::signers::local::PrivateKeySigner;
use anyhow::Result;
use hypersdk::Address;
use hypersdk::hypercore::{Chain, HttpClient, NonceHandler};
use serde_json::json;

use crate::cache::WsCache;
use crate::config::{self, Config};
use crate::hyperliquid;

#[derive(Clone)]
pub struct ServerState {
    pub client: Arc<HttpClient>,
    pub http: reqwest::Client,
    pub chain: Chain,
    pub agent_signer: Option<Arc<PrivateKeySigner>>,
    pub main_signer: Option<Arc<PrivateKeySigner>>,
    pub user_address: Option<Address>,
    pub agent_address: Option<Address>,
    pub asset_map: HashMap<String, usize>,
    pub nonce: Arc<NonceHandler>,
    pub builder_fee_approved: Arc<AtomicBool>,
    pub nudge_shown: Arc<AtomicBool>,
    pub cache: Arc<WsCache>,
}

impl ServerState {
    pub async fn new(config: Config) -> Result<Self> {
        let client = HttpClient::new(config.chain);
        let http = reqwest::Client::new();
        let nonce = NonceHandler::default();

        let mut asset_map = HashMap::new();

        match client.perps().await {
            Ok(perps) => {
                for market in &perps {
                    asset_map.insert(market.name.clone(), market.index);
                }
                tracing::info!(count = perps.len(), "Loaded perp markets");
            }
            Err(e) => tracing::warn!(error = %e, "Failed to load perp markets for asset map"),
        }

        match client.spot().await {
            Ok(spots) => {
                for market in &spots {
                    asset_map.insert(market.name.clone(), market.index);
                }
                tracing::info!(count = spots.len(), "Loaded spot markets");
            }
            Err(e) => tracing::warn!(error = %e, "Failed to load spot markets for asset map"),
        }

        // Use main wallet address for info queries (positions, balances, orders).
        // Falls back to agent address if no main address is configured.
        let user_address = config.main_address.or(config.agent_address);

        if config.main_address.is_none() && config.agent_address.is_some() {
            tracing::warn!(
                "No main wallet address configured. Account queries will use the agent wallet address. \
                 Set HYPERLIQUID_WALLET_ADDRESS in your .env to query your main account."
            );
        }

        let cache = if config.realtime {
            crate::ws::spawn(config.chain, user_address, http.clone())
        } else {
            crate::ws::cache_only()
        };

        Ok(ServerState {
            client: Arc::new(client),
            http,
            chain: config.chain,
            agent_signer: config.wallet.map(Arc::new),
            main_signer: config.main_wallet.map(Arc::new),
            user_address,
            agent_address: config.agent_address,
            asset_map,
            nonce: Arc::new(nonce),
            builder_fee_approved: Arc::new(AtomicBool::new(false)),
            nudge_shown: Arc::new(AtomicBool::new(false)),
            cache,
        })
    }

    pub fn require_address(&self) -> Result<Address, rmcp::model::ErrorData> {
        self.user_address.ok_or_else(|| {
            mcp_err(
                "Authentication required. Set HYPERLIQUID_AGENT_PRIVATE_KEY environment variable.",
            )
        })
    }

    pub fn require_agent_signer(&self) -> Result<&Arc<PrivateKeySigner>, rmcp::model::ErrorData> {
        self.agent_signer.as_ref().ok_or_else(|| {
            mcp_err(
                "Authentication required. Set HYPERLIQUID_AGENT_PRIVATE_KEY environment variable.",
            )
        })
    }

    pub fn require_main_signer(&self) -> Result<&Arc<PrivateKeySigner>, rmcp::model::ErrorData> {
        self.main_signer.as_ref().ok_or_else(|| {
            mcp_err(
                "Main wallet required for this operation. \
                 Set HYPERLIQUID_PRIVATE_KEY in your ~/.config/hyperliquid-mcp/.env file.",
            )
        })
    }

    pub fn resolve_asset(&self, coin: &str) -> Result<usize, rmcp::model::ErrorData> {
        self.asset_map.get(coin).copied().ok_or_else(|| {
            mcp_err(&format!(
                "Unknown market '{}'. Use get_markets to see available markets.",
                coin
            ))
        })
    }

    pub fn builder_info(&self) -> hyperliquid::BuilderInfo {
        hyperliquid::BuilderInfo {
            b: config::BUILDER_ADDRESS.to_lowercase(),
            f: config::BUILDER_FEE,
        }
    }

    pub fn next_nonce(&self) -> u64 {
        self.nonce.next()
    }

    pub async fn check_and_cache_builder_approval(&self) -> bool {
        let address = match self.user_address {
            Some(addr) => addr,
            None => return false,
        };

        let result = hyperliquid::raw_info_request(
            &self.http,
            self.chain,
            json!({
                "type": "maxBuilderFee",
                "user": format!("{:#x}", address),
                "builder": config::BUILDER_ADDRESS,
            }),
        )
        .await;

        let approved = match result {
            Ok(val) => {
                let max_fee = val.as_str().unwrap_or("0");
                max_fee != "0" && max_fee != "0%"
            }
            Err(_) => false,
        };

        self.builder_fee_approved.store(approved, Ordering::Relaxed);
        approved
    }

    pub async fn raw_info_request(
        &self,
        request: serde_json::Value,
    ) -> Result<serde_json::Value, rmcp::model::ErrorData> {
        hyperliquid::raw_info_request(&self.http, self.chain, request)
            .await
            .map_err(|e| mcp_err(&format!("API request failed: {e}")))
    }
}

pub fn mcp_err(msg: &str) -> rmcp::model::ErrorData {
    rmcp::model::ErrorData::new(
        rmcp::model::ErrorCode::INTERNAL_ERROR,
        msg.to_string(),
        None::<serde_json::Value>,
    )
}
