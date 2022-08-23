// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use aptos_config::network_id::PeerNetworkId;
use aptos_types::PeerId;
use async_trait::async_trait;
use network::{
    application::{
        interface::{MultiNetworkSender, NetworkInterface},
        storage::{LockingHashMap, PeerMetadataStorage},
    },
    peer_manager::{ConnectionRequestSender, PeerManagerRequestSender},
    protocols::network::{
        AppConfig, ApplicationNetworkSender, NetworkSender, NewNetworkSender, RpcError,
    },
    ProtocolId,
};
use peer_monitoring_service_types::{
    PeerMonitoringServiceError, PeerMonitoringServiceMessage, PeerMonitoringServiceRequest,
    PeerMonitoringServiceResponse,
};
use std::{sync::Arc, time::Duration};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Aptos network rpc error: {0}")]
    RpcError(#[from] RpcError),

    #[error("Error from remote monitoring service: {0}")]
    PeerMonitoringServiceError(#[from] PeerMonitoringServiceError),
}

/// The interface for sending peer monitoring service requests and querying
/// peer information.
#[derive(Clone, Debug)]
pub struct PeerMonitoringServiceClient {
    network_sender: PeerMonitoringServiceMultiSender,
    peer_metadata: Arc<PeerMetadataStorage>,
}

impl PeerMonitoringServiceClient {
    pub fn new(
        network_sender: PeerMonitoringServiceMultiSender,
        peer_metadata: Arc<PeerMetadataStorage>,
    ) -> Self {
        Self {
            network_sender,
            peer_metadata,
        }
    }

    pub async fn send_request(
        &self,
        recipient: PeerNetworkId,
        request: PeerMonitoringServiceRequest,
        timeout: Duration,
    ) -> Result<PeerMonitoringServiceResponse, Error> {
        let message = self
            .network_sender
            .send_rpc(
                recipient,
                PeerMonitoringServiceMessage::Request(request),
                timeout,
            )
            .await?;
        match message {
            PeerMonitoringServiceMessage::Response(Ok(response)) => Ok(response),
            PeerMonitoringServiceMessage::Response(Err(err)) => {
                Err(Error::PeerMonitoringServiceError(err))
            }
            PeerMonitoringServiceMessage::Request(_) => {
                Err(Error::RpcError(RpcError::InvalidRpcResponse))
            }
        }
    }
}

#[async_trait]
impl NetworkInterface<PeerMonitoringServiceMessage, PeerMonitoringServiceMultiSender>
    for PeerMonitoringServiceClient
{
    type AppDataKey = ();
    type AppData = ();

    fn peer_metadata_storage(&self) -> &PeerMetadataStorage {
        &self.peer_metadata
    }

    fn sender(&self) -> PeerMonitoringServiceMultiSender {
        unimplemented!("sender() is not required!")
    }

    fn app_data(&self) -> &LockingHashMap<Self::AppDataKey, Self::AppData> {
        unimplemented!("app_data() is not required!")
    }
}

/// A network sender that dispatches across multiple networks
pub type PeerMonitoringServiceMultiSender =
    MultiNetworkSender<PeerMonitoringServiceMessage, PeerMonitoringServiceNetworkSender>;

pub fn network_endpoint_config() -> AppConfig {
    AppConfig::client([ProtocolId::PeerMonitoringServiceRpc])
}

/// The peer monitoring service sender for a single network
#[derive(Clone, Debug)]
pub struct PeerMonitoringServiceNetworkSender {
    inner: NetworkSender<PeerMonitoringServiceMessage>,
}

impl NewNetworkSender for PeerMonitoringServiceNetworkSender {
    fn new(
        peer_manager_request_sender: PeerManagerRequestSender,
        connection_request_sender: ConnectionRequestSender,
    ) -> Self {
        Self {
            inner: NetworkSender::new(peer_manager_request_sender, connection_request_sender),
        }
    }
}

#[async_trait]
impl ApplicationNetworkSender<PeerMonitoringServiceMessage> for PeerMonitoringServiceNetworkSender {
    async fn send_rpc(
        &self,
        recipient: PeerId,
        message: PeerMonitoringServiceMessage,
        timeout: Duration,
    ) -> Result<PeerMonitoringServiceMessage, RpcError> {
        self.inner
            .send_rpc(
                recipient,
                ProtocolId::PeerMonitoringServiceRpc,
                message,
                timeout,
            )
            .await
    }
}
