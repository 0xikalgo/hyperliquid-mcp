# Tools Reference

All tools exposed by the Hyperliquid MCP server.

## Market Data Tools

These tools require no authentication and work in read-only mode.

### `get_markets`

List all available markets with current prices.

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `market_type` | string | No | `"all"` | Filter: `"perp"`, `"spot"`, or `"all"` |

**Example:** "Show me all perpetual markets on Hyperliquid"

### `get_market_summary`

Detailed stats for a specific market including funding rate, open interest, and 24h volume.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `coin` | string | Yes | Symbol, e.g. `"BTC"`, `"ETH"`, `"PURR/USDC"` |

**Example:** "What's the current state of the ETH market?"

### `get_order_book`

L2 order book with bids and asks.

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `coin` | string | Yes | — | Symbol |
| `depth` | number | No | `10` | Levels per side (max 20) |

**Example:** "Show me the top 5 levels of the BTC order book"

### `get_candles`

OHLCV candlestick data.

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `coin` | string | Yes | — | Symbol |
| `interval` | string | Yes | — | `"1m"`, `"5m"`, `"15m"`, `"1h"`, `"4h"`, `"1d"` |
| `count` | number | No | `100` | Number of candles (max 5000) |

**Example:** "Get the last 24 hourly candles for ETH"

### `get_funding_rates`

Current and historical funding rates for perpetuals.

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `coin` | string | Yes | — | Perpetual symbol |
| `lookback_hours` | number | No | `24` | Hours of history |

**Example:** "What's the BTC funding rate over the last 48 hours?"

---

## Account Tools

These tools require `HYPERLIQUID_AGENT_PRIVATE_KEY` to be set.

### `get_positions`

All open perpetual positions with PnL, leverage, and liquidation prices.

No parameters.

**Example:** "What are my current positions?"

### `get_balances`

Account balances for both perpetual and spot accounts.

No parameters.

**Example:** "How much money do I have available?"

### `get_open_orders`

All open orders, optionally filtered by market.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `coin` | string | No | Filter by symbol |

**Example:** "Show my open orders for BTC"

### `get_trade_history`

Recent trade fills.

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `coin` | string | No | — | Filter by symbol |
| `limit` | number | No | `50` | Number of trades (max 200) |

**Example:** "Show my last 10 trades"

---

## Trading Tools

These tools execute real trades. They require authentication.

### `place_order`

Place a new limit or market order.

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `coin` | string | Yes | — | Symbol |
| `side` | string | Yes | — | `"buy"` or `"sell"` |
| `size` | string | Yes | — | Size in coin units (e.g. `"0.01"`) |
| `price` | string | Limit only | — | Limit price in USD |
| `order_type` | string | No | `"limit"` | `"limit"` or `"market"` |
| `time_in_force` | string | No | `"gtc"` | `"gtc"`, `"ioc"`, `"alo"` (post-only) |
| `reduce_only` | boolean | No | `false` | Only reduce existing position |

**Examples:**
- "Buy 0.01 BTC at $85,000" → limit buy
- "Market sell 1 ETH" → market sell
- "Place a post-only buy of 0.5 SOL at $140" → ALO limit buy

### `cancel_order`

Cancel a specific order.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `coin` | string | Yes | Symbol |
| `order_id` | number | Yes | Order ID (from get_open_orders) |

### `cancel_all_orders`

Cancel all open orders.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `coin` | string | No | Cancel only for this market |

**Example:** "Cancel all my orders" or "Cancel all BTC orders"

### `modify_order`

Change the price and/or size of an existing order.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `order_id` | number | Yes | Order ID to modify |
| `coin` | string | Yes | Symbol |
| `side` | string | Yes | `"buy"` or `"sell"` |
| `new_price` | string | Yes | New limit price |
| `new_size` | string | Yes | New order size |

### `set_leverage`

Update leverage for a market.

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `coin` | string | Yes | — | Symbol |
| `leverage` | number | Yes | — | Multiplier (e.g. 10) |
| `mode` | string | No | `"cross"` | `"cross"` or `"isolated"` |

### `close_position`

Close an entire position at market price.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `coin` | string | Yes | Symbol |

**Example:** "Close my ETH position"

---

## Transfer & Fee Tools

### `transfer_between_spot_perps`

Move USDC between spot and perpetual accounts.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `amount` | string | Yes | USDC amount |
| `direction` | string | Yes | `"to_spot"` or `"to_perps"` |

### `check_builder_fee`

Check builder fee status and get approval instructions.

No parameters.

**Example:** "What are the builder fees on this server?"
