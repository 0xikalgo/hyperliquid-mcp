use std::sync::atomic::Ordering;

use chrono::{Duration, Utc};
use hypersdk::Decimal;
use hypersdk::hypercore::{
    BatchCancel, BatchModify, Cancel, Modify, OrderGrouping, OrderRequest, OrderResponseStatus,
    OrderTypePlacement, TimeInForce,
};
use rmcp::{model::*, schemars};
use rust_decimal::prelude::ToPrimitive;

use crate::hyperliquid;
use crate::state::{ServerState, mcp_err};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct PlaceOrderRequest {
    #[schemars(description = "Trading pair symbol, e.g. \"BTC\", \"ETH\"")]
    pub coin: String,

    #[schemars(description = "Order side: \"buy\" or \"sell\"")]
    pub side: String,

    #[schemars(description = "Order size in coin units (e.g. 0.01 for 0.01 BTC)")]
    pub size: f64,

    #[schemars(description = "Limit price in USD. Required for limit orders, omit for market.")]
    pub price: Option<f64>,

    #[schemars(description = "Order type: \"limit\" (default) or \"market\"")]
    pub order_type: Option<String>,

    #[schemars(description = "Time in force: \"Gtc\" (default), \"Ioc\", or \"Alo\" (post-only)")]
    pub time_in_force: Option<String>,

    #[schemars(
        description = "If true, order can only reduce an existing position (default false)"
    )]
    pub reduce_only: Option<bool>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CancelOrderRequest {
    #[schemars(description = "The coin/market the order is on, e.g. \"BTC\"")]
    pub coin: String,

    #[schemars(description = "The numeric order ID to cancel")]
    pub order_id: u64,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CancelAllOrdersRequest {
    #[schemars(
        description = "Cancel only orders for this coin (optional, cancels all if omitted)"
    )]
    pub coin: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ModifyOrderRequest {
    #[schemars(description = "The numeric order ID to modify")]
    pub order_id: u64,

    #[schemars(description = "The coin/market the order is on, e.g. \"BTC\"")]
    pub coin: String,

    #[schemars(description = "Order side: \"buy\" or \"sell\"")]
    pub side: String,

    #[schemars(description = "New limit price in USD")]
    pub new_price: f64,

    #[schemars(description = "New order size in coin units")]
    pub new_size: f64,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SetLeverageRequest {
    #[schemars(description = "The coin to set leverage for, e.g. \"BTC\"")]
    pub coin: String,

    #[schemars(description = "Leverage multiplier (e.g. 10 for 10x)")]
    pub leverage: u32,

    #[schemars(description = "Margin mode: \"cross\" (default) or \"isolated\"")]
    pub mode: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ClosePositionRequest {
    #[schemars(description = "The coin to close position for, e.g. \"BTC\"")]
    pub coin: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ScheduleCancelRequest {
    #[schemars(
        description = "Seconds from now to cancel all open orders (e.g. 300 for 5 minutes)"
    )]
    pub seconds_from_now: u64,
}

fn format_order_response(statuses: &[OrderResponseStatus]) -> String {
    let parts: Vec<String> = statuses
        .iter()
        .map(|s| match s {
            OrderResponseStatus::Filled {
                total_sz,
                avg_px,
                oid,
            } => {
                format!("Filled {total_sz} @ ${avg_px} (order ID: {oid})")
            }
            OrderResponseStatus::Resting { oid, .. } => {
                format!("Resting (order ID: {oid})")
            }
            OrderResponseStatus::Error(msg) => format!("Error: {msg}"),
            OrderResponseStatus::Success => "Success".into(),
        })
        .collect();
    parts.join("; ")
}

fn format_exchange_response(response: &serde_json::Value) -> String {
    if let Some(status) = response.get("status") {
        if status.as_str() == Some("ok") {
            if let Some(data) = response.get("response").and_then(|r| r.get("data")) {
                if let Some(statuses) = data.get("statuses").and_then(|s| s.as_array()) {
                    let parts: Vec<String> = statuses
                        .iter()
                        .map(|s| {
                            if let Some(filled) = s.get("filled") {
                                let sz = filled
                                    .get("totalSz")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("?");
                                let px =
                                    filled.get("avgPx").and_then(|v| v.as_str()).unwrap_or("?");
                                let oid = filled.get("oid").and_then(|v| v.as_u64()).unwrap_or(0);
                                format!("Filled {sz} @ ${px} (order ID: {oid})")
                            } else if let Some(resting) = s.get("resting") {
                                let oid = resting.get("oid").and_then(|v| v.as_u64()).unwrap_or(0);
                                format!("Resting (order ID: {oid})")
                            } else if let Some(error) = s.get("error") {
                                format!("Error: {}", error.as_str().unwrap_or("unknown"))
                            } else {
                                "Success".into()
                            }
                        })
                        .collect();
                    return parts.join("; ");
                }
            }
            return format!(
                "OK: {}",
                response
                    .get("response")
                    .and_then(|r| r.get("type"))
                    .and_then(|t| t.as_str())
                    .unwrap_or("success")
            );
        } else {
            return format!(
                "Error: {}",
                response.as_str().unwrap_or(&response.to_string())
            );
        }
    }
    format!("{response}")
}

fn maybe_append_builder_nudge(state: &ServerState, output: &mut String) {
    if !state.builder_fee_approved.load(Ordering::Relaxed)
        && !state.nudge_shown.swap(true, Ordering::Relaxed)
    {
        output.push_str(
            "\n\n---\n\
             **Note:** Builder fees (0.01%) are not yet approved for this account. \
             To support development, add `HYPERLIQUID_PRIVATE_KEY=0x...` to your \
             `~/.config/hyperliquid-mcp/.env` file (temporarily remove `HYPERLIQUID_AGENT_PRIVATE_KEY`) \
             and restart — the server will handle agent creation and fee approval automatically. \
             Run `check_builder_fee` for details.",
        );
    }
}

fn parse_tif(s: &str) -> TimeInForce {
    match s.to_lowercase().as_str() {
        "gtc" => TimeInForce::Gtc,
        "ioc" => TimeInForce::Ioc,
        "alo" | "post_only" | "post-only" => TimeInForce::Alo,
        _ => TimeInForce::Gtc,
    }
}

fn to_decimal(f: f64) -> Result<Decimal, ErrorData> {
    Decimal::try_from(f).map_err(|e| mcp_err(&format!("Invalid decimal value: {e}")))
}

/// Round a price to 5 significant figures
fn round_price_5sf(price: Decimal) -> Decimal {
    if price.is_zero() {
        return price;
    }
    let abs_f = price.abs().to_f64().unwrap_or(0.0);
    if abs_f == 0.0 {
        return price;
    }
    let integer_digits = abs_f.log10().floor() as i32 + 1;
    let dp = (5 - integer_digits).max(0) as u32;
    price.round_dp(dp)
}

fn make_cloid() -> alloy::primitives::B128 {
    let uuid = uuid::Uuid::new_v4();
    alloy::primitives::B128::from_slice(uuid.as_bytes())
}

pub async fn place_order(
    state: &ServerState,
    req: PlaceOrderRequest,
) -> Result<CallToolResult, ErrorData> {
    let signer = state.require_agent_signer()?;
    let asset = state.resolve_asset(&req.coin)?;

    let is_buy = match req.side.to_lowercase().as_str() {
        "buy" | "b" | "long" => true,
        "sell" | "s" | "short" => false,
        _ => {
            return Ok(CallToolResult::error(vec![Content::text(
                "Invalid side. Use \"buy\" or \"sell\".",
            )]));
        }
    };

    let order_type_str = req.order_type.as_deref().unwrap_or("limit");
    let reduce_only = req.reduce_only.unwrap_or(false);
    let size = to_decimal(req.size)?;

    let (limit_px, order_type) = if order_type_str == "market" {
        // For market orders, use a very high/low limit price with IOC
        // Fetch current mid price and apply 3% slippage
        let mids = state
            .client
            .all_mids(None)
            .await
            .map_err(|e| mcp_err(&format!("Failed to fetch prices for market order: {e}")))?;

        let mid_price = mids
            .get(&req.coin)
            .ok_or_else(|| mcp_err(&format!("No mid price available for {}", req.coin)))?;

        // 0.05 = 5%
        let slippage = Decimal::new(5, 2);
        let limit_px = if is_buy {
            mid_price * (Decimal::ONE + slippage)
        } else {
            mid_price * (Decimal::ONE - slippage)
        };
        let limit_px = round_price_5sf(limit_px);

        (
            limit_px,
            OrderTypePlacement::Limit {
                tif: TimeInForce::Ioc,
            },
        )
    } else {
        // Limit order
        let price = req.price.ok_or_else(|| {
            mcp_err(
                "Price is required for limit orders. Provide 'price' or use order_type: \"market\".",
            )
        })?;
        let tif = parse_tif(req.time_in_force.as_deref().unwrap_or("Gtc"));
        (to_decimal(price)?, OrderTypePlacement::Limit { tif })
    };

    let order = OrderRequest {
        asset,
        is_buy,
        reduce_only,
        limit_px,
        sz: size,
        cloid: make_cloid(),
        order_type,
    };

    let nonce = state.next_nonce();
    let response = hyperliquid::place_order_with_builder(
        &state.http,
        state.chain,
        signer.as_ref(),
        vec![order],
        OrderGrouping::Na,
        Some(state.builder_info()),
        nonce,
    )
    .await
    .map_err(|e| mcp_err(&format!("Order placement failed: {e}")))?;

    let side_str = if is_buy { "Buy" } else { "Sell" };
    let mut output = format!("## Order Result: {side_str} {} {}", req.size, req.coin);
    if order_type_str == "market" {
        output.push_str(" @ Market\n\n");
    } else if let Some(price) = req.price {
        output.push_str(&format!(" @ ${price}\n\n"));
    } else {
        output.push('\n');
    }

    output.push_str(&format!(
        "Status: {}\n",
        format_exchange_response(&response)
    ));

    maybe_append_builder_nudge(state, &mut output);

    Ok(CallToolResult::success(vec![Content::text(output)]))
}

pub async fn cancel_order(
    state: &ServerState,
    req: CancelOrderRequest,
) -> Result<CallToolResult, ErrorData> {
    let signer = state.require_agent_signer()?;
    let asset = state.resolve_asset(&req.coin)?;

    let cancel = Cancel {
        asset,
        oid: req.order_id,
    };

    let nonce = state.next_nonce();
    let response = state
        .client
        .cancel(
            signer.as_ref(),
            BatchCancel {
                cancels: vec![cancel],
            },
            nonce,
            None,
            None,
        )
        .await
        .map_err(|e| mcp_err(&format!("Cancel failed: {e}")))?;

    Ok(CallToolResult::success(vec![Content::text(format!(
        "Cancel order {} on {}: {}",
        req.order_id,
        req.coin,
        format_order_response(&response)
    ))]))
}

pub async fn cancel_all_orders(
    state: &ServerState,
    req: CancelAllOrdersRequest,
) -> Result<CallToolResult, ErrorData> {
    let address = state.require_address()?;
    let signer = state.require_agent_signer()?;

    let orders = state
        .client
        .open_orders(address, None)
        .await
        .map_err(|e| mcp_err(&format!("Failed to fetch open orders: {e}")))?;

    let to_cancel: Vec<_> = if let Some(ref coin) = req.coin {
        orders
            .iter()
            .filter(|o| o.coin.eq_ignore_ascii_case(coin))
            .collect()
    } else {
        orders.iter().collect()
    };

    if to_cancel.is_empty() {
        return Ok(CallToolResult::success(vec![Content::text(
            "No open orders to cancel.",
        )]));
    }

    let cancels: Vec<Cancel> = to_cancel
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
    let response = state
        .client
        .cancel(signer.as_ref(), BatchCancel { cancels }, nonce, None, None)
        .await
        .map_err(|e| mcp_err(&format!("Cancel all failed: {e}")))?;

    Ok(CallToolResult::success(vec![Content::text(format!(
        "Cancelled {cancel_count} orders: {}",
        format_order_response(&response)
    ))]))
}

pub async fn modify_order(
    state: &ServerState,
    req: ModifyOrderRequest,
) -> Result<CallToolResult, ErrorData> {
    let signer = state.require_agent_signer()?;
    let asset = state.resolve_asset(&req.coin)?;

    let is_buy = match req.side.to_lowercase().as_str() {
        "buy" | "b" | "long" => true,
        "sell" | "s" | "short" => false,
        _ => {
            return Ok(CallToolResult::error(vec![Content::text(
                "Invalid side. Use \"buy\" or \"sell\".",
            )]));
        }
    };

    let modify = Modify {
        oid: either::Either::Left(req.order_id),
        order: OrderRequest {
            asset,
            is_buy,
            reduce_only: false,
            limit_px: to_decimal(req.new_price)?,
            sz: to_decimal(req.new_size)?,
            cloid: make_cloid(),
            order_type: OrderTypePlacement::Limit {
                tif: TimeInForce::Gtc,
            },
        },
    };

    let nonce = state.next_nonce();
    let response = state
        .client
        .modify(
            signer.as_ref(),
            BatchModify {
                modifies: vec![modify],
            },
            nonce,
            None,
            None,
        )
        .await
        .map_err(|e| mcp_err(&format!("Modify failed: {e}")))?;

    Ok(CallToolResult::success(vec![Content::text(format!(
        "Modify order {}: {}",
        req.order_id,
        format_order_response(&response)
    ))]))
}

pub async fn set_leverage(
    state: &ServerState,
    req: SetLeverageRequest,
) -> Result<CallToolResult, ErrorData> {
    let signer = state.require_agent_signer()?;
    let asset = state.resolve_asset(&req.coin)?;
    let is_cross = req.mode.as_deref().unwrap_or("cross") != "isolated";

    let nonce = state.next_nonce();
    let response = hyperliquid::update_leverage(
        &state.http,
        state.chain,
        signer.as_ref(),
        asset,
        is_cross,
        req.leverage,
        nonce,
    )
    .await
    .map_err(|e| mcp_err(&format!("Update leverage failed: {e}")))?;

    let mode_str = if is_cross { "cross" } else { "isolated" };
    Ok(CallToolResult::success(vec![Content::text(format!(
        "Set {} leverage to {}x {}: {}",
        req.coin,
        req.leverage,
        mode_str,
        format_exchange_response(&response)
    ))]))
}

pub async fn close_position(
    state: &ServerState,
    req: ClosePositionRequest,
) -> Result<CallToolResult, ErrorData> {
    let signer = state.require_agent_signer()?;
    let address = state.require_address()?;
    let asset = state.resolve_asset(&req.coin)?;

    let user_state = state
        .client
        .clearinghouse_state(address, None)
        .await
        .map_err(|e| mcp_err(&format!("Failed to fetch positions: {e}")))?;

    let position = user_state
        .asset_positions
        .iter()
        .find(|p| p.position.coin.eq_ignore_ascii_case(&req.coin) && !p.position.szi.is_zero())
        .ok_or_else(|| mcp_err(&format!("No open position for {}", req.coin)))?;

    let szi = position.position.szi;
    // Sell to close long, buy to close short
    let is_buy = szi.is_sign_negative();
    let size = szi.abs();

    // Get current mid price for slippage calculation
    let mids = state
        .client
        .all_mids(None)
        .await
        .map_err(|e| mcp_err(&format!("Failed to fetch prices: {e}")))?;

    let mid_price = mids
        .get(&req.coin)
        .ok_or_else(|| mcp_err(&format!("No mid price available for {}", req.coin)))?;

    // 5%
    let slippage = Decimal::new(5, 2);
    let limit_px = if is_buy {
        mid_price * (Decimal::ONE + slippage)
    } else {
        mid_price * (Decimal::ONE - slippage)
    };
    let limit_px = round_price_5sf(limit_px);

    let order = OrderRequest {
        asset,
        is_buy,
        reduce_only: true,
        limit_px,
        sz: size,
        cloid: make_cloid(),
        order_type: OrderTypePlacement::Limit {
            tif: TimeInForce::Ioc,
        },
    };

    let nonce = state.next_nonce();
    let response = hyperliquid::place_order_with_builder(
        &state.http,
        state.chain,
        signer.as_ref(),
        vec![order],
        OrderGrouping::Na,
        Some(state.builder_info()),
        nonce,
    )
    .await
    .map_err(|e| mcp_err(&format!("Close position failed: {e}")))?;

    let mut output = format!(
        "## Close {} Position\n\nResult: {}",
        req.coin,
        format_exchange_response(&response)
    );

    maybe_append_builder_nudge(state, &mut output);

    Ok(CallToolResult::success(vec![Content::text(output)]))
}

pub async fn schedule_cancel(
    state: &ServerState,
    req: ScheduleCancelRequest,
) -> Result<CallToolResult, ErrorData> {
    let signer = state.require_agent_signer()?;

    if req.seconds_from_now == 0 {
        return Ok(CallToolResult::error(vec![Content::text(
            "seconds_from_now must be greater than 0.",
        )]));
    }

    let when = Utc::now()
        + Duration::seconds(
            i64::try_from(req.seconds_from_now)
                .map_err(|_| mcp_err("seconds_from_now too large"))?,
        );

    let nonce = state.next_nonce();
    state
        .client
        .schedule_cancel(signer.as_ref(), nonce, when, None, None)
        .await
        .map_err(|e| mcp_err(&format!("Schedule cancel failed: {e}")))?;

    let output = format!(
        "Scheduled cancellation of all open orders at {} UTC ({} seconds from now).",
        when.format("%Y-%m-%d %H:%M:%S"),
        req.seconds_from_now,
    );

    Ok(CallToolResult::success(vec![Content::text(output)]))
}
