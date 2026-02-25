use rmcp::{model::*, schemars};
use serde_json::json;

use crate::hyperliquid;
use crate::state::{ServerState, mcp_err};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct TransferSpotPerpsRequest {
    #[schemars(description = "Amount of USDC to transfer")]
    pub amount: f64,

    #[schemars(description = "Transfer direction: \"to_spot\" or \"to_perps\"")]
    pub direction: String,
}

pub async fn transfer_between_spot_perps(
    state: &ServerState,
    req: TransferSpotPerpsRequest,
) -> Result<CallToolResult, ErrorData> {
    let signer = state.require_signer()?;

    if req.amount <= 0.0 {
        return Ok(CallToolResult::error(vec![Content::text(
            "Amount must be positive.",
        )]));
    }

    let to_perp = match req.direction.to_lowercase().as_str() {
        "to_perps" | "perps" | "to_perp" => true,
        "to_spot" | "spot" => false,
        _ => {
            return Ok(CallToolResult::error(vec![Content::text(
                "Invalid direction. Use \"to_spot\" or \"to_perps\".",
            )]));
        }
    };

    let amount = rust_decimal::Decimal::try_from(req.amount)
        .map_err(|e| mcp_err(&format!("Invalid amount: {e}")))?;

    let tokens = state
        .client
        .spot_tokens()
        .await
        .map_err(|e| mcp_err(&format!("Failed to fetch token info: {e}")))?;

    let usdc_token = tokens
        .iter()
        .find(|t| t.name == "USDC" || t.name == "usdc")
        .ok_or_else(|| mcp_err("USDC token not found"))?;

    let nonce = state.next_nonce();
    if to_perp {
        state
            .client
            .transfer_to_perps(signer.as_ref(), usdc_token.clone(), amount, nonce)
            .await
            .map_err(|e| mcp_err(&format!("Transfer to perps failed: {e}")))?;
    } else {
        state
            .client
            .transfer_to_spot(signer.as_ref(), usdc_token.clone(), amount, nonce)
            .await
            .map_err(|e| mcp_err(&format!("Transfer to spot failed: {e}")))?;
    }

    let dir_str = if to_perp {
        "spot → perps"
    } else {
        "perps → spot"
    };

    Ok(CallToolResult::success(vec![Content::text(format!(
        "Transferred {} USDC ({dir_str})",
        req.amount
    ))]))
}

pub async fn create_agent_wallet(state: &ServerState) -> Result<CallToolResult, ErrorData> {
    use crate::agent;

    let main_signer = state.require_main_signer()?;
    let signing_address = main_signer.address();
    tracing::info!(address = %signing_address, "Creating agent wallet with main wallet");

    let display_name = agent::agent_name_today();

    let nonce = state.next_nonce();
    let agent_key_hex = agent::create_agent_wallet(&state.client, main_signer.as_ref(), nonce)
        .await
        .map_err(|e| {
            mcp_err(&format!(
                "Agent wallet creation failed (signing address: {signing_address:#x}): {e}"
            ))
        })?;

    let (_agent_wallet, agent_address) = agent::wallet_and_address(&agent_key_hex)
        .map_err(|e| mcp_err(&format!("Failed to parse agent key: {e}")))?;

    let env_path = agent::save_agent_key_to_env(&agent_key_hex)
        .map_err(|e| mcp_err(&format!("Failed to save agent key: {e}")))?;

    Ok(CallToolResult::success(vec![Content::text(format!(
        "New agent wallet created.\n\n\
         Name: {display_name}\n\
         Agent address: {agent_address:#x}\n\
         Created by main wallet: {signing_address:#x}\n\
         Saved to: {}\n\n\
         Restart the MCP server to use the new agent wallet.",
        env_path.display()
    ))]))
}

pub async fn approve_builder_fee(state: &ServerState) -> Result<CallToolResult, ErrorData> {
    use crate::config;

    let main_signer = state.require_main_signer()?;
    let signing_address = main_signer.address();
    tracing::info!(address = %signing_address, "Approving builder fee with main wallet");

    let builder_addr: hypersdk::Address = config::BUILDER_ADDRESS
        .parse()
        .map_err(|_| mcp_err("Invalid builder address constant"))?;

    let nonce = state.next_nonce();
    let status = hyperliquid::approve_builder_fee(
        &state.http,
        state.chain,
        main_signer.as_ref(),
        builder_addr,
        "0.01%",
        nonce,
    )
    .await
    .map_err(|e| {
        mcp_err(&format!(
            "Builder fee approval failed (signing address: {signing_address:#x}): {e}"
        ))
    })?;

    state
        .builder_fee_approved
        .store(true, std::sync::atomic::Ordering::Relaxed);

    Ok(CallToolResult::success(vec![Content::text(format!(
        "Builder fees approved (0.01%) for account {signing_address:#x}. Status: {status}"
    ))]))
}

pub async fn check_builder_fee(state: &ServerState) -> Result<CallToolResult, ErrorData> {
    use crate::config;

    let fee_bps = config::BUILDER_FEE as f64 / 10.0;
    let fee_pct = fee_bps / 100.0;

    let mut output = format!(
        "## Builder Fee Information\n\n\
         Builder Address: `{}`\n\
         Fee Rate: {fee_bps} bps ({fee_pct}%)\n\n",
        config::BUILDER_ADDRESS
    );

    if let Some(address) = state.user_address {
        let result = state
            .raw_info_request(json!({
                "type": "maxBuilderFee",
                "user": format!("{:#x}", address),
                "builder": config::BUILDER_ADDRESS,
            }))
            .await;

        match result {
            Ok(val) => {
                let max_fee = val.as_str().unwrap_or("0");
                if max_fee == "0" || max_fee == "0%" {
                    output.push_str(
                        "**Status: Not approved**\n\n\
                         You have not yet approved builder fees for this server. \
                         Builder fees help support the development of this MCP server.\n\n\
                         **To approve:** Use the `approve_builder_fee` tool. \
                         Requires `HYPERLIQUID_PRIVATE_KEY` (main wallet) in your \
                         `~/.config/hyperliquid-mcp/.env` file.",
                    );
                } else {
                    output.push_str(&format!(
                        "**Status: Approved** (max fee rate: {max_fee})\n\n\
                         Builder fees are active. A small fee of {fee_bps} bps is applied \
                         to each trade to support this MCP server's development."
                    ));
                }
            }
            Err(_) => {
                output.push_str(
                    "Could not check approval status. Builder fees may or may not be active.",
                );
            }
        }
    } else {
        output.push_str(
            "No wallet configured — cannot check approval status. \
             Set HYPERLIQUID_PRIVATE_KEY or HYPERLIQUID_AGENT_PRIVATE_KEY to enable trading.",
        );
    }

    Ok(CallToolResult::success(vec![Content::text(output)]))
}
