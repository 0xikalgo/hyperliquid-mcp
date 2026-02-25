# hyperliquid-mcp

[![CI](https://github.com/0xikalgo/hyperliquid-mcp/actions/workflows/ci.yml/badge.svg)](https://github.com/0xikalgo/hyperliquid-mcp/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/hyperliquid-mcp)](https://crates.io/crates/hyperliquid-mcp)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

MCP server for trading on [Hyperliquid](https://hyperliquid.xyz) through AI agents.

Trade perpetual futures and spot assets using natural language with Claude, Cursor, or any MCP-compatible client.

> This MCP server is under active development and may change without notice. Always use a dedicated [agent wallet](https://app.hyperliquid.xyz/API) — never your main wallet. There are no guarantees about actions your AI agent may take.

## Quick Start

**1. Install**

Requires [Rust](https://www.rust-lang.org/tools/install) 1.85+. If you don't have it:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Then install the MCP server:

```bash
cargo install hyperliquid -- hyperliquid-mcp
```

**2. Configure your wallet** — create `~/.config/hyperliquid-mcp/.env` with your **main wallet** private key:

```env
# ~/.config/hyperliquid-mcp/.env
HYPERLIQUID_PRIVATE_KEY=0xyour_main_wallet_key
HYPERLIQUID_NETWORK=mainnet
```

On first run, the server will automatically:
- Create a dedicated named agent wallet for trading
- Approve builder fees (0.01% to support development)
- Save the agent key to your `.env` file
- Continue running normally

After setup, your `.env` will contain both keys. The server uses `HYPERLIQUID_AGENT_PRIVATE_KEY` for trading.

> **Already have an agent wallet?** Skip the auto-setup — just put `HYPERLIQUID_AGENT_PRIVATE_KEY=0xyour_agent_key` in the `.env` file. See [Private Key Options](#private-key-options).

**3. Add to your MCP client**

<details>
<summary><strong>Claude Code</strong></summary>

```bash
claude mcp add hyperliquid-mcp
```

</details>

<details>
<summary><strong>Claude Desktop</strong></summary>

Edit `~/Library/Application Support/Claude/claude_desktop_config.json` (macOS) or `%APPDATA%\Claude\claude_desktop_config.json` (Windows):

```json
{
  "mcpServers": {
    "hyperliquid": {
      "command": "hyperliquid-mcp"
    }
  }
}
```

Restart Claude Desktop after saving.

</details>

<details>
<summary><strong>Cursor</strong></summary>

Edit `~/.cursor/mcp.json`:

```json
{
  "mcpServers": {
    "hyperliquid": {
      "command": "hyperliquid-mcp"
    }
  }
}
```

</details>

You may need to symlink `hyperliquid-mcp` if your client cannot connect to the MCP.

```bash
$ sudo ln -s ~/.cargo/bin/hyperliquid-mcp /usr/local/bin/hyperliquid-mcp
```

**4. Start trading** — ask Claude things like *"What are the top Hyperliquid markets?"* or *"Buy 0.01 BTC at $85,000"*.

> **Builder fees** (0.01%) are automatically approved during setup. If you set up manually, see [Builder Fees](docs/builder-fees.md) for approval instructions.

## Tools

See [tools](docs/tools-reference.md).

## Private Key Options

The server checks for your private key in the following order:

### 1. `.env` file (recommended)

The server automatically loads `~/.config/hyperliquid-mcp/.env` on startup. Your key stays in one secured file, outside of any MCP client config.

```env
# ~/.config/hyperliquid-mcp/.env
HYPERLIQUID_AGENT_PRIVATE_KEY=0xyour_agent_key
HYPERLIQUID_NETWORK=mainnet
```

### 2. Inline environment variable

Pass the key directly in your MCP client config. Simpler, but the key ends up in a plaintext JSON file that may be synced by cloud backups or accidentally committed.

```json
{
  "mcpServers": {
    "hyperliquid": {
      "command": "hyperliquid-mcp",
      "env": {
        "HYPERLIQUID_AGENT_PRIVATE_KEY": "0xyour_agent_key"
      }
    }
  }
}
```

### 3. Shell environment

Export the variable in your shell before running the server, or use `claude mcp add-env`:

```bash
# Shell export
export HYPERLIQUID_AGENT_PRIVATE_KEY=0xyour_agent_key
hyperliquid-mcp

# Or via Claude Code
claude mcp add-env hyperliquid HYPERLIQUID_AGENT_PRIVATE_KEY 0xyour_agent_key
```

> **Note:** Explicit environment variables always take precedence over the `.env` file. If `HYPERLIQUID_AGENT_PRIVATE_KEY` is already set in your environment, the `.env` value is ignored.

### No key (read-only mode)

If no private key is provided, the server runs in read-only mode. All market data tools work — you can check prices, order books, and funding rates without authentication.

## Configuration

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `HYPERLIQUID_AGENT_PRIVATE_KEY` | No | — | Agent wallet private key. Read-only without it. |
| `HYPERLIQUID_PRIVATE_KEY` | No | — | Main wallet key for first-time setup (agent creation + builder fee approval). Remove after setup. |
| `HYPERLIQUID_NETWORK` | No | `mainnet` | `mainnet` or `testnet` |
| `REALTIME_ENABLED` | No | `true` | Set to `false` to disable WebSocket streaming and use HTTP-only |

## Security

**Prioritize using an agent wallet — never your main wallet key.**

Create one at [app.hyperliquid.xyz/API](https://app.hyperliquid.xyz/API). Agent wallets are trade-only keys with strict limitations:

| Action | Agent Wallet | Main Wallet |
|--------|:------------:|:-----------:|
| Place, cancel, modify orders | Yes | Yes |
| Update leverage / margin | Yes | Yes |
| **Withdraw to L1** | **No** | Yes |
| **Transfer USDC / spot tokens** | **No** | Yes |
| **Approve builder fees** | **No** | Yes |

If an agent key is ever compromised, your funds remain safe — agent wallets cannot move money out of your account.

### Agent wallet expiration

Agent wallets expire after a maximum of **180 days**. When one expires:

- All actions signed by it are rejected — open orders remain, but nothing new can be placed or cancelled
- You must create a new agent wallet and update your config
- **Never reuse an expired key** — generate a fresh one each time

Check expiration dates at [app.hyperliquid.xyz/API](https://app.hyperliquid.xyz/API). If you suspect a key is compromised, revoke it there immediately.

### Other security notes

- **Use the `.env` file** — keep your private key in `~/.config/hyperliquid-mcp/.env`, not inline in MCP client configs. See [Private Key Options](#private-key-options).
- Private key is read from env only — never logged or sent over the network.
- Read-only mode works without any key.

See [docs/configuration.md](docs/configuration.md) for the full security guide.

## Docs

- [Tools Reference](docs/tools-reference.md)
- [Configuration & Security](docs/configuration.md)
- [Builder Fees](docs/builder-fees.md)

## License

MIT
