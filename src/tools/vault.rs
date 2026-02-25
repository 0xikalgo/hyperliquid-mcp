use rmcp::{model::*, schemars};
use serde_json::json;

use crate::hyperliquid;
use crate::state::{ServerState, mcp_err};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetVaultDetailsRequest {
    #[schemars(description = "Vault address (defaults to HYPERLIQUID_VAULT_ADDRESS if set)")]
    pub vault_address: Option<String>,
}

pub async fn get_vault_details(
    state: &ServerState,
    req: GetVaultDetailsRequest,
) -> Result<CallToolResult, ErrorData> {
    let vault_addr: hypersdk::Address = match req.vault_address {
        Some(ref addr) => addr
            .trim()
            .parse()
            .map_err(|_| mcp_err("Invalid vault address"))?,
        None => state.vault_address.ok_or_else(|| {
            mcp_err(
                "No vault address. Set HYPERLIQUID_VAULT_ADDRESS or provide vault_address parameter.",
            )
        })?,
    };

    let details = state
        .raw_info_request(json!({
            "type": "vaultDetails",
            "vaultAddress": format!("{:#x}", vault_addr),
        }))
        .await?;

    let name = details
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown");
    let leader = details
        .get("leader")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown");
    let description = details
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let portfolio = details.get("portfolio").and_then(|v| v.as_object());
    let followers = details
        .get("followers")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);

    let mut output = format!(
        "## Vault: {name}\n\n\
         | Field | Value |\n\
         |-------|-------|\n\
         | Vault Address | `{:#x}` |\n\
         | Leader | `{leader}` |\n\
         | Followers | {followers} |\n",
        vault_addr,
    );

    if !description.is_empty() {
        output.push_str(&format!("| Description | {} |\n", description));
    }

    if let Some(p) = portfolio {
        if let Some(acv) = p.get("accountValue").and_then(|v| v.as_str()) {
            output.push_str(&format!("| Account Value | ${acv} |\n"));
        }
        if let Some(pnl) = p.get("allTimePnl").and_then(|v| v.as_str()) {
            output.push_str(&format!("| All-Time PnL | ${pnl} |\n"));
        }
    }

    if state.is_vault_mode() && state.vault_address == Some(vault_addr) {
        output.push_str("\n**Operating as vault leader.**\n");
    }

    Ok(CallToolResult::success(vec![Content::text(output)]))
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct EmergencyCloseAllRequest {
    #[schemars(
        description = "Must be true to confirm emergency close of ALL positions and orders"
    )]
    pub confirm: bool,
}

pub async fn emergency_close_all(
    state: &ServerState,
    req: EmergencyCloseAllRequest,
) -> Result<CallToolResult, ErrorData> {
    use hypersdk::Decimal;
    use hypersdk::hypercore::{
        BatchCancel, Cancel, OrderGrouping, OrderRequest, OrderTypePlacement, TimeInForce,
    };

    if !req.confirm {
        return Ok(CallToolResult::error(vec![Content::text(
            "Emergency close requires confirm: true. \
             This will close ALL positions and cancel ALL orders.",
        )]));
    }

    let address = state.query_address()?;
    let signer = state.require_signer()?;
    let mut output = "## Emergency Close All\n\n".to_string();

    let orders = state
        .client
        .open_orders(address, None)
        .await
        .map_err(|e| mcp_err(&format!("Failed to fetch open orders: {e}")))?;

    if !orders.is_empty() {
        let cancels: Vec<Cancel> = orders
            .iter()
            .filter_map(|o| {
                state
                    .asset_map
                    .get(&o.coin)
                    .map(|&asset| Cancel { asset, oid: o.oid })
            })
            .collect();

        let cancel_count = cancels.len();
        let nonce = state.next_nonce();
        state
            .client
            .cancel(
                signer.as_ref(),
                BatchCancel { cancels },
                nonce,
                state.vault_addr(),
                None,
            )
            .await
            .map_err(|e| mcp_err(&format!("Cancel all failed: {e}")))?;

        output.push_str(&format!("Cancelled {cancel_count} orders.\n"));
    } else {
        output.push_str("No open orders to cancel.\n");
    }

    let user_state = state
        .client
        .clearinghouse_state(address, None)
        .await
        .map_err(|e| mcp_err(&format!("Failed to fetch positions: {e}")))?;

    let positions: Vec<_> = user_state
        .asset_positions
        .iter()
        .filter(|p| !p.position.szi.is_zero())
        .collect();

    if !positions.is_empty() {
        let mids = state
            .client
            .all_mids(None)
            .await
            .map_err(|e| mcp_err(&format!("Failed to fetch prices: {e}")))?;

        // 5%
        let slippage = Decimal::new(5, 2);

        let close_orders: Vec<OrderRequest> = positions
            .iter()
            .filter_map(|ap| {
                let p = &ap.position;
                let asset = state.asset_map.get(&p.coin).copied()?;
                let is_buy = p.szi.is_sign_negative();
                let size = p.szi.abs();
                let mid = mids.get(&p.coin)?;
                let limit_px = if is_buy {
                    mid * (Decimal::ONE + slippage)
                } else {
                    mid * (Decimal::ONE - slippage)
                };
                Some(OrderRequest {
                    asset,
                    is_buy,
                    reduce_only: true,
                    limit_px,
                    sz: size,
                    cloid: {
                        let uuid = uuid::Uuid::new_v4();
                        alloy::primitives::B128::from_slice(uuid.as_bytes())
                    },
                    order_type: OrderTypePlacement::Limit {
                        tif: TimeInForce::Ioc,
                    },
                })
            })
            .collect();

        let close_count = close_orders.len();
        let nonce = state.next_nonce();
        hyperliquid::place_order_with_builder(
            &state.http,
            state.chain,
            signer.as_ref(),
            close_orders,
            OrderGrouping::Na,
            Some(state.builder_info()),
            nonce,
            state.vault_addr(),
        )
        .await
        .map_err(|e| mcp_err(&format!("Close positions failed: {e}")))?;

        output.push_str(&format!("Closed {close_count} positions.\n"));
    } else {
        output.push_str("No open positions to close.\n");
    }

    state.cache.invalidate_user_data().await;

    Ok(CallToolResult::success(vec![Content::text(output)]))
}
