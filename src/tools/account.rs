use std::time::Duration;

use either::Either;
use rmcp::{model::*, schemars};

use crate::cache::CachedValue;
use crate::state::{ServerState, mcp_err};

const POSITIONS_TTL: Duration = Duration::from_secs(3);
const OPEN_ORDERS_TTL: Duration = Duration::from_secs(2);

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetOpenOrdersRequest {
    #[schemars(
        description = "Filter by coin symbol, e.g. \"BTC\" (optional, returns all if omitted)"
    )]
    pub coin: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetTradeHistoryRequest {
    #[schemars(description = "Filter by coin symbol (optional)")]
    pub coin: Option<String>,

    #[schemars(description = "Number of trades to return (default 50, max 200)")]
    pub limit: Option<usize>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetOrderStatusRequest {
    #[schemars(description = "The numeric order ID to look up")]
    pub order_id: u64,
}

pub async fn get_wallet_address(state: &ServerState) -> Result<CallToolResult, ErrorData> {
    let address = state.require_address()?;
    let mut output = format!("Main wallet (account owner): {:#x}", address);

    if let Some(agent_addr) = state.agent_address {
        if agent_addr != address {
            output.push_str(&format!(
                "\nAgent wallet (trade signing): {:#x}",
                agent_addr
            ));
        }
    }

    Ok(CallToolResult::success(vec![Content::text(output)]))
}

pub async fn get_positions(state: &ServerState) -> Result<CallToolResult, ErrorData> {
    let address = state.require_address()?;

    let user_state = get_cached_clearinghouse(state, address).await?;

    let positions: Vec<_> = user_state
        .asset_positions
        .iter()
        .filter(|p| !p.position.szi.is_zero())
        .collect();

    if positions.is_empty() {
        return Ok(CallToolResult::success(vec![Content::text(
            "No open positions.",
        )]));
    }

    let mut output = format!("## Open Positions ({})\n\n", positions.len());
    output.push_str(
        "| Market | Side | Size | Entry Price | Mark Value | Unrealized PnL | ROE | Liq. Price | Leverage | Margin Used |\n",
    );
    output.push_str(
        "|--------|------|------|-------------|------------|----------------|-----|------------|----------|-------------|\n",
    );

    for ap in &positions {
        let p = &ap.position;
        let side = if p.szi.is_sign_positive() {
            "Long"
        } else {
            "Short"
        };
        let size = p.szi.abs();
        let entry = p
            .entry_px
            .map(|px| format!("${px}"))
            .unwrap_or_else(|| "N/A".into());
        let liq = p
            .liquidation_px
            .map(|px| format!("${px}"))
            .unwrap_or_else(|| "N/A".into());
        let lev = format!("{}x {}", p.leverage.value, p.leverage.leverage_type);
        let roe_pct = format!(
            "{:.2}",
            p.return_on_equity * rust_decimal::Decimal::from(100)
        );

        output.push_str(&format!(
            "| {} | {} | {} | {} | ${} | ${} | {}% | {} | {} | ${} |\n",
            p.coin,
            side,
            size,
            entry,
            p.position_value,
            p.unrealized_pnl,
            roe_pct,
            liq,
            lev,
            p.margin_used,
        ));
    }

    let ms = &user_state.margin_summary;
    let available = ms.account_value - ms.total_margin_used;
    output.push_str(&format!(
        "\n## Account Summary\n\n\
         | Metric | Value |\n\
         |--------|-------|\n\
         | Account Value | ${} |\n\
         | Total Position Notional | ${} |\n\
         | Total Margin Used | ${} |\n\
         | Available Margin | ${:.2} |\n\
         | Withdrawable | ${} |\n",
        ms.account_value,
        ms.total_ntl_pos,
        ms.total_margin_used,
        available,
        user_state.withdrawable,
    ));

    Ok(CallToolResult::success(vec![Content::text(output)]))
}

pub async fn get_balances(state: &ServerState) -> Result<CallToolResult, ErrorData> {
    let address = state.require_address()?;

    let user_state = get_cached_clearinghouse(state, address).await?;

    let ms = &user_state.margin_summary;
    let available = ms.account_value - ms.total_margin_used;

    let mut output = "## Perpetual Account\n\n".to_string();
    output.push_str("| Metric | Value |\n");
    output.push_str("|--------|-------|\n");
    output.push_str(&format!("| Account Value | ${} |\n", ms.account_value));
    output.push_str(&format!(
        "| Total Margin Used | ${} |\n",
        ms.total_margin_used
    ));
    output.push_str(&format!("| Available Margin | ${:.2} |\n", available));
    output.push_str(&format!(
        "| Withdrawable | ${} |\n",
        user_state.withdrawable
    ));

    let token_balances = state
        .client
        .user_balances(address)
        .await
        .map_err(|e| mcp_err(&format!("Failed to fetch spot balances: {e}")))?;

    let nonzero: Vec<_> = token_balances
        .iter()
        .filter(|b| !b.total.is_zero())
        .collect();

    if !nonzero.is_empty() {
        output.push_str("\n## Spot Balances\n\n");
        output.push_str("| Token | Total | Available | Held |\n");
        output.push_str("|-------|-------|-----------|------|\n");

        for b in &nonzero {
            let available = b.total - b.hold;
            output.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                b.coin, b.total, available, b.hold,
            ));
        }
    }

    Ok(CallToolResult::success(vec![Content::text(output)]))
}

pub async fn get_open_orders(
    state: &ServerState,
    req: GetOpenOrdersRequest,
) -> Result<CallToolResult, ErrorData> {
    let address = state.require_address()?;

    let orders = get_cached_open_orders(state, address).await?;

    let filtered: Vec<_> = if let Some(ref coin) = req.coin {
        orders
            .iter()
            .filter(|o| o.coin.eq_ignore_ascii_case(coin))
            .collect()
    } else {
        orders.iter().collect()
    };

    if filtered.is_empty() {
        let msg = match &req.coin {
            Some(c) => format!("No open orders for {c}."),
            None => "No open orders.".into(),
        };
        return Ok(CallToolResult::success(vec![Content::text(msg)]));
    }

    let mut output = format!("## Open Orders ({})\n\n", filtered.len());
    output.push_str("| Market | Side | Price | Size | Order ID |\n");
    output.push_str("|--------|------|-------|------|----------|\n");

    for o in &filtered {
        let side = match o.side {
            hypersdk::hypercore::Side::Bid => "Buy",
            hypersdk::hypercore::Side::Ask => "Sell",
        };
        output.push_str(&format!(
            "| {} | {} | ${} | {} | {} |\n",
            o.coin, side, o.limit_px, o.sz, o.oid,
        ));
    }

    Ok(CallToolResult::success(vec![Content::text(output)]))
}

pub async fn get_trade_history(
    state: &ServerState,
    req: GetTradeHistoryRequest,
) -> Result<CallToolResult, ErrorData> {
    let address = state.require_address()?;

    let fills = state
        .client
        .user_fills(address)
        .await
        .map_err(|e| mcp_err(&format!("Failed to fetch trade history: {e}")))?;

    let limit = req.limit.unwrap_or(50).min(200);

    let filtered: Vec<_> = if let Some(ref coin) = req.coin {
        fills
            .iter()
            .filter(|f| f.coin.eq_ignore_ascii_case(coin))
            .take(limit)
            .collect()
    } else {
        fills.iter().take(limit).collect()
    };

    if filtered.is_empty() {
        return Ok(CallToolResult::success(vec![Content::text(
            "No recent trades.",
        )]));
    }

    let mut output = format!("## Recent Trades ({})\n\n", filtered.len());
    output.push_str("| Time | Market | Side | Price | Size | Direction | Fee | Closed PnL |\n");
    output.push_str("|------|--------|------|-------|------|-----------|-----|------------|\n");

    for f in &filtered {
        let time = chrono_from_ms(f.time);
        let side = match f.side {
            hypersdk::hypercore::Side::Bid => "Buy",
            hypersdk::hypercore::Side::Ask => "Sell",
        };
        let pnl_str = if f.closed_pnl.is_zero() {
            "—".into()
        } else {
            format!("${}", f.closed_pnl)
        };
        output.push_str(&format!(
            "| {} | {} | {} | ${} | {} | {} | ${} | {} |\n",
            time, f.coin, side, f.px, f.sz, f.dir, f.fee, pnl_str,
        ));
    }

    Ok(CallToolResult::success(vec![Content::text(output)]))
}

pub async fn get_order_status(
    state: &ServerState,
    req: GetOrderStatusRequest,
) -> Result<CallToolResult, ErrorData> {
    let address = state.require_address()?;

    let update = state
        .client
        .order_status(address, Either::Left(req.order_id))
        .await
        .map_err(|e| mcp_err(&format!("Failed to fetch order status: {e}")))?;

    let update = match update {
        Some(u) => u,
        None => {
            return Ok(CallToolResult::error(vec![Content::text(format!(
                "Order {} not found.",
                req.order_id
            ))]));
        }
    };

    let o = &update.order;
    let side = match o.side {
        hypersdk::hypercore::Side::Bid => "Buy",
        hypersdk::hypercore::Side::Ask => "Sell",
    };
    let time = chrono_from_ms(update.status_timestamp);

    let output = format!(
        "## Order {} — {}\n\n\
         | Field | Value |\n\
         |-------|-------|\n\
         | Coin | {} |\n\
         | Side | {} |\n\
         | Price | ${} |\n\
         | Remaining Size | {} |\n\
         | Original Size | {} |\n\
         | Status Time | {} |\n",
        o.oid, update.status, o.coin, side, o.limit_px, o.sz, o.orig_sz, time,
    );

    Ok(CallToolResult::success(vec![Content::text(output)]))
}

async fn get_cached_clearinghouse(
    state: &ServerState,
    address: hypersdk::Address,
) -> Result<hypersdk::hypercore::ClearinghouseState, ErrorData> {
    {
        let guard = state.cache.clearinghouse_cache.read().await;
        if let Some(cached) = guard.as_ref() {
            if cached.is_fresh(POSITIONS_TTL) {
                tracing::debug!("clearinghouse cache hit");
                return Ok(cached.value.clone());
            }
        }
    }

    let user_state = state
        .client
        .clearinghouse_state(address, None)
        .await
        .map_err(|e| mcp_err(&format!("Failed to fetch positions: {e}")))?;

    *state.cache.clearinghouse_cache.write().await = Some(CachedValue::new(user_state.clone()));
    Ok(user_state)
}

async fn get_cached_open_orders(
    state: &ServerState,
    address: hypersdk::Address,
) -> Result<Vec<hypersdk::hypercore::types::BasicOrder>, ErrorData> {
    {
        let guard = state.cache.open_orders_cache.read().await;
        if let Some(cached) = guard.as_ref() {
            if cached.is_fresh(OPEN_ORDERS_TTL) {
                tracing::debug!("open_orders cache hit");
                return Ok(cached.value.clone());
            }
        }
    }

    let orders = state
        .client
        .open_orders(address, None)
        .await
        .map_err(|e| mcp_err(&format!("Failed to fetch open orders: {e}")))?;

    *state.cache.open_orders_cache.write().await = Some(CachedValue::new(orders.clone()));
    Ok(orders)
}

fn chrono_from_ms(ms: u64) -> String {
    let secs = (ms / 1000) as i64;
    let nanos = ((ms % 1000) * 1_000_000) as u32;
    chrono::DateTime::from_timestamp(secs, nanos)
        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| ms.to_string())
}
