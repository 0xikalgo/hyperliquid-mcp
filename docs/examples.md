# Example Conversations

This guide walks you through real conversations you can have with Claude once the Hyperliquid MCP server is set up. 

> **Note:** These examples assume you have already installed and configured the server. If you have not done that yet, see the [README](README.md).

---

## 1. Checking the Markets

The simplest thing to start with is just looking around. You do not need a wallet or private key for any of these -- market data is public.

**Browsing available markets:**

> **You:** What markets can I trade on Hyperliquid?
>
> **Claude:** Hyperliquid has 150+ perpetual markets and a growing number of spot markets. Here are some of the highlights:
>
> | Market | Price | Max Leverage |
> |--------|-------|--------------|
> | BTC | $97,432.00 | 50x |
> | ETH | $3,245.10 | 50x |
> | SOL | $178.62 | 20x |
> | DOGE | $0.2143 | 20x |
> | ... | ... | ... |
>
> Would you like to see the full list, or just perpetuals or spot markets?

Claude pulls live prices from Hyperliquid when answering, so the numbers you see will always be current.

**Getting a price:**

> **You:** What's the current price of ETH?
>
> **Claude:** Here is the current summary for ETH:
>
> | Metric | Value |
> |--------|-------|
> | Mark Price | $3,245.10 |
> | Oracle Price | $3,244.87 |
> | Funding Rate | 0.0042% |
> | Open Interest | $1,245,678,900 |
> | 24h Volume | $3,456,789,012 |
> | Max Leverage | 50x |
>
> ETH is up about 2.3% from yesterday's close of $3,172.50.

**Looking at the order book:**

> **You:** Show me the BTC order book.
>
> **Claude:** Here is the BTC order book (top 10 levels on each side):
>
> **Asks (people selling):**
>
> | Price | Size | Orders |
> |-------|------|--------|
> | $97,450 | 2.15 | 8 |
> | $97,445 | 1.82 | 5 |
> | $97,440 | 3.47 | 12 |
> | ... | ... | ... |
>
> **Bids (people buying):**
>
> | Price | Size | Orders |
> |-------|------|--------|
> | $97,430 | 4.21 | 15 |
> | $97,425 | 2.68 | 9 |
> | $97,420 | 1.93 | 7 |
> | ... | ... | ... |
>
> The spread is about $10 right now, which is very tight. There is solid liquidity on both sides.

---

## 2. Viewing Your Portfolio

These actions require your wallet to be connected. Claude will let you know if it is not configured.

**Checking your positions:**

> **You:** What are my open positions?
>
> **Claude:** You have 2 open positions:
>
> | Market | Side | Size | Entry Price | Unrealized PnL | ROE | Liq. Price |
> |--------|------|------|-------------|----------------|-----|------------|
> | ETH | Long | 1.5 | $3,100.00 | +$217.65 | +7.02% | $2,480.00 |
> | SOL | Short | 50 | $185.40 | +$339.00 | +4.61% | $232.50 |
>
> Your ETH long is doing well -- up over 7%. The SOL short is also profitable. Neither position is anywhere near liquidation.

**Checking your balances:**

> **You:** How much money do I have?
>
> **Claude:** Here is your account breakdown:
>
> **Perpetual Account:**
>
> | Metric | Value |
> |--------|-------|
> | Account Value | $12,456.78 |
> | Total Margin Used | $4,200.00 |
> | Available Margin | $8,256.78 |
> | Withdrawable | $7,890.12 |
>
> **Spot Balances:**
>
> | Token | Total | Available |
> |-------|-------|-----------|
> | USDC | 500.00 | 500.00 |
> | PURR | 10,000 | 10,000 |
>
> You have about $8,200 of available margin for new trades on the perps side, plus $500 USDC sitting in your spot account.

**Reviewing recent trades:**

> **You:** Show my recent trades.
>
> **Claude:** Here are your last few trades:
>
> | Time | Market | Side | Price | Size | Fee | Closed PnL |
> |------|--------|------|-------|------|-----|------------|
> | 2025-02-24 14:32 | ETH | Buy | $3,100.00 | 1.5 | $0.93 | -- |
> | 2025-02-24 12:15 | SOL | Sell | $185.40 | 50 | $0.46 | -- |
> | 2025-02-23 09:44 | BTC | Sell | $96,800.00 | 0.05 | $1.21 | +$125.00 |
>
> Your most recent activity was opening the ETH and SOL positions. Before that, you closed a BTC trade yesterday for a $125 profit.

---

## 3. Placing Your First Trade

When you tell Claude you want to trade, it will ask for the details it needs and confirm everything before placing the order. Real money is involved, so Claude always double-checks.

**Starting a trade:**

> **You:** I want to buy some ETH.
>
> **Claude:** Sure, I can help you place an ETH order. I just need a few details:
>
> 1. **How much?** -- How many ETH do you want to buy? (e.g., 0.1 ETH)
> 2. **At what price?** -- Do you want a limit order at a specific price, or a market order at the current price?
>
> ETH is currently trading at $3,245 for reference.

> **You:** Buy 0.1 ETH at $3,200.
>
> **Claude:** Just to confirm before I place this:
>
> - **Action:** Buy 0.1 ETH
> - **Type:** Limit order
> - **Price:** $3,200.00
> - **Total value:** ~$320
>
> This order will sit on the book until ETH drops to $3,200 or you cancel it. Want me to go ahead?

> **You:** Yes, go ahead.
>
> **Claude:** Done! Your order has been placed:
>
> - **Status:** Resting (order ID: 284719365)
> - Buy 0.1 ETH @ $3,200.00
>
> The order is now live on the book. ETH is at $3,245 right now, so it will fill if the price drops about 1.4%. I can check on it anytime -- just ask.

Notice that Claude confirmed the details before executing. It will always do this for trades since they involve real funds.

**Placing a market order:**

> **You:** Market buy 0.05 ETH right now.
>
> **Claude:** To confirm -- you want to buy 0.05 ETH at the current market price (approximately $3,245)? This will execute immediately. Go ahead?

> **You:** Yes.
>
> **Claude:** Order filled!
>
> - **Status:** Filled 0.05 @ $3,245.30 (order ID: 284719412)
>
> You now have a total ETH position of 0.05 (assuming no previous position). Your average entry is $3,245.30.

---

## 4. Managing Risk

Protecting your positions is important. You can ask Claude to set stop losses or close positions entirely.

**Setting a stop loss:**

> **You:** Set a stop loss on my ETH position at $3,000.
>
> **Claude:** I will place a stop loss for your ETH position. Since you are long 1.5 ETH, I will set a sell limit order at $3,000 with reduce-only so it only closes your position:
>
> - **Action:** Sell 1.5 ETH
> - **Price:** $3,000.00
> - **Reduce only:** Yes
>
> If ETH drops to $3,000, this order will trigger and close your position. Want me to place it?

> **You:** Do it.
>
> **Claude:** Stop loss placed:
>
> - **Status:** Resting (order ID: 284720001)
> - Sell 1.5 ETH @ $3,000.00 (reduce only)
>
> If ETH hits $3,000, your position will be closed. This would lock in a loss of about $150 versus your $3,100 entry, but it protects you from bigger moves down.

**Closing a position:**

> **You:** Close my BTC position.
>
> **Claude:** You have a BTC long position of 0.1 BTC (entry: $95,200). I will close it at market price. BTC is currently at $97,430.
>
> This should lock in a profit of approximately $223. Want me to close it?

> **You:** Yes, close it.
>
> **Claude:** Position closed!
>
> - **Status:** Filled 0.1 @ $97,428.50
> - **Realized PnL:** +$222.85
>
> Your BTC position is now flat. Nice trade!

---

## 5. Checking Funding Rates

If you trade perpetual futures, funding rates matter. They are small periodic payments between longs and shorts that keep the perp price in line with the spot price.

**Checking a specific rate:**

> **You:** What's the funding rate on BTC?
>
> **Claude:** Here is the current BTC funding data:
>
> | Time | Funding Rate | Premium |
> |------|-------------|---------|
> | 2025-02-24 16:00 | 0.0031% | 0.0012% |
> | 2025-02-24 08:00 | 0.0028% | 0.0009% |
> | 2025-02-24 00:00 | 0.0035% | 0.0015% |
>
> The current rate is **0.0031% per 8 hours**, which is positive. That means longs are paying shorts. On an annualized basis, this works out to about 3.4%.
>
> If you are long BTC, you are paying a small amount every 8 hours. If you are short, you are collecting it.

**Finding the highest funding:**

> **You:** Which markets have the highest funding right now?
>
> **Claude:** Let me check the top markets by funding rate. Here are the ones standing out right now:
>
> | Market | Funding Rate (8h) | Annualized | Direction |
> |--------|-------------------|------------|-----------|
> | DOGE | 0.0185% | ~20.3% | Longs pay shorts |
> | WIF | 0.0142% | ~15.6% | Longs pay shorts |
> | PEPE | 0.0128% | ~14.1% | Longs pay shorts |
> | BTC | 0.0031% | ~3.4% | Longs pay shorts |
> | ETH | -0.0015% | ~-1.6% | Shorts pay longs |
>
> The meme coins have the highest rates right now. DOGE longs are paying over 20% annualized to hold their positions. ETH funding is actually slightly negative, meaning shorts are paying longs -- unusual but it happens.

---

## 6. Managing Orders

You can view, cancel, or cancel all your open orders at any time.

**Viewing open orders:**

> **You:** Show my open orders.
>
> **Claude:** You have 3 open orders:
>
> | Market | Side | Price | Size | Type | Order ID |
> |--------|------|-------|------|------|----------|
> | ETH | Buy | $3,200.00 | 0.1 | Limit (GTC) | 284719365 |
> | ETH | Sell | $3,000.00 | 1.5 | Limit (GTC) | 284720001 |
> | BTC | Buy | $92,000.00 | 0.02 | Limit (GTC) | 284718990 |
>
> The first ETH order is your limit buy at $3,200. The second is your stop loss. The BTC order is a limit buy sitting well below the current price.

**Canceling a specific order:**

> **You:** Cancel my BTC order.
>
> **Claude:** Cancelled order 284718990 on BTC: Success.
>
> Your BTC buy at $92,000 has been removed. You still have 2 ETH orders open.

**Canceling everything:**

> **You:** Cancel all my orders.
>
> **Claude:** Cancelled 2/2 orders.
>
> All your open orders have been cleared. You have no pending orders on any market now. Keep in mind this also cancelled your ETH stop loss at $3,000 -- you may want to set a new one if you still want downside protection.

---

## 7. Moving Money

Hyperliquid has separate accounts for perpetual trading and spot trading. You can move USDC between them.

**Transferring to spot:**

> **You:** Transfer $500 from perps to spot.
>
> **Claude:** Transferred 500 USDC (perps to spot).
>
> Your spot balance should now reflect the additional $500. You can use it to trade spot markets or just hold it there.

**Transferring to perps:**

> **You:** Move $1,000 from spot to my perps account.
>
> **Claude:** Transferred 1000 USDC (spot to perps).
>
> That gives you an extra $1,000 of margin for perpetual trading.

---

## Tips for Getting the Most Out of It

- **Claude always confirms trades.** Before placing, modifying, or closing any order, Claude will show you what it is about to do and ask for confirmation. No surprises.

- **Start with read-only.** You can browse markets, check prices, and look at order books without configuring a wallet at all. Good for exploring before you commit.

- **Be specific when it helps.** "Buy 0.1 ETH at $3,200" is faster than "I want to buy some ETH" because Claude does not need to ask follow-up questions.

- **You can chain requests.** "Show me the ETH price, my open positions, and any open orders" will get you all three in one response.
