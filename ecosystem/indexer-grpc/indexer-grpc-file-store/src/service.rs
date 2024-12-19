// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{data_manager::DataManager, metadata_manager::MetadataManager};
use aptos_protos::indexer::v1::{
    grpc_manager_server::GrpcManager, service_info::Info, GetDataServiceForRequestRequest,
    GetDataServiceForRequestResponse, GetTransactionsRequest, HeartbeatRequest, HeartbeatResponse,
    TransactionsResponse,
};
use std::sync::Arc;
use tonic::{Request, Response, Status};

const MAX_BATCH_SIZE: usize = 5 * (1 << 20);

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

    fn pick_live_data_service(&self, starting_version: u64) -> Option<String> {
        // TODO(grao): Picking the one with least # of streams for now, can be smarter in the
        // future.
        let mut min_num_streams = usize::MAX;
        let mut best_address = None;
        for candidate in self.metadata_manager.get_live_data_services_info() {
            if let Some(info) = candidate.1.back().as_ref() {
                // TODO(grao): Handle the case when the requested starting version is beyond the
                // latest version.
                if info.min_servable_version.is_none()
                    || starting_version < info.min_servable_version.unwrap()
                {
                    continue;
                }
                // TODO(grao): Validate the data at the metadata manager side to make sure
                // stream_info is always available.
                let num_active_streams = info.stream_info.as_ref().unwrap().active_streams.len();
                if num_active_streams < min_num_streams {
                    min_num_streams = num_active_streams;
                    best_address = Some(candidate.0);
                }
            } else {
                continue;
            }
        }

        best_address
    }

    async fn pick_historical_data_service(&self, starting_version: u64) -> Option<String> {
        let file_store_version = self.data_manager.get_file_store_version().await;
        if starting_version >= file_store_version {
            return None;
        }

        // TODO(grao): Picking the one with least # of streams for now, can be smarter in the
        // future.
        let mut min_num_streams = usize::MAX;
        let mut best_address = None;
        for candidate in self.metadata_manager.get_historical_data_services_info() {
            if let Some(info) = candidate.1.back().as_ref() {
                // TODO(grao): Validate the data at the metadata manager side to make sure
                // stream_info is always available.
                let num_active_streams = info.stream_info.as_ref().unwrap().active_streams.len();
                if num_active_streams < min_num_streams {
                    min_num_streams = num_active_streams;
                    best_address = Some(candidate.0);
                }
            } else {
                continue;
            }
        }

        best_address
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
                        .map_err(|e| Status::internal(&format!("Error handling heartbeat: {e}")));
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
            .get_transactions(request.starting_version(), MAX_BATCH_SIZE)
            .await
            .map_err(|e| Status::internal(format!("{e}")))?;

        Ok(Response::new(TransactionsResponse {
            transactions,
            chain_id: Some(self.chain_id),
        }))
    }

    async fn get_data_service_for_request(
        &self,
        request: Request<GetDataServiceForRequestRequest>,
    ) -> Result<Response<GetDataServiceForRequestResponse>, Status> {
        let request = request.into_inner();

        if request.user_request.is_none() {
            return Err(Status::invalid_argument("Bad request."));
        }

        let user_request = request.user_request.unwrap();
        if user_request.starting_version.is_none() {
            return Err(Status::invalid_argument("Bad request."));
        }

        let starting_version = user_request.starting_version();

        let data_service_address =
            // TODO(grao): Use a simple strategy for now. Consider to make it smarter in the
            // future.
            if let Some(address) = self.pick_live_data_service(starting_version) {
                address
            } else if let Some(address) = self.pick_historical_data_service(starting_version).await {
                address
            } else {
                return Err(Status::internal(
                    "Cannot find a data service instance to serve the provided request.",
                ));
            };

        Ok(Response::new(GetDataServiceForRequestResponse {
            data_service_address: Some(data_service_address),
        }))
    }
}
