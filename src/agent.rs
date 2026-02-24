//! Agent wallet creation and management utilities.
//!
//! Uses hypersdk's `approve_agent()` method for reliable EIP-712 signing.

use alloy::signers::local::PrivateKeySigner;
use anyhow::Result;
use hypersdk::Address;

use crate::config;

pub async fn create_agent_wallet(
    client: &hypersdk::hypercore::HttpClient,
    main_signer: &PrivateKeySigner,
    nonce: u64,
) -> Result<String> {
    let agent_signer = PrivateKeySigner::random();
    let agent_address = agent_signer.address();
    let agent_name = agent_name_today();

    client
        .approve_agent(main_signer, agent_address, agent_name, nonce)
        .await
        .map_err(|e| anyhow::anyhow!("Agent wallet creation failed: {e}"))?;

    tracing::info!(address = %agent_address, "Agent wallet approved via hypersdk");

    // Return the private key hex (without 0x prefix)
    let key_b256 = agent_signer.to_bytes();
    Ok(format!("{:x}", key_b256))
}

pub fn agent_name_today() -> String {
    chrono::Utc::now().format("hlmcp-%m%d%y").to_string()
}

pub fn save_agent_key_to_env(agent_key_hex: &str) -> Result<std::path::PathBuf> {
    use std::io::Write;

    let home =
        dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;
    let env_dir = home.join(".config/hyperliquid-mcp");
    std::fs::create_dir_all(&env_dir)?;

    let env_path = home.join(config::ENV_FILE_PATH);
    let tmp_path = env_dir.join(".env.tmp");

    let existing = std::fs::read_to_string(&env_path).unwrap_or_default();
    let mut lines: Vec<String> = existing
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.starts_with("HYPERLIQUID_AGENT_PRIVATE_KEY=")
        })
        .map(|l| l.to_string())
        .collect();

    lines.insert(
        0,
        format!("HYPERLIQUID_AGENT_PRIVATE_KEY=0x{agent_key_hex}"),
    );

    // Write atomically
    {
        let mut f = std::fs::File::create(&tmp_path)?;
        f.write_all(lines.join("\n").as_bytes())?;
        f.write_all(b"\n")?;
        f.sync_all()?;
    }

    // Set permissions to 0600 (owner read/write only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp_path, std::fs::Permissions::from_mode(0o600))?;
    }

    std::fs::rename(&tmp_path, &env_path)?;

    Ok(env_path)
}

pub fn wallet_and_address(key_hex: &str) -> Result<(PrivateKeySigner, Address)> {
    let signer: PrivateKeySigner = key_hex
        .parse()
        .map_err(|e| anyhow::anyhow!("Failed to parse agent key: {e}"))?;
    let address = signer.address();
    Ok((signer, address))
}
