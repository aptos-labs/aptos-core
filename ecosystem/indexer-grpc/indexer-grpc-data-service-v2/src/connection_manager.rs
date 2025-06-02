// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    config::{LIVE_DATA_SERVICE, MAX_MESSAGE_SIZE},
    metrics::NUM_CONNECTED_STREAMS,
};
use aptos_indexer_grpc_utils::{system_time_to_proto, timestamp_now_proto};
use aptos_protos::indexer::v1::{
    grpc_manager_client::GrpcManagerClient, service_info::Info, ActiveStream, HeartbeatRequest,
    HistoricalDataServiceInfo, LiveDataServiceInfo, ServiceInfo, StreamInfo, StreamProgress,
    StreamProgressSampleProto,
};
use dashmap::DashMap;
use rand::prelude::*;
use std::{
    collections::VecDeque,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime},
};
use tonic::{codec::CompressionEncoding, transport::channel::Channel};
use tracing::{info, warn};

pub static MAX_HEARTBEAT_RETRIES: usize = 3;

static OLD_PROGRESS_SAMPLING_RATE: Duration = Duration::from_secs(60);
static RECENT_PROGRESS_SAMPLING_RATE: Duration = Duration::from_secs(1);
static MAX_RECENT_SAMPLES_TO_KEEP: usize = 60;
static MAX_OLD_SAMPLES_TO_KEEP: usize = 60;

#[derive(Default)]
struct StreamProgressSamples {
    old_samples: VecDeque<StreamProgressSample>,
    recent_samples: VecDeque<StreamProgressSample>,
}

impl StreamProgressSamples {
    fn new() -> Self {
        Default::default()
    }

    fn to_proto(&self) -> Vec<StreamProgressSampleProto> {
        self.old_samples
            .iter()
            .chain(self.recent_samples.iter())
            .map(|sample| StreamProgressSampleProto {
                timestamp: Some(system_time_to_proto(sample.timestamp)),
                version: sample.version,
                size_bytes: sample.size_bytes,
            })
            .collect()
    }

    fn maybe_add_sample(&mut self, version: u64, size_bytes: u64) {
        let now = SystemTime::now();
        let sample = StreamProgressSample {
            timestamp: now,
            version,
            size_bytes,
        };

        if Self::accept_sample(&self.recent_samples, &sample, RECENT_PROGRESS_SAMPLING_RATE) {
            self.recent_samples.push_back(sample);
            if self.recent_samples.len() > MAX_RECENT_SAMPLES_TO_KEEP {
                let sample = self.recent_samples.pop_front().unwrap();
                if Self::accept_sample(&self.old_samples, &sample, OLD_PROGRESS_SAMPLING_RATE) {
                    self.old_samples.push_back(sample);
                    if self.old_samples.len() > MAX_OLD_SAMPLES_TO_KEEP {
                        self.old_samples.pop_front();
                    }
                }
            }
        }
    }

    fn accept_sample(
        samples: &VecDeque<StreamProgressSample>,
        sample: &StreamProgressSample,
        sampling_rate: Duration,
    ) -> bool {
        if let Some(last_sample) = samples.back() {
            if let Ok(delta) = sample.timestamp.duration_since(last_sample.timestamp) {
                if delta >= sampling_rate {
                    return true;
                }
            }
        } else {
            return true;
        };

        false
    }
}

struct StreamProgressSample {
    timestamp: SystemTime,
    version: u64,
    size_bytes: u64,
}

pub(crate) struct ConnectionManager {
    chain_id: u64,
    grpc_manager_connections: DashMap<String, GrpcManagerClient<Channel>>,
    self_advertised_address: String,
    known_latest_version: AtomicU64,
    active_streams: DashMap<String, (ActiveStream, StreamProgressSamples)>,
    is_live_data_service: bool,
}

impl ConnectionManager {
    pub(crate) async fn new(
        chain_id: u64,
        grpc_manager_addresses: Vec<String>,
        self_advertised_address: String,
        is_live_data_service: bool,
    ) -> Self {
        let grpc_manager_connections = DashMap::new();
        grpc_manager_addresses.into_iter().for_each(|address| {
            grpc_manager_connections
                .insert(address.clone(), Self::create_client_from_address(&address));
        });
        let res = Self {
            chain_id,
            grpc_manager_connections,
            self_advertised_address,
            known_latest_version: AtomicU64::new(0),
            active_streams: DashMap::new(),
            is_live_data_service,
        };

        // Keep fetching latest version until it is available.
        while res.known_latest_version.load(Ordering::SeqCst) == 0 {
            for entry in res.grpc_manager_connections.iter() {
                let address = entry.key();
                if let Err(e) = res.heartbeat(address).await {
                    warn!("Error during heartbeat: {e}.");
                }
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }

        res
    }

    pub(crate) async fn start(&self) {
        loop {
            for entry in self.grpc_manager_connections.iter() {
                let address = entry.key();
                let mut retries = 0;
                loop {
                    let result = self.heartbeat(address).await;
                    if result.is_ok() {
                        break;
                    }
                    retries += 1;
                    if retries > MAX_HEARTBEAT_RETRIES {
                        warn!("Failed to send heartbeat to GrpcManager at {address}, last error: {result:?}.");
                        break;
                    }
                }
                continue;
            }

            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }

    pub(crate) fn chain_id(&self) -> u64 {
        self.chain_id
    }

    pub(crate) fn get_grpc_manager_client_for_request(&self) -> GrpcManagerClient<Channel> {
        let mut rng = thread_rng();
        self.grpc_manager_connections
            .iter()
            .choose(&mut rng)
            .map(|kv| kv.value().clone())
            .unwrap()
    }

    pub(crate) fn known_latest_version(&self) -> u64 {
        self.known_latest_version.load(Ordering::SeqCst)
    }

    pub(crate) fn update_known_latest_version(&self, version: u64) {
        self.known_latest_version
            .fetch_max(version, Ordering::SeqCst);
    }

    pub(crate) fn insert_active_stream(
        &self,
        id: &str,
        start_version: u64,
        end_version: Option<u64>,
    ) {
        self.active_streams.insert(
            id.to_owned(),
            (
                ActiveStream {
                    id: id.to_owned(),
                    start_time: Some(timestamp_now_proto()),
                    start_version,
                    end_version,
                    progress: None,
                },
                StreamProgressSamples::new(),
            ),
        );
        let label = if self.is_live_data_service {
            ["live_data_service"]
        } else {
            ["historical_data_service"]
        };
        NUM_CONNECTED_STREAMS.with_label_values(&label).inc();
    }

    pub(crate) fn remove_active_stream(&self, id: &String) {
        self.active_streams.remove(id);
        let label = if self.is_live_data_service {
            ["live_data_service"]
        } else {
            ["historical_data_service"]
        };
        NUM_CONNECTED_STREAMS.with_label_values(&label).dec();
    }

    pub(crate) fn update_stream_progress(&self, id: &str, version: u64, size_bytes: u64) {
        self.active_streams
            .get_mut(id)
            .unwrap()
            .1
            .maybe_add_sample(version, size_bytes);
    }

    pub(crate) fn get_active_streams(&self) -> Vec<ActiveStream> {
        self.active_streams
            .iter()
            .map(|entry| {
                let (active_stream, samples) = entry.value();
                let mut active_stream = active_stream.clone();
                active_stream.progress = Some(StreamProgress {
                    samples: samples.to_proto(),
                });
                active_stream
            })
            .collect()
    }

    async fn heartbeat(&self, address: &str) -> Result<(), tonic::Status> {
        info!("Sending heartbeat to GrpcManager {address}.");
        let timestamp = Some(timestamp_now_proto());
        let known_latest_version = Some(self.known_latest_version());
        let stream_info = Some(StreamInfo {
            active_streams: self.get_active_streams(),
        });

        let info = if self.is_live_data_service {
            let min_servable_version = match LIVE_DATA_SERVICE.get() {
                Some(svc) => Some(svc.get_min_servable_version().await),
                None => None,
            };
            Some(Info::LiveDataServiceInfo(LiveDataServiceInfo {
                chain_id: self.chain_id,
                timestamp,
                known_latest_version,
                stream_info,
                min_servable_version,
            }))
        } else {
            Some(Info::HistoricalDataServiceInfo(HistoricalDataServiceInfo {
                chain_id: self.chain_id,
                timestamp,
                known_latest_version,
                stream_info,
            }))
        };
        let service_info = ServiceInfo {
            address: Some(self.self_advertised_address.clone()),
            info,
        };
        let request = HeartbeatRequest {
            service_info: Some(service_info),
        };
        let response = self
            .grpc_manager_connections
            .get(address)
            // TODO(grao): Consider to not use unwrap here.
            .unwrap()
            .clone()
            .heartbeat(request)
            .await?
            .into_inner();
        if let Some(known_latest_version) = response.known_latest_version {
            info!("Received known_latest_version ({known_latest_version}) from GrpcManager {address}.");
            self.update_known_latest_version(known_latest_version);
        } else {
            warn!("HeartbeatResponse doesn't contain known_latest_version, GrpcManager address: {address}");
        }

        Ok(())
    }

    fn create_client_from_address(address: &str) -> GrpcManagerClient<Channel> {
        info!("Creating GrpcManagerClient for {address}.");
        let channel = Channel::from_shared(address.to_string())
            .expect("Bad address.")
            .connect_lazy();
        GrpcManagerClient::new(channel)
            .send_compressed(CompressionEncoding::Zstd)
            .accept_compressed(CompressionEncoding::Zstd)
            .max_decoding_message_size(MAX_MESSAGE_SIZE)
            .max_encoding_message_size(MAX_MESSAGE_SIZE)
    }
}
