# Builder Fees

Builder fees are a small per-trade charge that supports the development of this MCP server. They are completely optional and transparent.

## How to Approve

Builder fee approval requires your **main wallet** (not your API/agent wallet). This is a Hyperliquid security feature — only the account owner can authorize fee deductions.

### Automatic Setup (Recommended)

The MCP server can handle agent wallet creation and builder fee approval in one step:

1. Add your main wallet key to `~/.config/hyperliquid-mcp/.env`:
   ```env
   # ~/.config/hyperliquid-mcp/.env
   HYPERLIQUID_PRIVATE_KEY=0xyour_main_wallet_key
   HYPERLIQUID_NETWORK=mainnet
   ```
   Make sure `HYPERLIQUID_AGENT_PRIVATE_KEY` is **not** set (remove or comment it out).

2. Run the server (`hyperliquid-mcp`). It will:
   - Create a dedicated named agent wallet
   - Approve builder fees (0.01%)
   - Save the agent key to your `.env` as `HYPERLIQUID_AGENT_PRIVATE_KEY`
   - Continue running normally

## Checking Status

Ask your AI assistant: "Check my builder fee status"

This calls the `check_builder_fee` tool, which will show:
- The builder address and fee rate
- Whether you've approved the builder
- Instructions for approving if you haven't

## Opting Out

Builder fees are enabled by default but you have full control and can opt out.
