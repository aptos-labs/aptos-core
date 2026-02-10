// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! V2 API context. Wraps the existing v1 Context to share DB/mempool/caches
//! while adding v2-specific capabilities and decoupling from Poem error traits.

use super::{
    cursor::Cursor,
    error::{ErrorCode, V2Error},
};
#[cfg(any(feature = "api-v2-websocket", feature = "api-v2-sse"))]
use super::websocket::{broadcaster, types::WsEvent};
use crate::context::Context;
use aptos_api_types::LedgerInfo;
use aptos_config::config::ApiV2Config;
use aptos_storage_interface::state_store::state_view::db_state_view::DbStateView;
use aptos_types::{
    account_address::AccountAddress, contract_event::EventWithVersion, event::EventKey,
    transaction::Version,
};
use move_core_types::language_storage::{ModuleId, StructTag};
#[cfg(any(feature = "api-v2-websocket", feature = "api-v2-sse"))]
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use std::time::Instant;
#[cfg(any(feature = "api-v2-websocket", feature = "api-v2-sse"))]
use tokio::sync::broadcast;
use tokio::sync::{watch, RwLock};

/// Default TTL for cached ledger info (in milliseconds).
/// Ledger info changes every block (~250ms on mainnet), so 50ms TTL
/// is a good balance between freshness and avoiding redundant DB reads.
const LEDGER_INFO_CACHE_TTL_MS: u64 = 50;

/// Cached ledger info with a timestamp for TTL expiration.
struct CachedLedgerInfo {
    info: LedgerInfo,
    fetched_at: Instant,
}

/// V2 API context. Wraps the existing v1 Context and adds v2-specific state.
///
/// This is the Axum `State` type for all v2 handlers.
/// It is `Clone` (via `Arc` wrapping of inner Context).
#[derive(Clone)]
pub struct V2Context {
    /// The shared v1 context.
    inner: Arc<Context>,
    /// v2-specific configuration.
    pub v2_config: Arc<V2Config>,
    /// WebSocket/SSE broadcast channel sender. All connected WS/SSE clients subscribe
    /// to this channel to receive block/event notifications.
    #[cfg(any(feature = "api-v2-websocket", feature = "api-v2-sse"))]
    ws_broadcaster: broadcast::Sender<WsEvent>,
    /// Count of active WebSocket connections (for connection limiting).
    #[cfg(any(feature = "api-v2-websocket", feature = "api-v2-sse"))]
    ws_active_connections: Arc<AtomicUsize>,
    /// TTL-cached ledger info. Under high QPS, many concurrent requests
    /// within the same ~50ms window share a single DB read.
    ledger_info_cache: Arc<RwLock<Option<CachedLedgerInfo>>>,
    /// Shutdown signal sender. Sending `true` triggers graceful shutdown
    /// of the server and all background tasks.
    shutdown_tx: Arc<watch::Sender<bool>>,
    /// Shutdown signal receiver. Clone this for each consumer that needs
    /// to be notified of shutdown.
    shutdown_rx: watch::Receiver<bool>,
}

/// v2-specific configuration parsed at startup.
#[derive(Debug, Clone)]
pub struct V2Config {
    pub enabled: bool,
    pub websocket_enabled: bool,
    pub sse_enabled: bool,
    pub websocket_max_connections: usize,
    pub websocket_max_subscriptions_per_conn: usize,
    pub http2_enabled: bool,
    pub json_rpc_batch_max_size: usize,
    pub content_length_limit: u64,
    pub max_gas_view_function: u64,
    pub max_account_resources_page_size: u16,
    pub max_account_modules_page_size: u16,
    pub max_transactions_page_size: u16,
    pub max_events_page_size: u16,
    pub wait_by_hash_timeout_ms: u64,
    pub wait_by_hash_poll_interval_ms: u64,
    pub request_timeout_ms: u64,
    pub graceful_shutdown_timeout_ms: u64,
}

impl V2Config {
    pub fn from_configs(v2: &ApiV2Config, api: &aptos_config::config::ApiConfig) -> Self {
        V2Config {
            enabled: v2.enabled,
            websocket_enabled: v2.websocket_enabled,
            sse_enabled: v2.sse_enabled,
            websocket_max_connections: v2.websocket_max_connections,
            websocket_max_subscriptions_per_conn: v2.websocket_max_subscriptions_per_conn,
            http2_enabled: v2.http2_enabled,
            json_rpc_batch_max_size: v2.json_rpc_batch_max_size,
            content_length_limit: v2
                .content_length_limit
                .unwrap_or_else(|| api.content_length_limit()),
            max_gas_view_function: api.max_gas_view_function,
            max_account_resources_page_size: api.max_account_resources_page_size,
            max_account_modules_page_size: api.max_account_modules_page_size,
            max_transactions_page_size: api.max_transactions_page_size,
            max_events_page_size: api.max_events_page_size,
            wait_by_hash_timeout_ms: api.wait_by_hash_timeout_ms,
            wait_by_hash_poll_interval_ms: api.wait_by_hash_poll_interval_ms,
            request_timeout_ms: v2.request_timeout_ms,
            graceful_shutdown_timeout_ms: v2.graceful_shutdown_timeout_ms,
        }
    }
}

impl V2Context {
    pub fn new(inner: Context, v2_config: V2Config) -> Self {
        #[cfg(any(feature = "api-v2-websocket", feature = "api-v2-sse"))]
        let (ws_broadcaster, _) = broadcaster::create_broadcast_channel();
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        Self {
            inner: Arc::new(inner),
            v2_config: Arc::new(v2_config),
            #[cfg(any(feature = "api-v2-websocket", feature = "api-v2-sse"))]
            ws_broadcaster,
            #[cfg(any(feature = "api-v2-websocket", feature = "api-v2-sse"))]
            ws_active_connections: Arc::new(AtomicUsize::new(0)),
            ledger_info_cache: Arc::new(RwLock::new(None)),
            shutdown_tx: Arc::new(shutdown_tx),
            shutdown_rx,
        }
    }

    /// Access the underlying v1 context.
    pub fn inner(&self) -> &Context {
        &self.inner
    }

    /// Get a new broadcast receiver for WebSocket/SSE events.
    #[cfg(any(feature = "api-v2-websocket", feature = "api-v2-sse"))]
    pub fn ws_subscribe(&self) -> broadcast::Receiver<WsEvent> {
        self.ws_broadcaster.subscribe()
    }

    /// Get a clone of the broadcast sender (for the block poller).
    #[cfg(any(feature = "api-v2-websocket", feature = "api-v2-sse"))]
    pub fn ws_broadcaster(&self) -> broadcast::Sender<WsEvent> {
        self.ws_broadcaster.clone()
    }

    /// Get the active WebSocket connection counter.
    #[cfg(any(feature = "api-v2-websocket", feature = "api-v2-sse"))]
    pub fn ws_active_connections(&self) -> &AtomicUsize {
        &self.ws_active_connections
    }

    /// Trigger graceful shutdown of the v2 API server and all background tasks.
    pub fn trigger_shutdown(&self) {
        let _ = self.shutdown_tx.send(true);
    }

    /// Clone the shutdown receiver for passing to background tasks or servers.
    pub fn shutdown_receiver(&self) -> watch::Receiver<bool> {
        self.shutdown_rx.clone()
    }

    /// Returns a future that resolves when shutdown is requested.
    pub async fn shutdown_signal(&self) {
        let mut rx = self.shutdown_rx.clone();
        let _ = rx.wait_for(|&v| v).await;
    }

    // --- Core accessors (v2 error conversion) ---

    /// Get the latest ledger info with TTL caching.
    ///
    /// Under high QPS, many concurrent requests within the same ~50ms window
    /// will share a single DB read instead of each hitting the DB independently.
    /// The cache is invalidated after `LEDGER_INFO_CACHE_TTL_MS` milliseconds.
    pub fn ledger_info(&self) -> Result<LedgerInfo, V2Error> {
        // Fast path: try read lock first.
        // We can't use async RwLock in a synchronous context, so we use try_read.
        if let Ok(guard) = self.ledger_info_cache.try_read() {
            if let Some(ref cached) = *guard {
                if cached.fetched_at.elapsed().as_millis() < LEDGER_INFO_CACHE_TTL_MS as u128 {
                    super::metrics::LEDGER_INFO_CACHE
                        .with_label_values(&["hit"])
                        .inc();
                    return Ok(cached.info.clone());
                }
            }
        }

        super::metrics::LEDGER_INFO_CACHE
            .with_label_values(&["miss"])
            .inc();

        // Cache miss or expired: fetch from DB and update cache.
        let info = self
            .inner
            .get_latest_ledger_info_wrapped()
            .map_err(V2Error::internal)?;

        // Best-effort cache update (don't block if another writer is updating).
        if let Ok(mut guard) = self.ledger_info_cache.try_write() {
            *guard = Some(CachedLedgerInfo {
                info: info.clone(),
                fetched_at: Instant::now(),
            });
        }

        Ok(info)
    }

    /// Get the latest ledger info, bypassing the cache.
    /// Use this when you absolutely need the freshest data (e.g., tx wait loops).
    pub fn ledger_info_uncached(&self) -> Result<LedgerInfo, V2Error> {
        self.inner
            .get_latest_ledger_info_wrapped()
            .map_err(V2Error::internal)
    }

    /// Get a state view at a specific version, verifying it's valid.
    pub fn state_view_at(
        &self,
        requested_version: Option<u64>,
    ) -> Result<(LedgerInfo, Version, DbStateView), V2Error> {
        let ledger_info = self.ledger_info()?;
        let version = requested_version.unwrap_or_else(|| ledger_info.version());

        if version > ledger_info.version() {
            return Err(V2Error::not_found(
                ErrorCode::VersionNotFound,
                format!(
                    "Version {} is in the future (latest: {})",
                    version,
                    ledger_info.version()
                ),
            ));
        }
        if version < ledger_info.oldest_version() {
            return Err(V2Error::gone(
                ErrorCode::VersionPruned,
                format!(
                    "Version {} has been pruned (oldest: {})",
                    version,
                    ledger_info.oldest_version()
                ),
            ));
        }

        let state_view = self
            .inner
            .state_view_at_version(version)
            .map_err(V2Error::internal)?;

        Ok((ledger_info, version, state_view))
    }

    // --- Pagination helpers ---

    /// Get paginated resources for an account.
    pub fn get_resources_paginated(
        &self,
        address: AccountAddress,
        cursor: Option<&Cursor>,
        version: u64,
    ) -> Result<(Vec<(StructTag, Vec<u8>)>, Option<Cursor>), V2Error> {
        let prev_key = cursor.map(|c| c.as_state_key()).transpose()?;
        let page_size = self.v2_config.max_account_resources_page_size as u64;

        let (resources, next_key) = self
            .inner
            .get_resources_by_pagination(address, prev_key.as_ref(), version, page_size)
            .map_err(V2Error::internal)?;

        let next_cursor = next_key.map(|k| Cursor::from_state_key(&k));
        Ok((resources, next_cursor))
    }

    /// Get paginated modules for an account.
    pub fn get_modules_paginated(
        &self,
        address: AccountAddress,
        cursor: Option<&Cursor>,
        version: u64,
    ) -> Result<(Vec<(ModuleId, Vec<u8>)>, Option<Cursor>), V2Error> {
        let prev_key = cursor.map(|c| c.as_state_key()).transpose()?;
        let page_size = self.v2_config.max_account_modules_page_size as u64;

        let (modules, next_key) = self
            .inner
            .get_modules_by_pagination(address, prev_key.as_ref(), version, page_size)
            .map_err(V2Error::internal)?;

        let next_cursor = next_key.map(|k| Cursor::from_state_key(&k));
        Ok((modules, next_cursor))
    }

    /// Get paginated transactions by version.
    pub fn get_transactions_paginated(
        &self,
        cursor: Option<&Cursor>,
        ledger_version: u64,
    ) -> Result<(Vec<aptos_api_types::TransactionOnChainData>, Option<Cursor>), V2Error> {
        let page_size = self.v2_config.max_transactions_page_size;
        let start_version = match cursor {
            Some(c) => c.as_version()? + 1,
            None => 0,
        };

        if start_version > ledger_version {
            return Ok((vec![], None));
        }

        let txns = self
            .inner
            .get_transactions(start_version, page_size, ledger_version)
            .map_err(V2Error::internal)?;

        let next_cursor = if txns.len() as u16 == page_size {
            txns.last().map(|t| Cursor::from_version(t.version))
        } else {
            None
        };

        Ok((txns, next_cursor))
    }

    /// Get paginated events by event key.
    pub fn get_events_paginated(
        &self,
        event_key: &EventKey,
        cursor: Option<&Cursor>,
        ledger_version: u64,
    ) -> Result<(Vec<EventWithVersion>, Option<Cursor>), V2Error> {
        let page_size = self.v2_config.max_events_page_size;
        let start_seq = match cursor {
            Some(c) => Some(c.as_sequence_number()? + 1),
            None => Some(0),
        };

        let events = self
            .inner
            .get_events(event_key, start_seq, page_size, ledger_version)
            .map_err(V2Error::internal)?;

        // For cursor: use the start_seq + count to determine next cursor
        let next_cursor = if events.len() as u16 == page_size {
            let last_seq = start_seq.unwrap_or(0) + events.len() as u64 - 1;
            Some(Cursor::from_sequence_number(last_seq))
        } else {
            None
        };

        Ok((events, next_cursor))
    }

    // --- Block access ---

    /// Get a block by height, returning V2Error on failure.
    pub fn get_block_by_height(
        &self,
        height: u64,
        with_transactions: bool,
    ) -> Result<(aptos_api_types::BcsBlock, LedgerInfo), V2Error> {
        let ledger_info = self.ledger_info()?;

        let oldest_block_height: u64 = ledger_info.oldest_block_height.into();
        if height < oldest_block_height {
            return Err(V2Error::gone(
                ErrorCode::BlockPruned,
                format!("Block {} has been pruned", height),
            ));
        }

        let block_height: u64 = ledger_info.block_height.into();
        if height > block_height {
            return Err(V2Error::not_found(
                ErrorCode::BlockNotFound,
                format!(
                    "Block height {} not found (latest: {})",
                    height, block_height
                ),
            ));
        }

        // Use BasicErrorWith404 which implements StdApiError (includes Gone + NotFound).
        let block = self
            .inner
            .get_block_by_height::<crate::response::BasicErrorWith404>(
                height,
                &ledger_info,
                with_transactions,
            )
            .map_err(|e| V2Error::internal(anyhow::anyhow!("{}", e)))?;

        Ok((block, ledger_info))
    }
}

/// Spawn a blocking task for synchronous DB operations.
pub async fn spawn_blocking<F, T>(f: F) -> Result<T, V2Error>
where
    F: FnOnce() -> Result<T, V2Error> + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| V2Error::internal(anyhow::anyhow!("Blocking task failed: {}", e)))?
}
