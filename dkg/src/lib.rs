// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

mod agg_trx_producer;
pub mod chunky;
mod counters;
mod dkg_manager;
pub mod epoch_manager;
pub mod network;
pub mod network_interface;
pub mod transcript_aggregation;
pub mod types;

use crate::{
    epoch_manager::EpochManager, network::NetworkTask, network_interface::DKGNetworkClient,
};
use aptos_config::config::{ReliableBroadcastConfig, SafetyRulesConfig};
use aptos_event_notifications::{
    DbBackedOnChainConfig, EventNotificationListener, ReconfigNotificationListener,
};
use aptos_logger::{info, warn};
use aptos_metrics_core::{IntGauge, IntGaugeVec};
use aptos_network::application::interface::{NetworkClient, NetworkServiceEvents};
use aptos_types::{
    chain_id::ChainId,
    dkg::chunky_dkg::{
        initialize_digest_key, initialize_public_parameters, set_digest_key_path,
        set_public_parameters_path, DigestKeySource, PublicParametersSource, DIGEST_KEY,
        PUBLIC_PARAMETERS,
    },
};
use aptos_validator_transaction_pool::VTxnPoolState;
use move_core_types::account_address::AccountAddress;
use std::{
    io,
    path::{Path, PathBuf},
    time::Instant,
};
use tokio::runtime::Runtime;
pub use types::DKGMessage;

pub fn start_dkg_runtime(
    my_addr: AccountAddress,
    safety_rules_config: &SafetyRulesConfig,
    network_client: NetworkClient<DKGMessage>,
    network_service_events: NetworkServiceEvents<DKGMessage>,
    reconfig_events: ReconfigNotificationListener<DbBackedOnChainConfig>,
    dkg_start_events: EventNotificationListener,
    vtxn_pool: VTxnPoolState,
    rb_config: ReliableBroadcastConfig,
    randomness_override_seq_num: u64,
    chunky_dkg_override_seq_num: u64,
) -> Runtime {
    let runtime = aptos_runtimes::spawn_named_runtime("dkg".into(), Some(4));
    let (self_sender, self_receiver) = aptos_channels::new(1_024, &counters::PENDING_SELF_MESSAGES);
    let dkg_network_client = DKGNetworkClient::new(network_client);

    let dkg_epoch_manager = EpochManager::new(
        safety_rules_config,
        my_addr,
        reconfig_events,
        dkg_start_events,
        self_sender,
        dkg_network_client,
        vtxn_pool,
        rb_config,
        randomness_override_seq_num,
        chunky_dkg_override_seq_num,
    );
    let (network_task, network_receiver) = NetworkTask::new(network_service_events, self_receiver);
    runtime.spawn(network_task.start());
    runtime.spawn(dkg_epoch_manager.start(network_receiver));
    runtime
}

/// Begin deserializing the randomness trusted setup (`DigestKey` / `PublicParameters`)
/// from disk on background threads.
///
/// These are otherwise loaded lazily on first use during consensus epoch setup, where
/// the (tens-of-seconds) BCS deserialize blocks the node from rejoining. Kicking the
/// load off here -- before the database is opened -- lets it run concurrently with the
/// RocksDB open instead of serially after it. The lazy globals are thread-safe, so the
/// later first access simply returns the value these threads have already populated.
///
/// Only the file-backed load is started here (it needs no `chain_id`); test-chain
/// fallbacks with no blob file are resolved later by the `*_with_counters` functions.
/// Must be called before those functions (it owns the one-time path setup).
pub fn prefetch_trusted_setup(
    digest_key_blob_path: Option<&PathBuf>,
    public_parameters_blob_path: Option<&PathBuf>,
) {
    // PublicParameters are needed by every role (fullnodes use them during state sync).
    if let Some(path) = public_parameters_blob_path {
        set_public_parameters_path(path.clone());
        let path = path.clone();
        std::thread::Builder::new()
            .name("pub-params-load".into())
            .spawn(move || {
                let start = Instant::now();
                let _ = &*PUBLIC_PARAMETERS;
                counters::PUBLIC_PARAMS_LOAD_DURATION_SECONDS
                    .observe(start.elapsed().as_secs_f64());
                publish_blake3(&path, &counters::PUBLIC_PARAMS_BLAKE3, "PublicParameters");
            })
            .expect("Failed to spawn pub-params-load thread");
    }
    if let Some(path) = digest_key_blob_path {
        set_digest_key_path(path.clone());
        let path = path.clone();
        std::thread::Builder::new()
            .name("digest-key-load".into())
            .spawn(move || {
                let start = Instant::now();
                let _ = &*DIGEST_KEY;
                counters::DIGEST_KEY_LOAD_DURATION_SECONDS.observe(start.elapsed().as_secs_f64());
                publish_blake3(&path, &counters::DIGEST_KEY_BLAKE3, "DigestKey");
            })
            .expect("Failed to spawn digest-key-load thread");
    }
}

/// Emit Prometheus counters describing the resolved `DigestKey` source. The file load
/// itself is kicked off earlier by [`prefetch_trusted_setup`].
pub fn initialize_digest_key_with_counters(chain_id: ChainId, is_validator: bool) {
    match initialize_digest_key(chain_id, is_validator) {
        DigestKeySource::WillLoadFromFile { file_size, .. } => {
            counters::DIGEST_KEY_FILE_SIZE_BYTES.set(file_size as i64);
            counters::DIGEST_KEY_SOURCE
                .with_label_values(&["file"])
                .set(1);
        },
        DigestKeySource::TestKeyFallback => {
            counters::DIGEST_KEY_SOURCE
                .with_label_values(&["test_fallback"])
                .set(1);
        },
        DigestKeySource::NotAvailable => {
            counters::DIGEST_KEY_SOURCE
                .with_label_values(&["none"])
                .set(1);
        },
    }
}

/// Emit Prometheus counters describing the resolved `PublicParameters` source. The file
/// load itself is kicked off earlier by [`prefetch_trusted_setup`].
pub fn initialize_public_parameters_with_counters(chain_id: ChainId) {
    match initialize_public_parameters(chain_id) {
        PublicParametersSource::WillLoadFromFile { file_size, .. } => {
            counters::PUBLIC_PARAMS_FILE_SIZE_BYTES.set(file_size as i64);
            counters::PUBLIC_PARAMS_SOURCE
                .with_label_values(&["file"])
                .set(1);
        },
        PublicParametersSource::TestKeyFallback => {
            counters::PUBLIC_PARAMS_SOURCE
                .with_label_values(&["test_fallback"])
                .set(1);
        },
        PublicParametersSource::NotAvailable => {
            counters::PUBLIC_PARAMS_SOURCE
                .with_label_values(&["none"])
                .set(1);
        },
    }
}

fn hash_file_blake3(path: &Path) -> io::Result<blake3::Hash> {
    let mut hasher = blake3::Hasher::new();
    hasher.update_mmap(path)?;
    Ok(hasher.finalize())
}

fn publish_blake3(path: &Path, gauge: &IntGaugeVec, kind: &str) {
    match hash_file_blake3(path) {
        Ok(hash) => {
            let hex = hash.to_hex();
            gauge.with_label_values(&[hex.as_str()]).set(1);
            info!(
                "{} blake3 hash for {}: {}",
                kind,
                path.display(),
                hex.as_str()
            );
        },
        Err(e) => {
            warn!(
                "Failed to compute {} blake3 hash for {}: {}",
                kind,
                path.display(),
                e
            );
        },
    }
}

pub struct IntGaugeGuard {
    gauge: IntGauge,
}

impl IntGaugeGuard {
    fn new(gauge: IntGauge) -> Self {
        gauge.inc();
        Self { gauge }
    }
}

impl Drop for IntGaugeGuard {
    fn drop(&mut self) {
        self.gauge.dec();
    }
}

/// Helper function to record metrics for external calls.
/// Include call counts, time, and whether it's inside or not (1 or 0).
/// It assumes a OpMetrics defined as OP_COUNTERS in crate::counters;
#[macro_export]
macro_rules! monitor {
    ($name:literal, $fn:expr) => {{
        use $crate::{counters::OP_COUNTERS, IntGaugeGuard};
        let _timer = OP_COUNTERS.timer($name);
        let _guard = IntGaugeGuard::new(OP_COUNTERS.gauge(concat!($name, "_running")));
        $fn
    }};
}
