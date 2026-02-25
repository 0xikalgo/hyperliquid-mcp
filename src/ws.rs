use std::collections::HashMap;
use std::sync::Arc;

use futures::StreamExt;
use hypersdk::Address;
use hypersdk::hypercore::types::{Incoming, Subscription};
use hypersdk::hypercore::ws::{ConnectionHandle, ConnectionStream, Event};
use hypersdk::hypercore::{self, Chain};
use rust_decimal::Decimal;
use serde_json::json;
use tokio::sync::watch;

use crate::cache::{CachedValue, WsCache};

pub fn cache_only() -> Arc<WsCache> {
    let (_tx, rx) = watch::channel(HashMap::<String, Decimal>::new());
    Arc::new(WsCache::new(rx))
}

pub fn spawn(chain: Chain, user_address: Option<Address>, http: reqwest::Client) -> Arc<WsCache> {
    let ws = match chain {
        Chain::Mainnet => hypercore::mainnet_ws(),
        Chain::Testnet => hypercore::testnet_ws(),
    };

    let (handle, stream) = ws.split();

    let (mids_tx, mids_rx) = watch::channel(HashMap::<String, Decimal>::new());
    let cache = Arc::new(WsCache::new(mids_rx));

    handle.subscribe(Subscription::AllMids { dex: None });
    if let Some(user) = user_address {
        handle.subscribe(Subscription::OrderUpdates { user });
        handle.subscribe(Subscription::UserFills { user });
    }

    let event_cache = Arc::clone(&cache);
    tokio::spawn(event_loop(stream, handle, event_cache, mids_tx));

    let poll_cache = Arc::clone(&cache);
    tokio::spawn(poll_meta_loop(http, chain, poll_cache));

    cache
}

async fn event_loop(
    mut stream: ConnectionStream,
    _handle: ConnectionHandle,
    cache: Arc<WsCache>,
    mids_tx: watch::Sender<HashMap<String, Decimal>>,
) {
    while let Some(event) = stream.next().await {
        match event {
            Event::Connected => tracing::info!("WebSocket connected"),
            Event::Disconnected => tracing::warn!("WebSocket disconnected (will reconnect)"),
            Event::Message(msg) => handle_message(msg, &cache, &mids_tx).await,
        }
    }
    tracing::error!("WebSocket event loop ended unexpectedly");
}

async fn handle_message(
    msg: Incoming,
    cache: &WsCache,
    mids_tx: &watch::Sender<HashMap<String, Decimal>>,
) {
    match msg {
        Incoming::AllMids { mids, .. } => {
            let _ = mids_tx.send(mids);
        }
        Incoming::OrderUpdates(_) | Incoming::UserFills { .. } => {
            cache.invalidate_user_data().await;
        }
        _ => {}
    }
}

async fn poll_meta_loop(http: reqwest::Client, chain: Chain, cache: Arc<WsCache>) {
    use std::time::Duration;
    use tokio::time;

    let interval = Duration::from_secs(5);
    let base_url = match chain {
        Chain::Mainnet => "https://api.hyperliquid.xyz",
        Chain::Testnet => "https://api.hyperliquid-testnet.xyz",
    };
    let url = format!("{base_url}/info");

    fetch_and_cache_meta(&http, &url, &cache).await;

    let mut ticker = time::interval(interval);
    ticker.tick().await;
    loop {
        ticker.tick().await;
        fetch_and_cache_meta(&http, &url, &cache).await;
    }
}

async fn fetch_and_cache_meta(http: &reqwest::Client, url: &str, cache: &WsCache) {
    match http
        .post(url)
        .json(&json!({"type": "metaAndAssetCtxs"}))
        .send()
        .await
    {
        Ok(resp) => match resp.json::<serde_json::Value>().await {
            Ok(data) => {
                *cache.meta_cache.write().await = Some(CachedValue::new(data));
                tracing::debug!("Polled metaAndAssetCtxs");
            }
            Err(e) => tracing::warn!(error = %e, "Failed to parse metaAndAssetCtxs response"),
        },
        Err(e) => tracing::warn!(error = %e, "Failed to fetch metaAndAssetCtxs"),
    }

    match http
        .post(url)
        .json(&json!({"type": "spotMetaAndAssetCtxs"}))
        .send()
        .await
    {
        Ok(resp) => match resp.json::<serde_json::Value>().await {
            Ok(data) => {
                *cache.spot_meta_cache.write().await = Some(CachedValue::new(data));
                tracing::debug!("Polled spotMetaAndAssetCtxs");
            }
            Err(e) => tracing::warn!(error = %e, "Failed to parse spotMetaAndAssetCtxs response"),
        },
        Err(e) => tracing::warn!(error = %e, "Failed to fetch spotMetaAndAssetCtxs"),
    }
}
