use std::collections::HashMap;
use std::time::{Duration, Instant};

use hypersdk::hypercore::ClearinghouseState;
use hypersdk::hypercore::types::BasicOrder;
use rust_decimal::Decimal;
use serde_json::Value;
use tokio::sync::RwLock;
use tokio::sync::watch;

pub struct CachedValue<T> {
    pub value: T,
    pub inserted_at: Instant,
}

impl<T> CachedValue<T> {
    pub fn new(value: T) -> Self {
        Self {
            value,
            inserted_at: Instant::now(),
        }
    }

    pub fn is_fresh(&self, ttl: Duration) -> bool {
        self.inserted_at.elapsed() < ttl
    }
}

pub struct WsCache {
    pub all_mids: watch::Receiver<HashMap<String, Decimal>>,
    pub meta_cache: RwLock<Option<CachedValue<Value>>>,
    pub spot_meta_cache: RwLock<Option<CachedValue<Value>>>,
    pub clearinghouse_cache: RwLock<Option<CachedValue<ClearinghouseState>>>,
    pub open_orders_cache: RwLock<Option<CachedValue<Vec<BasicOrder>>>>,
}

impl WsCache {
    pub fn new(mids_rx: watch::Receiver<HashMap<String, Decimal>>) -> Self {
        Self {
            all_mids: mids_rx,
            meta_cache: RwLock::new(None),
            spot_meta_cache: RwLock::new(None),
            clearinghouse_cache: RwLock::new(None),
            open_orders_cache: RwLock::new(None),
        }
    }

    pub async fn invalidate_user_data(&self) {
        *self.clearinghouse_cache.write().await = None;
        *self.open_orders_cache.write().await = None;
    }
}
