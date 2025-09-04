// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{config::LIVE_DATA_SERVICE, connection_manager::ConnectionManager};
use anyhow::Result;
use velor_indexer_grpc_utils::timestamp_now_proto;
use velor_protos::indexer::v1::{
    data_service_server::DataService, ping_data_service_response::Info, raw_data_server::RawData,
    GetTransactionsRequest, HistoricalDataServiceInfo, LiveDataServiceInfo, PingDataServiceRequest,
    PingDataServiceResponse, StreamInfo, TransactionsResponse,
};
use futures::{Stream, StreamExt};
use std::{pin::Pin, sync::Arc};
use tokio::sync::mpsc::{channel, Sender};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

type ResponseStream = Pin<Box<dyn Stream<Item = Result<TransactionsResponse, Status>> + Send>>;

// Note: We still allow starting both services together, so people don't have to rely on
// GrpcManager for routing, and it's also make it easier to run in testing environment.
pub struct DataServiceWrapperWrapper {
    live_data_service: Option<DataServiceWrapper>,
    historical_data_service: Option<DataServiceWrapper>,
}

impl DataServiceWrapperWrapper {
    pub fn new(
        live_data_service: Option<DataServiceWrapper>,
        historical_data_service: Option<DataServiceWrapper>,
    ) -> Self {
        Self {
            live_data_service,
            historical_data_service,
        }
    }
}

#[tonic::async_trait]
impl DataService for DataServiceWrapperWrapper {
    type GetTransactionsStream = ResponseStream;

    async fn get_transactions(
        &self,
        req: Request<GetTransactionsRequest>,
    ) -> Result<Response<Self::GetTransactionsStream>, Status> {
        if let Some(live_data_service) = self.live_data_service.as_ref() {
            if let Some(historical_data_service) = self.historical_data_service.as_ref() {
                let request = req.into_inner();
                let mut stream = live_data_service
                    .get_transactions(Request::new(request.clone()))
                    .await?
                    .into_inner();
                let peekable = std::pin::pin!(stream.as_mut().peekable());
                if let Some(Ok(_)) = peekable.peek().await {
                    return live_data_service
                        .get_transactions(Request::new(request.clone()))
                        .await;
                }

                historical_data_service
                    .get_transactions(Request::new(request))
                    .await
            } else {
                live_data_service.get_transactions(req).await
            }
        } else if let Some(historical_data_service) = self.historical_data_service.as_ref() {
            historical_data_service.get_transactions(req).await
        } else {
            unreachable!("Must have at least one of the data services enabled.");
        }
    }

    async fn ping(
        &self,
        req: Request<PingDataServiceRequest>,
    ) -> Result<Response<PingDataServiceResponse>, Status> {
        let request = req.get_ref();
        if request.ping_live_data_service {
            if let Some(live_data_service) = self.live_data_service.as_ref() {
                live_data_service.ping(req).await
            } else {
                Err(Status::not_found("LiveDataService is not enabled."))
            }
        } else if let Some(historical_data_service) = self.historical_data_service.as_ref() {
            historical_data_service.ping(req).await
        } else {
            Err(Status::not_found("HistoricalDataService is not enabled."))
        }
    }
}

#[tonic::async_trait]
impl RawData for DataServiceWrapperWrapper {
    type GetTransactionsStream = ResponseStream;

    async fn get_transactions(
        &self,
        req: Request<GetTransactionsRequest>,
    ) -> Result<Response<Self::GetTransactionsStream>, Status> {
        DataService::get_transactions(self, req).await
    }
}

pub struct DataServiceWrapper {
    connection_manager: Arc<ConnectionManager>,
    handler_tx: Sender<(
        Request<GetTransactionsRequest>,
        Sender<Result<TransactionsResponse, Status>>,
    )>,
    pub data_service_response_channel_size: usize,
    is_live_data_service: bool,
}

impl DataServiceWrapper {
    pub fn new(
        connection_manager: Arc<ConnectionManager>,
        handler_tx: Sender<(
            Request<GetTransactionsRequest>,
            Sender<Result<TransactionsResponse, Status>>,
        )>,
        data_service_response_channel_size: usize,
        is_live_data_service: bool,
    ) -> Self {
        Self {
            connection_manager,
            handler_tx,
            data_service_response_channel_size,
            is_live_data_service,
        }
    }
}

#[tonic::async_trait]
impl DataService for DataServiceWrapper {
    type GetTransactionsStream = ResponseStream;

    async fn get_transactions(
        &self,
        req: Request<GetTransactionsRequest>,
    ) -> Result<Response<Self::GetTransactionsStream>, Status> {
        let (tx, rx) = channel(self.data_service_response_channel_size);
        self.handler_tx.send((req, tx)).await.unwrap();

        let output_stream = ReceiverStream::new(rx);
        let response = Response::new(Box::pin(output_stream) as Self::GetTransactionsStream);

        Ok(response)
    }

    async fn ping(
        &self,
        req: Request<PingDataServiceRequest>,
    ) -> Result<Response<PingDataServiceResponse>, Status> {
        let request = req.into_inner();
        if request.ping_live_data_service != self.is_live_data_service {
            if request.ping_live_data_service {
                return Err(Status::not_found("LiveDataService is not enabled."));
            } else {
                return Err(Status::not_found("HistoricalDataService is not enabled."));
            }
        }

        let known_latest_version = request.known_latest_version();
        self.connection_manager
            .update_known_latest_version(known_latest_version);
        let stream_info = StreamInfo {
            active_streams: self.connection_manager.get_active_streams(),
        };

        let response = if self.is_live_data_service {
            let min_servable_version = match LIVE_DATA_SERVICE.get() {
                Some(svc) => Some(svc.get_min_servable_version().await),
                None => None,
            };
            let info = LiveDataServiceInfo {
                chain_id: self.connection_manager.chain_id(),
                timestamp: Some(timestamp_now_proto()),
                known_latest_version: Some(known_latest_version),
                stream_info: Some(stream_info),
                min_servable_version,
            };
            PingDataServiceResponse {
                info: Some(Info::LiveDataServiceInfo(info)),
            }
        } else {
            let info = HistoricalDataServiceInfo {
                chain_id: self.connection_manager.chain_id(),
                timestamp: Some(timestamp_now_proto()),
                known_latest_version: Some(known_latest_version),
                stream_info: Some(stream_info),
            };
            PingDataServiceResponse {
                info: Some(Info::HistoricalDataServiceInfo(info)),
            }
        };

        Ok(Response::new(response))
    }
}
