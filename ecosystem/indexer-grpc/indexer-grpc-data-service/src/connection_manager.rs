// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::config::MAX_MESSAGE_SIZE;
use aptos_indexer_grpc_utils::timestamp_now_proto;
use aptos_protos::indexer::v1::{
    grpc_manager_client::GrpcManagerClient, service_info::ServiceType, ActiveStream,
    DataServiceInfo, HeartbeatRequest, ServiceInfo, StreamInfo,
};
use dashmap::DashMap;
use rand::prelude::*;
use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};
use tonic::{codec::CompressionEncoding, transport::channel::Channel};
use tracing::{info, warn};

pub static MAX_HEARTBEAT_RETRIES: usize = 3;

pub(crate) struct ConnectionManager {
    grpc_manager_connections: DashMap<String, GrpcManagerClient<Channel>>,
    self_advertised_address: String,
    known_latest_version: AtomicU64,
    active_streams: DashMap<String, ActiveStream>,
    is_live_data_service: bool,
}

impl ConnectionManager {
    pub(crate) async fn new(
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
                    }
                }
                continue;
            }

            tokio::time::sleep(Duration::from_secs(1)).await;
        }
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
        id: &String,
        start_version: u64,
        end_version: Option<u64>,
    ) {
        self.active_streams.insert(
            id.clone(),
            ActiveStream {
                id: Some(id.clone()),
                current_version: Some(start_version),
                end_version,
            },
        );
    }

    pub(crate) fn remove_active_stream(&self, id: &String) {
        self.active_streams.remove(id);
    }

    pub(crate) fn update_stream_progress(&self, id: &String, version: u64) {
        self.active_streams.get_mut(id).unwrap().current_version = Some(version);
    }

    pub(crate) fn get_active_streams(&self) -> Vec<ActiveStream> {
        self.active_streams
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    async fn heartbeat(&self, address: &str) -> Result<(), tonic::Status> {
        info!("Sending heartbeat to GrpcManager {address}.");
        let stream_info = StreamInfo {
            active_streams: self.get_active_streams(),
        };
        let data_service_info = DataServiceInfo {
            timestamp: Some(timestamp_now_proto()),
            known_latest_version: Some(self.known_latest_version()),
            stream_info: Some(stream_info),
        };
        let service_info = ServiceInfo {
            address: Some(self.self_advertised_address.clone()),
            service_type: if self.is_live_data_service {
                Some(ServiceType::LiveDataServiceInfo(data_service_info))
            } else {
                Some(ServiceType::HistoricalDataServiceInfo(data_service_info))
            },
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
