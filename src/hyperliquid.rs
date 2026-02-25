//! Custom Hyperliquid exchange operations not available in hypersdk.
//!
//! Implements:
//! - Order placement with builder fee
//! - Builder fee approval (EIP-712 user-signed action)
//! - Leverage updates (RMP-based L1 action)
//! - Raw info requests (POST to /info)
use alloy::dyn_abi::{Eip712Types, Resolver, TypedData};
use alloy::primitives::{Address, B256, keccak256};
use alloy::signers::SignerSync;
use alloy::sol;
use alloy::sol_types::SolStruct;
use hypersdk::hypercore::{Chain, OrderGrouping, OrderRequest};
use serde::Serialize;
use serde_json::Value;

sol! {
    struct Agent {
        string source;
        bytes32 connectionId;
    }

    struct ApproveBuilderFee {
        string hyperliquidChain;
        string maxFeeRate;
        address builder;
        uint64 nonce;
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BuilderInfo {
    pub b: String,
    pub f: u64,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum RmpAction {
    Order(OrderPayload),
    UpdateLeverage(UpdateLeveragePayload),
}

#[derive(Serialize)]
pub struct OrderPayload {
    pub orders: Vec<OrderRequest>,
    pub grouping: OrderGrouping,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub builder: Option<BuilderInfo>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateLeveragePayload {
    pub asset: usize,
    pub is_cross: bool,
    pub leverage: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExchangeRequest<A: Serialize> {
    action: A,
    nonce: u64,
    signature: SignaturePayload,
    vault_address: Option<Address>,
}

#[derive(Serialize)]
struct SignaturePayload {
    r: String,
    s: String,
    v: u64,
}

fn base_url(chain: Chain) -> &'static str {
    match chain {
        Chain::Mainnet => "https://api.hyperliquid.xyz",
        Chain::Testnet => "https://api.hyperliquid-testnet.xyz",
    }
}

fn chain_source(chain: Chain) -> &'static str {
    match chain {
        Chain::Mainnet => "a",
        Chain::Testnet => "b",
    }
}

/// Compute the RMP hash (connection_id) for phantom agent signing.
/// 1. Serialize action with `rmp_serde::to_vec_named`
/// 2. Append nonce (8 bytes big-endian)
/// 3. Append vault flag (0x00 = no vault)
/// 4. keccak256 the buffer
fn rmp_hash(
    action: &impl Serialize,
    nonce: u64,
    vault_address: Option<Address>,
) -> anyhow::Result<B256> {
    let mut bytes = rmp_serde::to_vec_named(action)?;
    bytes.extend(nonce.to_be_bytes());
    if let Some(vault) = vault_address {
        bytes.push(1);
        bytes.extend(vault.as_slice());
    } else {
        bytes.push(0);
    }
    Ok(keccak256(bytes))
}

/// Sign an RMP-based L1 action using the phantom Agent EIP-712 pattern.
fn sign_rmp_action<S: SignerSync>(
    signer: &S,
    action: &impl Serialize,
    nonce: u64,
    chain: Chain,
    vault_address: Option<Address>,
) -> anyhow::Result<SignaturePayload> {
    let connection_id = rmp_hash(action, nonce, vault_address)?;

    let agent = Agent {
        source: chain_source(chain).to_string(),
        connectionId: connection_id,
    };

    let domain = alloy::sol_types::eip712_domain! {
        name: "Exchange",
        version: "1",
        chain_id: 1337,
        verifying_contract: Address::ZERO,
    };

    let signing_hash = agent.eip712_signing_hash(&domain);
    let sig = signer
        .sign_hash_sync(&signing_hash)
        .map_err(|e| anyhow::anyhow!("Signing failed: {e}"))?;

    Ok(SignaturePayload {
        r: format!("{:#066x}", sig.r()),
        s: format!("{:#066x}", sig.s()),
        v: if sig.v() { 28 } else { 27 },
    })
}

#[allow(clippy::too_many_arguments)]
pub async fn place_order_with_builder<S: SignerSync>(
    http: &reqwest::Client,
    chain: Chain,
    signer: &S,
    orders: Vec<OrderRequest>,
    grouping: OrderGrouping,
    builder: Option<BuilderInfo>,
    nonce: u64,
    vault_address: Option<Address>,
) -> anyhow::Result<Value> {
    let action = RmpAction::Order(OrderPayload {
        orders,
        grouping,
        builder,
    });

    let signature = sign_rmp_action(signer, &action, nonce, chain, vault_address)?;

    let request = ExchangeRequest {
        action,
        nonce,
        signature,
        vault_address,
    };

    let url = format!("{}/exchange", base_url(chain));
    let resp = http.post(&url).json(&request).send().await?;
    let body: Value = resp.json().await?;
    Ok(body)
}

#[allow(clippy::too_many_arguments)]
pub async fn update_leverage<S: SignerSync>(
    http: &reqwest::Client,
    chain: Chain,
    signer: &S,
    asset: usize,
    is_cross: bool,
    leverage: u32,
    nonce: u64,
    vault_address: Option<Address>,
) -> anyhow::Result<Value> {
    let action = RmpAction::UpdateLeverage(UpdateLeveragePayload {
        asset,
        is_cross,
        leverage,
    });

    let signature = sign_rmp_action(signer, &action, nonce, chain, vault_address)?;

    let request = ExchangeRequest {
        action,
        nonce,
        signature,
        vault_address,
    };

    let url = format!("{}/exchange", base_url(chain));
    let resp = http.post(&url).json(&request).send().await?;
    let body: Value = resp.json().await?;
    Ok(body)
}

/// Approve builder fees. Uses EIP-712 user-signed action with
pub async fn approve_builder_fee<S: SignerSync>(
    http: &reqwest::Client,
    chain: Chain,
    signer: &S,
    builder_address: Address,
    max_fee_rate: &str,
    nonce: u64,
) -> anyhow::Result<Value> {
    let hyperliquid_chain = match chain {
        Chain::Mainnet => "Mainnet",
        Chain::Testnet => "Testnet",
    };

    // Build the EIP-712 message value
    let message = serde_json::json!({
        "hyperliquidChain": hyperliquid_chain,
        "maxFeeRate": max_fee_rate,
        "builder": format!("{:#x}", builder_address),
        "nonce": nonce,
    });

    let mut resolver = Resolver::from_struct::<ApproveBuilderFee>();
    resolver
        .ingest_string(ApproveBuilderFee::eip712_encode_type())
        .expect("failed to ingest EIP-712 type");
    let mut types = Eip712Types::from(&resolver);
    let fee_type = types
        .remove(ApproveBuilderFee::NAME)
        .expect("ApproveBuilderFee type not found");
    let primary_type = format!("HyperliquidTransaction:{}", ApproveBuilderFee::NAME);
    types.insert(primary_type.clone(), fee_type);

    let domain = alloy::sol_types::eip712_domain! {
        name: "HyperliquidSignTransaction",
        version: "1",
        chain_id: 421614,
        verifying_contract: Address::ZERO,
    };

    let typed_data = TypedData {
        domain,
        resolver: Resolver::from(types),
        primary_type,
        message,
    };

    let signing_hash = typed_data
        .eip712_signing_hash()
        .map_err(|e| anyhow::anyhow!("EIP-712 hash failed: {e}"))?;

    let sig = signer
        .sign_hash_sync(&signing_hash)
        .map_err(|e| anyhow::anyhow!("Signing failed: {e}"))?;

    let signature = SignaturePayload {
        r: format!("{:#066x}", sig.r()),
        s: format!("{:#066x}", sig.s()),
        v: if sig.v() { 28 } else { 27 },
    };

    let action = serde_json::json!({
        "type": "approveBuilderFee",
        "hyperliquidChain": hyperliquid_chain,
        "signatureChainId": "0x66eee",
        "maxFeeRate": max_fee_rate,
        "builder": format!("{:#x}", builder_address),
        "nonce": nonce,
    });

    let request = serde_json::json!({
        "action": action,
        "nonce": nonce,
        "signature": {
            "r": signature.r,
            "s": signature.s,
            "v": signature.v,
        },
        "vaultAddress": null,
    });

    let url = format!("{}/exchange", base_url(chain));
    let resp = http.post(&url).json(&request).send().await?;
    let body: Value = resp.json().await?;
    Ok(body)
}

pub async fn raw_info_request(
    http: &reqwest::Client,
    chain: Chain,
    request: Value,
) -> anyhow::Result<Value> {
    let url = format!("{}/info", base_url(chain));
    let resp = http.post(&url).json(&request).send().await?;
    let body: Value = resp.json().await?;
    Ok(body)
}
