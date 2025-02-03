// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::config::GrpcAddress;
use anyhow::{bail, Result};
use aptos_indexer_grpc_utils::timestamp_now_proto;
use aptos_protos::{
    indexer::v1::{
        data_service_client::DataServiceClient, grpc_manager_client::GrpcManagerClient,
        service_info::Info, FullnodeInfo, GrpcManagerInfo, HeartbeatRequest,
        HistoricalDataServiceInfo, LiveDataServiceInfo, PingDataServiceRequest, ServiceInfo,
    },
    internal::fullnode::v1::{fullnode_data_client::FullnodeDataClient, PingFullnodeRequest},
    util::timestamp::Timestamp,
};
use dashmap::DashMap;
use rand::{prelude::*, thread_rng};
use std::{
    collections::{HashMap, VecDeque},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tonic::transport::channel::Channel;
use tracing::trace;

// The maximum # of states for each service we keep.
const MAX_NUM_OF_STATES_TO_KEEP: usize = 100;

struct Peer {
    client: GrpcManagerClient<Channel>,
    recent_states: VecDeque<GrpcManagerInfo>,
}

impl Peer {
    fn new(address: GrpcAddress) -> Self {
        let channel = Channel::from_shared(address)
            .expect("Bad address.")
            .connect_lazy();
        let client = GrpcManagerClient::new(channel);
        Self {
            client,
            recent_states: VecDeque::new(),
        }
    }
}

struct Fullnode {
    client: FullnodeDataClient<Channel>,
    recent_states: VecDeque<FullnodeInfo>,
}

impl Fullnode {
    fn new(address: GrpcAddress) -> Self {
        let channel = Channel::from_shared(address)
            .expect("Bad address.")
            .connect_lazy();
        let client = FullnodeDataClient::new(channel);
        Self {
            client,
            recent_states: VecDeque::new(),
        }
    }
}

struct LiveDataService {
    client: DataServiceClient<Channel>,
    recent_states: VecDeque<LiveDataServiceInfo>,
}

impl LiveDataService {
    fn new(address: GrpcAddress) -> Self {
        let channel = Channel::from_shared(address)
            .expect("Bad address.")
            .connect_lazy();
        let client = DataServiceClient::new(channel);
        Self {
            client,
            recent_states: VecDeque::new(),
        }
    }
}

struct HistoricalDataService {
    client: DataServiceClient<Channel>,
    recent_states: VecDeque<HistoricalDataServiceInfo>,
}

impl HistoricalDataService {
    fn new(address: GrpcAddress) -> Self {
        let channel = Channel::from_shared(address)
            .expect("Bad address.")
            .connect_lazy();
        let client = DataServiceClient::new(channel);
        Self {
            client,
            recent_states: VecDeque::new(),
        }
    }
}

pub(crate) struct MetadataManager {
    chain_id: u64,
    self_advertised_address: GrpcAddress,
    grpc_managers: DashMap<GrpcAddress, Peer>,
    fullnodes: DashMap<GrpcAddress, Fullnode>,
    live_data_services: DashMap<GrpcAddress, LiveDataService>,
    historical_data_services: DashMap<GrpcAddress, HistoricalDataService>,
    known_latest_version: AtomicU64,
}

impl MetadataManager {
    pub(crate) fn new(
        chain_id: u64,
        self_advertised_address: GrpcAddress,
        grpc_manager_addresses: Vec<GrpcAddress>,
        fullnode_addresses: Vec<GrpcAddress>,
    ) -> Self {
        let grpc_managers = DashMap::new();
        for address in grpc_manager_addresses {
            grpc_managers.insert(address.clone(), Peer::new(address));
        }
        let fullnodes = DashMap::new();
        for address in fullnode_addresses {
            fullnodes.insert(address.clone(), Fullnode::new(address));
        }
        Self {
            chain_id,
            self_advertised_address,
            grpc_managers,
            fullnodes,
            live_data_services: DashMap::new(),
            historical_data_services: DashMap::new(),
            known_latest_version: AtomicU64::new(0),
        }
    }

    fn need_ping(latest_state_timestamp: Timestamp, threshold: Duration) -> bool {
        let latest_state_timestamp_since_epoch = Duration::new(
            latest_state_timestamp.seconds as u64,
            latest_state_timestamp.nanos as u32,
        );
        let now_since_epoch = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let staleness = now_since_epoch.saturating_sub(latest_state_timestamp_since_epoch);

        staleness >= threshold
    }

    pub(crate) async fn start(&self) -> Result<()> {
        loop {
            tokio_scoped::scope(|s| {
                for kv in &self.grpc_managers {
                    let grpc_manager = kv.value();
                    let client = grpc_manager.client.clone();
                    s.spawn(async move {
                        let _ = self.heartbeat(client).await;
                    });
                }

                for kv in &self.fullnodes {
                    let (address, fullnode) = kv.pair();
                    let need_ping = fullnode.recent_states.back().map_or(true, |s| {
                        Self::need_ping(s.timestamp.unwrap_or_default(), Duration::from_secs(1))
                    });
                    if need_ping {
                        let address = address.clone();
                        let client = fullnode.client.clone();
                        s.spawn(async move {
                            let _ = self.ping_fullnode(address, client).await;
                        });
                    }
                }

                for kv in &self.live_data_services {
                    let (address, live_data_service) = kv.pair();
                    let need_ping = live_data_service.recent_states.back().map_or(true, |s| {
                        Self::need_ping(s.timestamp.unwrap_or_default(), Duration::from_secs(5))
                    });
                    if need_ping {
                        let address = address.clone();
                        let client = live_data_service.client.clone();
                        s.spawn(async move {
                            let _ = self.ping_live_data_service(address, client).await;
                        });
                    }
                }

                for kv in &self.historical_data_services {
                    let (address, historical_data_service) = kv.pair();
                    let need_ping =
                        historical_data_service
                            .recent_states
                            .back()
                            .map_or(true, |s| {
                                Self::need_ping(
                                    s.timestamp.unwrap_or_default(),
                                    Duration::from_secs(5),
                                )
                            });
                    if need_ping {
                        let address = address.clone();
                        let client = historical_data_service.client.clone();
                        s.spawn(async move {
                            let _ = self.ping_historical_data_service(address, client).await;
                        });
                    }
                }
            });

            // TODO(grao): Remove unreachable services from the map.

            // TODO(grao): Double check if we should change this value, and/or we should separate
            // ping for different services to different loops.
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }

    pub(crate) fn handle_heartbeat(&self, address: GrpcAddress, info: Info) -> Result<()> {
        match info {
            Info::LiveDataServiceInfo(info) => self.handle_live_data_service_info(address, info),
            Info::HistoricalDataServiceInfo(info) => {
                self.handle_historical_data_service_info(address, info)
            },
            Info::FullnodeInfo(info) => self.handle_fullnode_info(address, info),
            Info::GrpcManagerInfo(info) => self.handle_grpc_manager_info(address, info),
        }
    }

    pub(crate) fn get_fullnode_for_request(&self) -> FullnodeDataClient<Channel> {
        let mut rng = thread_rng();
        // TODO(grao): Filter out bad FNs.
        self.fullnodes
            .iter()
            .choose(&mut rng)
            .map(|kv| kv.value().client.clone())
            .unwrap()
    }

    pub(crate) fn get_fullnodes_info(&self) -> HashMap<String, VecDeque<FullnodeInfo>> {
        self.fullnodes
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().recent_states.clone()))
            .collect()
    }

    pub(crate) fn get_live_data_services_info(
        &self,
    ) -> HashMap<GrpcAddress, VecDeque<LiveDataServiceInfo>> {
        self.live_data_services
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().recent_states.clone()))
            .collect()
    }

    pub(crate) fn get_historical_data_services_info(
        &self,
    ) -> HashMap<GrpcAddress, VecDeque<HistoricalDataServiceInfo>> {
        self.historical_data_services
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().recent_states.clone()))
            .collect()
    }

    pub(crate) fn get_known_latest_version(&self) -> u64 {
        self.known_latest_version.load(Ordering::SeqCst)
    }

    fn update_known_latest_version(&self, version: u64) {
        self.known_latest_version
            .fetch_max(version, Ordering::SeqCst);
    }

    async fn heartbeat(&self, mut client: GrpcManagerClient<Channel>) -> Result<()> {
        let grpc_manager_info = GrpcManagerInfo {
            chain_id: self.chain_id,
            timestamp: Some(timestamp_now_proto()),
            known_latest_version: Some(self.get_known_latest_version()),
            // TODO(grao): Store and broadcast master address if known.
            master_address: None,
        };
        let service_info = ServiceInfo {
            address: Some(self.self_advertised_address.clone()),
            info: Some(Info::GrpcManagerInfo(grpc_manager_info)),
        };
        let request = HeartbeatRequest {
            service_info: Some(service_info),
        };
        let _ = client.heartbeat(request).await?;

        Ok(())
    }

    async fn ping_fullnode(
        &self,
        address: GrpcAddress,
        mut client: FullnodeDataClient<Channel>,
    ) -> Result<()> {
        trace!("Pinging fullnode {address}.");
        let request = PingFullnodeRequest {};
        let response = client.ping(request).await?;
        if let Some(info) = response.into_inner().info {
            self.handle_fullnode_info(address, info)
        } else {
            bail!("Bad response.")
        }
    }

    async fn ping_live_data_service(
        &self,
        address: GrpcAddress,
        mut client: DataServiceClient<Channel>,
    ) -> Result<()> {
        let request = PingDataServiceRequest {
            known_latest_version: Some(self.get_known_latest_version()),
            ping_live_data_service: true,
        };
        let response = client.ping(request).await?;
        if let Some(info) = response.into_inner().info {
            match info {
                aptos_protos::indexer::v1::ping_data_service_response::Info::LiveDataServiceInfo(info) => {
                    self.handle_live_data_service_info(address, info)
                },
                _ => bail!("Bad response."),
            }
        } else {
            bail!("Bad response.")
        }
    }

    async fn ping_historical_data_service(
        &self,
        address: GrpcAddress,
        mut client: DataServiceClient<Channel>,
    ) -> Result<()> {
        let request = PingDataServiceRequest {
            known_latest_version: Some(self.get_known_latest_version()),
            ping_live_data_service: false,
        };
        let response = client.ping(request).await?;
        if let Some(info) = response.into_inner().info {
            match info {
                aptos_protos::indexer::v1::ping_data_service_response::Info::HistoricalDataServiceInfo(info) => {
                    self.handle_historical_data_service_info(address, info)
                },
                _ => bail!("Bad response."),
            }
        } else {
            bail!("Bad response.")
        }
    }

    fn handle_live_data_service_info(
        &self,
        address: GrpcAddress,
        info: LiveDataServiceInfo,
    ) -> Result<()> {
        let mut entry = self
            .live_data_services
            .entry(address.clone())
            .or_insert(LiveDataService::new(address));
        entry.value_mut().recent_states.push_back(info);
        if entry.value().recent_states.len() > MAX_NUM_OF_STATES_TO_KEEP {
            entry.value_mut().recent_states.pop_front();
        }

        Ok(())
    }

    fn handle_historical_data_service_info(
        &self,
        address: GrpcAddress,
        info: HistoricalDataServiceInfo,
    ) -> Result<()> {
        let mut entry = self
            .historical_data_services
            .entry(address.clone())
            .or_insert(HistoricalDataService::new(address));
        entry.value_mut().recent_states.push_back(info);
        if entry.value().recent_states.len() > MAX_NUM_OF_STATES_TO_KEEP {
            entry.value_mut().recent_states.pop_front();
        }

        Ok(())
    }

    fn handle_fullnode_info(&self, address: GrpcAddress, info: FullnodeInfo) -> Result<()> {
        let mut entry = self
            .fullnodes
            .entry(address.clone())
            .or_insert(Fullnode::new(address.clone()));
        entry.value_mut().recent_states.push_back(info);
        if let Some(known_latest_version) = info.known_latest_version {
            trace!(
                "Received known_latest_version ({known_latest_version}) from fullnode {address}."
            );
            self.update_known_latest_version(known_latest_version);
        }
        if entry.value().recent_states.len() > MAX_NUM_OF_STATES_TO_KEEP {
            entry.value_mut().recent_states.pop_front();
        }

        Ok(())
    }

    fn handle_grpc_manager_info(&self, address: GrpcAddress, info: GrpcManagerInfo) -> Result<()> {
        let mut entry = self
            .grpc_managers
            .entry(address.clone())
            .or_insert(Peer::new(address));
        entry.value_mut().recent_states.push_back(info);
        if entry.value().recent_states.len() > MAX_NUM_OF_STATES_TO_KEEP {
            entry.value_mut().recent_states.pop_front();
        }

        Ok(())
    }
}
