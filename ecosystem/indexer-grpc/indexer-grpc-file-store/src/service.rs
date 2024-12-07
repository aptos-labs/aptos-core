// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{data_manager::DataManager, metadata_manager::MetadataManager};
use aptos_protos::indexer::v1::{
    grpc_manager_server::GrpcManager, service_info::ServiceType, GetTransactionsRequest,
    HeartbeatRequest, HeartbeatResponse, TransactionsResponse,
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
        service_type: ServiceType,
    ) -> anyhow::Result<Response<HeartbeatResponse>> {
        self.metadata_manager
            .handle_heartbeat(address, service_type)?;

        Ok(Response::new(HeartbeatResponse {
            known_latest_version: Some(self.metadata_manager.get_known_latest_version()),
        }))
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
                if let Some(service_type) = service_info.service_type {
                    return self
                        .handle_heartbeat(address, service_type)
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
}
