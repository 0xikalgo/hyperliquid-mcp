use std::time::Duration;

use rmcp::{model::*, schemars};
use serde_json::json;

use crate::cache::CachedValue;
use crate::state::{ServerState, mcp_err};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetMarketsRequest {
    #[schemars(description = "Filter by market type: \"perp\", \"spot\", or \"all\"")]
    pub market_type: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetMarketSummaryRequest {
    #[schemars(description = "Trading pair symbol, e.g. \"BTC\", \"ETH\", \"PURR/USDC\"")]
    pub coin: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetOrderBookRequest {
    #[schemars(description = "Trading pair symbol, e.g. \"BTC\", \"ETH\"")]
    pub coin: String,

    #[schemars(description = "Number of price levels per side (default 10, max 20)")]
    pub depth: Option<usize>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetCandlesRequest {
    #[schemars(description = "Trading pair symbol, e.g. \"BTC\", \"ETH\"")]
    pub coin: String,

    #[schemars(description = "Candle interval: \"1m\", \"5m\", \"15m\", \"1h\", \"4h\", \"1d\"")]
    pub interval: String,

    #[schemars(description = "Number of candles to return (default 100, max 5000)")]
    pub count: Option<u64>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct GetFundingRatesRequest {
    #[schemars(description = "Perpetual market symbol, e.g. \"BTC\", \"ETH\"")]
    pub coin: String,

    #[schemars(description = "Hours of funding history to return (default 24)")]
    pub lookback_hours: Option<u64>,
}

const MARKET_SUMMARY_TTL: Duration = Duration::from_secs(5);

pub async fn get_markets(
    state: &ServerState,
    req: GetMarketsRequest,
) -> Result<CallToolResult, ErrorData> {
    let market_type = req.market_type.as_deref().unwrap_or("all");

    let ws_mids = state.cache.all_mids.borrow().clone();
    let mids = if !ws_mids.is_empty() {
        tracing::debug!("get_markets: using WS-cached mid prices");
        ws_mids
    } else {
        state
            .client
            .all_mids(None)
            .await
            .map_err(|e| mcp_err(&format!("Failed to fetch mid prices: {e}")))?
    };

    let mut output = String::new();

    if market_type == "all" || market_type == "perp" {
        let meta_data = get_cached_meta(state).await?;
        let (universe, ctxs) = parse_meta_and_ctxs(&meta_data);

        let mut rows: Vec<(&str, String, f64)> = Vec::new();
        if let Some(universe) = universe {
            for (i, asset) in universe.iter().enumerate() {
                let name = asset.get("name").and_then(|n| n.as_str()).unwrap_or("?");
                let price = mids
                    .get(name)
                    .map(|p| format!("${p}"))
                    .unwrap_or_else(|| "N/A".into());
                let volume = ctxs
                    .and_then(|c| c.get(i))
                    .and_then(|ctx| ctx.get("dayNtlVlm"))
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);
                rows.push((name, price, volume));
            }
        }
        rows.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

        output.push_str(&format!("## Perpetual Markets ({} total)\n\n", rows.len()));
        output.push_str("| Market | Price | 24h Volume |\n");
        output.push_str("|--------|-------|------------|\n");
        for (name, price, volume) in &rows {
            output.push_str(&format!("| {} | {} | ${:.0} |\n", name, price, volume));
        }
        output.push('\n');
    }

    if market_type == "all" || market_type == "spot" {
        let spot_data = get_cached_spot_meta(state).await?;
        let (universe, ctxs) = parse_meta_and_ctxs(&spot_data);

        let mut rows: Vec<(&str, String, f64)> = Vec::new();
        if let Some(universe) = universe {
            for (i, asset) in universe.iter().enumerate() {
                let name = asset.get("name").and_then(|n| n.as_str()).unwrap_or("?");
                let price = mids
                    .get(name)
                    .map(|p| format!("${p}"))
                    .unwrap_or_else(|| "N/A".into());
                let volume = ctxs
                    .and_then(|c| c.get(i))
                    .and_then(|ctx| ctx.get("dayNtlVlm"))
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);
                rows.push((name, price, volume));
            }
        }
        rows.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

        output.push_str(&format!("## Spot Markets ({} total)\n\n", rows.len()));
        output.push_str("| Market | Price | 24h Volume |\n");
        output.push_str("|--------|-------|------------|\n");
        for (name, price, volume) in &rows {
            output.push_str(&format!("| {} | {} | ${:.0} |\n", name, price, volume));
        }
        output.push('\n');
    }

    Ok(CallToolResult::success(vec![Content::text(output)]))
}

fn parse_meta_and_ctxs(
    data: &serde_json::Value,
) -> (
    Option<&Vec<serde_json::Value>>,
    Option<&Vec<serde_json::Value>>,
) {
    let arr = data.as_array();
    let universe = arr
        .and_then(|a| a.first())
        .and_then(|meta| meta.get("universe"))
        .and_then(|u| u.as_array());
    let ctxs = arr.and_then(|a| a.get(1)).and_then(|c| c.as_array());
    (universe, ctxs)
}

pub async fn get_market_summary(
    state: &ServerState,
    req: GetMarketSummaryRequest,
) -> Result<CallToolResult, ErrorData> {
    let perp_data = get_cached_meta(state).await?;

    if let Some(arr) = perp_data.as_array() {
        if arr.len() == 2 {
            let meta = &arr[0];
            let ctxs = &arr[1];

            if let (Some(universe), Some(ctx_arr)) = (
                meta.get("universe").and_then(|u| u.as_array()),
                ctxs.as_array(),
            ) {
                for (i, asset) in universe.iter().enumerate() {
                    let name = asset.get("name").and_then(|n| n.as_str()).unwrap_or("");
                    if name.eq_ignore_ascii_case(&req.coin) {
                        if let Some(ctx) = ctx_arr.get(i) {
                            let mark_px =
                                ctx.get("markPx").and_then(|v| v.as_str()).unwrap_or("N/A");
                            let oracle_px = ctx
                                .get("oraclePx")
                                .and_then(|v| v.as_str())
                                .unwrap_or("N/A");
                            let funding =
                                ctx.get("funding").and_then(|v| v.as_str()).unwrap_or("N/A");
                            let open_interest = ctx
                                .get("openInterest")
                                .and_then(|v| v.as_str())
                                .unwrap_or("N/A");
                            let day_vlm = ctx
                                .get("dayNtlVlm")
                                .and_then(|v| v.as_str())
                                .unwrap_or("N/A");
                            let prev_day_px = ctx
                                .get("prevDayPx")
                                .and_then(|v| v.as_str())
                                .unwrap_or("N/A");
                            let premium =
                                ctx.get("premium").and_then(|v| v.as_str()).unwrap_or("N/A");
                            let max_leverage = asset
                                .get("maxLeverage")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0);

                            let output = format!(
                                "## {name} Perpetual Market\n\n\
                                 | Metric | Value |\n\
                                 |--------|-------|\n\
                                 | Mark Price | ${mark_px} |\n\
                                 | Oracle Price | ${oracle_px} |\n\
                                 | Funding Rate | {funding} |\n\
                                 | Premium | {premium} |\n\
                                 | Open Interest | ${open_interest} |\n\
                                 | 24h Volume | ${day_vlm} |\n\
                                 | Previous Day Price | ${prev_day_px} |\n\
                                 | Max Leverage | {max_leverage}x |\n"
                            );
                            return Ok(CallToolResult::success(vec![Content::text(output)]));
                        }
                    }
                }
            }
        }
    }

    let spot_data = get_cached_spot_meta(state).await?;

    if let Some(arr) = spot_data.as_array() {
        if arr.len() == 2 {
            let meta = &arr[0];
            let ctxs = &arr[1];

            if let (Some(universe), Some(ctx_arr)) = (
                meta.get("universe").and_then(|u| u.as_array()),
                ctxs.as_array(),
            ) {
                for (i, pair) in universe.iter().enumerate() {
                    let name = pair.get("name").and_then(|n| n.as_str()).unwrap_or("");
                    if name.eq_ignore_ascii_case(&req.coin) {
                        if let Some(ctx) = ctx_arr.get(i) {
                            let mark_px =
                                ctx.get("markPx").and_then(|v| v.as_str()).unwrap_or("N/A");
                            let mid_px = ctx.get("midPx").and_then(|v| v.as_str()).unwrap_or("N/A");
                            let day_vlm = ctx
                                .get("dayNtlVlm")
                                .and_then(|v| v.as_str())
                                .unwrap_or("N/A");
                            let prev_day_px = ctx
                                .get("prevDayPx")
                                .and_then(|v| v.as_str())
                                .unwrap_or("N/A");

                            let output = format!(
                                "## {name} Spot Market\n\n\
                                 | Metric | Value |\n\
                                 |--------|-------|\n\
                                 | Mark Price | ${mark_px} |\n\
                                 | Mid Price | ${mid_px} |\n\
                                 | 24h Volume | ${day_vlm} |\n\
                                 | Previous Day Price | ${prev_day_px} |\n"
                            );
                            return Ok(CallToolResult::success(vec![Content::text(output)]));
                        }
                    }
                }
            }
        }
    }

    Ok(CallToolResult::error(vec![Content::text(format!(
        "Market '{}' not found. Use get_markets to see available markets.",
        req.coin
    ))]))
}

async fn get_cached_meta(state: &ServerState) -> Result<serde_json::Value, ErrorData> {
    {
        let guard = state.cache.meta_cache.read().await;
        if let Some(cached) = guard.as_ref() {
            if cached.is_fresh(MARKET_SUMMARY_TTL) {
                tracing::debug!("get_market_summary: meta cache hit");
                return Ok(cached.value.clone());
            }
        }
    }

    let data = state
        .raw_info_request(json!({"type": "metaAndAssetCtxs"}))
        .await?;

    *state.cache.meta_cache.write().await = Some(CachedValue::new(data.clone()));
    Ok(data)
}

async fn get_cached_spot_meta(state: &ServerState) -> Result<serde_json::Value, ErrorData> {
    {
        let guard = state.cache.spot_meta_cache.read().await;
        if let Some(cached) = guard.as_ref() {
            if cached.is_fresh(MARKET_SUMMARY_TTL) {
                tracing::debug!("get_market_summary: spot meta cache hit");
                return Ok(cached.value.clone());
            }
        }
    }

    let data = state
        .raw_info_request(json!({"type": "spotMetaAndAssetCtxs"}))
        .await?;

    *state.cache.spot_meta_cache.write().await = Some(CachedValue::new(data.clone()));
    Ok(data)
}

pub async fn get_order_book(
    state: &ServerState,
    req: GetOrderBookRequest,
) -> Result<CallToolResult, ErrorData> {
    let depth = req.depth.unwrap_or(10).min(20);

    let book = state
        .raw_info_request(json!({
            "type": "l2Book",
            "coin": req.coin,
        }))
        .await?;

    let levels = book.get("levels").and_then(|l| l.as_array());
    let levels = match levels {
        Some(l) if l.len() >= 2 => l,
        _ => {
            return Ok(CallToolResult::error(vec![Content::text(format!(
                "No order book data for '{}'",
                req.coin
            ))]));
        }
    };

    let bids = levels[0].as_array();
    let asks = levels[1].as_array();

    let mut output = format!("## {} Order Book\n\n", req.coin);

    // Asks (reversed so highest price is at top)
    output.push_str("### Asks (Sells)\n");
    output.push_str("| Price | Size | Orders |\n");
    output.push_str("|-------|------|--------|\n");
    if let Some(asks) = asks {
        let ask_slice: Vec<_> = asks.iter().take(depth).collect();
        for ask in ask_slice.iter().rev() {
            let px = ask.get("px").and_then(|v| v.as_str()).unwrap_or("?");
            let sz = ask.get("sz").and_then(|v| v.as_str()).unwrap_or("?");
            let n = ask.get("n").and_then(|v| v.as_u64()).unwrap_or(0);
            output.push_str(&format!("| ${px} | {sz} | {n} |\n"));
        }
    }

    output.push_str("\n### Bids (Buys)\n");
    output.push_str("| Price | Size | Orders |\n");
    output.push_str("|-------|------|--------|\n");
    if let Some(bids) = bids {
        for bid in bids.iter().take(depth) {
            let px = bid.get("px").and_then(|v| v.as_str()).unwrap_or("?");
            let sz = bid.get("sz").and_then(|v| v.as_str()).unwrap_or("?");
            let n = bid.get("n").and_then(|v| v.as_u64()).unwrap_or(0);
            output.push_str(&format!("| ${px} | {sz} | {n} |\n"));
        }
    }

    Ok(CallToolResult::success(vec![Content::text(output)]))
}

pub async fn get_candles(
    state: &ServerState,
    req: GetCandlesRequest,
) -> Result<CallToolResult, ErrorData> {
    let valid_intervals = [
        "1m", "3m", "5m", "15m", "30m", "1h", "2h", "4h", "8h", "12h", "1d", "3d", "1w", "1M",
    ];
    if !valid_intervals.contains(&req.interval.as_str()) {
        return Ok(CallToolResult::error(vec![Content::text(
            "Invalid interval. Use: 1m, 3m, 5m, 15m, 30m, 1h, 2h, 4h, 8h, 12h, 1d, 3d, 1w, 1M",
        )]));
    }

    let count = req.count.unwrap_or(100).min(5000);

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    let interval_ms: u64 = match req.interval.as_str() {
        "1m" => 60_000,
        "3m" => 180_000,
        "5m" => 300_000,
        "15m" => 900_000,
        "30m" => 1_800_000,
        "1h" => 3_600_000,
        "2h" => 7_200_000,
        "4h" => 14_400_000,
        "8h" => 28_800_000,
        "12h" => 43_200_000,
        "1d" => 86_400_000,
        "3d" => 259_200_000,
        "1w" => 604_800_000,
        "1M" => 2_592_000_000,
        _ => 3_600_000,
    };
    let start_time = now_ms.saturating_sub(count * interval_ms);

    let candle_interval = parse_candle_interval(&req.interval)
        .ok_or_else(|| mcp_err(&format!("Unsupported candle interval: {}", req.interval)))?;

    let candles = state
        .client
        .candle_snapshot(req.coin.clone(), candle_interval, start_time, now_ms)
        .await
        .map_err(|e| mcp_err(&format!("Failed to fetch candles: {e}")))?;

    let mut output = format!(
        "## {} Candles ({}, {} periods)\n\n",
        req.coin,
        req.interval,
        candles.len()
    );
    output.push_str("| Time | Open | High | Low | Close | Volume |\n");
    output.push_str("|------|------|------|-----|-------|--------|\n");

    let display_count = candles.len().min(count as usize);
    for candle in candles.iter().rev().take(display_count).rev() {
        let time = chrono_from_ms(candle.open_time);
        output.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} |\n",
            time, candle.open, candle.high, candle.low, candle.close, candle.volume
        ));
    }

    Ok(CallToolResult::success(vec![Content::text(output)]))
}

pub async fn get_funding_rates(
    state: &ServerState,
    req: GetFundingRatesRequest,
) -> Result<CallToolResult, ErrorData> {
    let hours = req.lookback_hours.unwrap_or(24);
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    let start_ms = now_ms.saturating_sub(hours * 3_600_000);

    let rates = state
        .client
        .funding_history(req.coin.clone(), start_ms, None)
        .await
        .map_err(|e| mcp_err(&format!("Failed to fetch funding history: {e}")))?;

    let mut output = format!(
        "## {} Funding Rates (last {} hours, {} entries)\n\n",
        req.coin,
        hours,
        rates.len()
    );
    output.push_str("| Time | Funding Rate | Premium |\n");
    output.push_str("|------|-------------|---------|\n");

    for rate in rates.iter().rev().take(100) {
        let time = chrono_from_ms(rate.time);
        output.push_str(&format!(
            "| {} | {} | {} |\n",
            time, rate.funding_rate, rate.premium
        ));
    }

    Ok(CallToolResult::success(vec![Content::text(output)]))
}

fn parse_candle_interval(s: &str) -> Option<hypersdk::hypercore::CandleInterval> {
    use hypersdk::hypercore::CandleInterval;
    match s {
        "1m" => Some(CandleInterval::OneMinute),
        "3m" => Some(CandleInterval::ThreeMinutes),
        "5m" => Some(CandleInterval::FiveMinutes),
        "15m" => Some(CandleInterval::FifteenMinutes),
        "30m" => Some(CandleInterval::ThirtyMinutes),
        "1h" => Some(CandleInterval::OneHour),
        "2h" => Some(CandleInterval::TwoHours),
        "4h" => Some(CandleInterval::FourHours),
        "8h" => Some(CandleInterval::EightHours),
        "12h" => Some(CandleInterval::TwelveHours),
        "1d" => Some(CandleInterval::OneDay),
        "3d" => Some(CandleInterval::ThreeDays),
        "1w" => Some(CandleInterval::OneWeek),
        "1M" => Some(CandleInterval::OneMonth),
        _ => None,
    }
}

fn chrono_from_ms(ms: u64) -> String {
    let secs = (ms / 1000) as i64;
    let nanos = ((ms % 1000) * 1_000_000) as u32;
    chrono::DateTime::from_timestamp(secs, nanos)
        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| ms.to_string())
}
