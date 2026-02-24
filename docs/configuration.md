# Configuration

## Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `HYPERLIQUID_AGENT_PRIVATE_KEY` | No | — | Agent wallet private key (hex, with or without `0x` prefix). If not set, runs in read-only mode. |
| `HYPERLIQUID_PRIVATE_KEY` | No | — | Master wallet private key. (hex, with or without `0x` prefix). If not set, runs in read-only mode. |
| `HYPERLIQUID_NETWORK` | No | `mainnet` | `mainnet` or `testnet` |
| `RUST_LOG` | No | — | Logging level. Set to `hyperliquid_mcp=debug` for verbose output. |

## Private Key Safety

Your agent key is the most sensitive piece of configuration. How you provide it matters.

### Recommended: `.env` file

The server automatically loads `~/.config/hyperliquid-mcp/.env` on startup. Keep your key there — outside any git repository, in a single file you can easily rotate:

```env
# ~/.config/hyperliquid-mcp/.env
HYPERLIQUID_AGENT_PRIVATE_KEY=0xyour_agent_wallet_key_here
HYPERLIQUID_NETWORK=mainnet
```

Your MCP client config stays key-free:

```json
{
  "mcpServers": {
    "hyperliquid": {
      "command": "hyperliquid-mcp"
    }
  }
}
```

### Alternative: Inline in MCP config

You can put the key directly in `claude_desktop_config.json` or `mcp.json`. 

```json
{
  "mcpServers": {
    "hyperliquid": {
      "command": "hyperliquid-mcp",
      "env": {
        "HYPERLIQUID_AGENT_PRIVATE_KEY": "0xyour_key_here"
      }
    }
  }
}
```

### Never:

- **Commit a private key to git** — even in a `.env` file inside a repo
- **Paste a key in a chat message** to Claude or any AI
- **Store a key in a cloud document** (Google Docs, Notion, etc.)

## Agent Wallets (API Wallets)

Agent wallets are the recommended way to authenticate with this MCP server. They are trade-only signing keys authorized by your main wallet.

### Expiration

Agent wallets have a maximum lifetime of **180 days**. When you create an agent in the UI, you set "Days Valid" (up to 180).

When an agent expires:
- Actions signed by it are rejected by the protocol
- You must create a new agent wallet and update your MCP config
- **Never reuse an expired/revoked agent address** — always generate a fresh keypair. Reusing an old address can create replay vulnerabilities because Hyperliquid may prune the nonce state for deregistered agents.

### Revoking an agent

1. Go to [app.hyperliquid.xyz/API](https://app.hyperliquid.xyz/API)
2. Find the agent in the list
3. Click **Remove** and sign with your main wallet

## Logging

Logs are written to **stderr** (never stdout, which is reserved for MCP protocol messages). Set verbosity via `RUST_LOG`:

```bash
# No logging (default)
RUST_LOG= hyperliquid-mcp

# Info level
RUST_LOG=hyperliquid_mcp=info hyperliquid-mcp

# Debug level (verbose)
RUST_LOG=hyperliquid_mcp=debug hyperliquid-mcp

# All dependencies too
RUST_LOG=debug hyperliquid-mcp
```

In Claude Desktop config (if using inline env):
```json
{
  "mcpServers": {
    "hyperliquid": {
      "command": "hyperliquid-mcp",
      "env": {
        "HYPERLIQUID_AGENT_PRIVATE_KEY": "0x...",
        "RUST_LOG": "hyperliquid_mcp=info"
      }
    }
  }
}
```

## Networks

| Network | API Endpoint | Use Case |
|---------|-------------|----------|
| `mainnet` | api.hyperliquid.xyz | Real trading with real funds |
| `testnet` | api.hyperliquid-testnet.xyz | Testing and development |
