// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_protos::indexer::v1::{
    grpc_manager_server::GrpcManager, service_info::Info, GetDataServiceForRequestRequest,
    GetDataServiceForRequestResponse, GetTransactionsRequest, HeartbeatRequest, HeartbeatResponse,
    TransactionsResponse,
};
use tonic::{Request, Response, Status};

pub struct GrpcManagerService {
    chain_id: u64,
}

impl GrpcManagerService {
    pub(crate) fn new(chain_id: u64) -> Self {
        Self { chain_id }
    }

    async fn handle_heartbeat(
        &self,
        _address: String,
        _info: Info,
    ) -> anyhow::Result<Response<HeartbeatResponse>> {
        // TODO(grao): Implement.
        todo!()
    }

    fn pick_live_data_service(&self, _starting_version: u64) -> Option<String> {
        // TODO(grao): Implement.
        todo!()
    }

    async fn pick_historical_data_service(&self, _starting_version: u64) -> Option<String> {
        // TODO(grao): Implement.
        todo!()
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
        let _request = request.into_inner();
        let transactions = vec![];
        // TODO(grao): Implement.

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
            data_service_address,
        }))
    }
}
