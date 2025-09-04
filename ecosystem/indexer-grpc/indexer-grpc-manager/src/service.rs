// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{data_manager::DataManager, metadata_manager::MetadataManager, metrics::COUNTER};
use velor_protos::indexer::v1::{
    grpc_manager_server::GrpcManager, service_info::Info, GetDataServiceForRequestRequest,
    GetDataServiceForRequestResponse, GetTransactionsRequest, HeartbeatRequest, HeartbeatResponse,
    TransactionsResponse,
};
use rand::{thread_rng, Rng};
use std::sync::Arc;
use tonic::{Request, Response, Status};

const MAX_SIZE_BYTES_FROM_CACHE: usize = 20 * (1 << 20);

pub struct GrpcManagerService {
    chain_id: u64,
    metadata_manager: Arc<MetadataManager>,
    data_manager: Arc<DataManager>,
}

impl GrpcManagerService {
    pub(crate) fn new(
        chain_id: u64,
        metadata_manager: Arc<MetadataManager>,
        data_manager: Arc<DataManager>,
    ) -> Self {
        Self {
            chain_id,
            metadata_manager,
            data_manager,
        }
    }

    async fn handle_heartbeat(
        &self,
        address: String,
        info: Info,
    ) -> anyhow::Result<Response<HeartbeatResponse>> {
        self.metadata_manager.handle_heartbeat(address, info)?;

        Ok(Response::new(HeartbeatResponse {
            known_latest_version: Some(self.metadata_manager.get_known_latest_version()),
        }))
    }

    fn pick_data_service_from_candidate(candidates: Vec<(String, usize)>) -> Option<String> {
        if candidates.is_empty() {
            return None;
        }

        // TODO(grao): This is a magic number, consider a different algorithm here.
        let capacity = std::cmp::max(candidates.iter().map(|c| c.1).max().unwrap() + 2, 20);

        let total_capacity: usize = candidates.iter().map(|c| capacity - c.1).sum();

        let mut rng = thread_rng();
        let pick = rng.gen_range(0, total_capacity);

        let mut cumulative_weight = 0;
        for candidate in candidates {
            cumulative_weight += capacity - candidate.1;
            if pick < cumulative_weight {
                return Some(candidate.0);
            }
        }

        unreachable!();
    }

    fn pick_live_data_service(&self, starting_version: u64) -> Option<String> {
        let mut candidates = vec![];
        for candidate in self.metadata_manager.get_live_data_services_info() {
            if let Some(info) = candidate.1.back().as_ref() {
                // TODO(grao): Handle the case when the requested starting version is beyond the
                // latest version.
                if info.min_servable_version.is_none()
                    || starting_version < info.min_servable_version.unwrap()
                {
                    continue;
                }
                let num_active_streams = info.stream_info.as_ref().unwrap().active_streams.len();
                candidates.push((candidate.0, num_active_streams));
            }
        }

        Self::pick_data_service_from_candidate(candidates)
    }

    async fn pick_historical_data_service(&self, starting_version: u64) -> Option<String> {
        let file_store_version = self.data_manager.get_file_store_version().await;
        if starting_version >= file_store_version {
            return None;
        }

        let mut candidates = vec![];
        for candidate in self.metadata_manager.get_historical_data_services_info() {
            if let Some(info) = candidate.1.back().as_ref() {
                let num_active_streams = info.stream_info.as_ref().unwrap().active_streams.len();
                candidates.push((candidate.0, num_active_streams));
            }
        }

        Self::pick_data_service_from_candidate(candidates)
    }
}

#[tonic::async_trait]
impl GrpcManager for GrpcManagerService {
    async fn heartbeat(
        &self,
        request: Request<HeartbeatRequest>,
    ) -> Result<Response<HeartbeatResponse>, Status> {
        let request = request.into_inner();
        if let Some(service_info) = request.service_info {
            if let Some(address) = service_info.address {
                if let Some(info) = service_info.info {
                    return self
                        .handle_heartbeat(address, info)
                        .await
                        .map_err(|e| Status::internal(format!("Error handling heartbeat: {e}")));
                }
            }
        }

        Err(Status::invalid_argument("Bad request."))
    }

    async fn get_transactions(
        &self,
        request: Request<GetTransactionsRequest>,
    ) -> Result<Response<TransactionsResponse>, Status> {
        let request = request.into_inner();
        let transactions = self
            .data_manager
            .get_transactions(request.starting_version(), MAX_SIZE_BYTES_FROM_CACHE)
            .await
            .map_err(|e| Status::internal(format!("{e}")))?;

        Ok(Response::new(TransactionsResponse {
            transactions,
            chain_id: Some(self.chain_id),
            // Not used.
            processed_range: None,
        }))
    }

    async fn get_data_service_for_request(
        &self,
        request: Request<GetDataServiceForRequestRequest>,
    ) -> Result<Response<GetDataServiceForRequestResponse>, Status> {
        let request = request.into_inner();

        if request.user_request.is_none()
            || request
                .user_request
                .as_ref()
                .unwrap()
                .starting_version
                .is_none()
        {
            let candidates = self.metadata_manager.get_live_data_services_info();
            if let Some(candidate) = candidates.iter().next() {
                let data_service_address = candidate.0.clone();
                return Ok(Response::new(GetDataServiceForRequestResponse {
                    data_service_address,
                }));
            } else {
                return Err(Status::internal(
                    "Cannot find a data service instance to serve the provided request.",
                ));
            }
        }

        let starting_version = request.user_request.unwrap().starting_version();

        let data_service_address =
            // TODO(grao): Use a simple strategy for now. Consider to make it smarter in the
            // future.
            if let Some(address) = self.pick_live_data_service(starting_version) {
                COUNTER.with_label_values(&["live_data_service_picked"]).inc();
                address
            } else if let Some(address) = self.pick_historical_data_service(starting_version).await {
                COUNTER.with_label_values(&["historical_data_service_picked"]).inc();
                address
            } else {
                COUNTER.with_label_values(&["failed_to_pick_data_service"]).inc();
                return Err(Status::internal(
                    "Cannot find a data service instance to serve the provided request.",
                ));
            };

        Ok(Response::new(GetDataServiceForRequestResponse {
            data_service_address,
        }))
    }
}
