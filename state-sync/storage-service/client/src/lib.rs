// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use aptos_config::network_id::PeerNetworkId;
use aptos_network2::{
    application::{interface::NetworkClientInterface, storage::PeersAndMetadata},
    protocols::network::RpcError,
};
use aptos_storage_service_types::{
    requests::StorageServiceRequest, responses::StorageServiceResponse, StorageServiceError,
    StorageServiceMessage,
};
use std::{collections::HashSet, sync::Arc, time::Duration};
use thiserror::Error;
use aptos_crypto::_once_cell::sync::Lazy;
use aptos_metrics_core::{histogram_opts, register_histogram_vec, HistogramVec};
use aptos_logger::{info,warn};


#[derive(Debug, Error)]
pub enum Error {
    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Network RPC error: {0}")]
    RpcError(#[from] RpcError),

    #[error("Error from remote storage service: {0}")]
    StorageServiceError(#[from] StorageServiceError),
}

/// The interface for sending Storage Service requests and
/// querying network peer information.
#[derive(Clone, Debug)]
pub struct StorageServiceClient<NetworkClient> {
    network_client: NetworkClient,
}

impl<NetworkClient: NetworkClientInterface<StorageServiceMessage>>
    StorageServiceClient<NetworkClient>
{
    pub fn new(network_client: NetworkClient) -> Self {
        Self { network_client }
    }

    pub async fn send_request(
        &self,
        recipient: PeerNetworkId,
        timeout: Duration,
        request: StorageServiceRequest,
    ) -> Result<StorageServiceResponse, Error> {
        let start = std::time::Instant::now();
        let request_label = request.data_request.get_label();
        let timer = REQUEST_LATENCIES_C.with_label_values(&[&request.get_label(), recipient.network_id().as_str()]).start_timer();
        let nr = self
            .network_client
            .send_to_peer_rpc(StorageServiceMessage::Request(request), timeout, recipient)
            .await;
        let dt = std::time::Instant::now().duration_since(start);
        let millis = dt.as_millis();
        let response = match nr {
            Ok(x) => {x}
            Err(error) => {
                warn!("storage RPC took {:?} ms; {:?} nerr {:?}", millis, request_label, error);
                timer.stop_and_discard();
                return Err(Error::NetworkError(error.to_string()));
            }
        };
        timer.observe_duration();
        if millis > 1100 {
            // log with detail below
        } else if millis > 500 {
            // sample!(SampleRate::Frequency(10), info!("storage RPC took {:?} ms", millis));
            info!("storage RPC took {:?} ms", millis);
        // } else if millis > 10 {
        //     sample!(SampleRate::Duration(Duration::from_secs(1)), info!("storage RPC took {:?}", dt));
        }

        match response {
            StorageServiceMessage::Response(Ok(response)) => {
                if millis > 1100 {
                    info!("storage RPC took {:?} ms; {:?} -> {:?}", millis, request_label, response.get_label());
                }
                Ok(response)
            },
            StorageServiceMessage::Response(Err(err)) => {
                if millis > 1100 {
                    warn!("storage RPC took {:?} ms; {:?} rerr {:?}", millis, request_label, err);
                }
                Err(Error::StorageServiceError(err))
            },
            StorageServiceMessage::Request(request) => {
                if millis > 1100 {
                    warn!("storage RPC took {:?} ms; {:?} reqreqerr {:?}", millis, request_label, request.data_request.get_label());
                }
                Err(Error::NetworkError(format!(
                    "Got storage service request instead of response! Request: {:?}",
                    request
                )))
            },
        }
    }

    pub fn get_available_peers(&self) -> Result<HashSet<PeerNetworkId>, Error> {
        self.network_client
            .get_available_peers()
            .map(|peers| peers.into_iter().collect())
            .map_err(|error| Error::NetworkError(error.to_string()))
    }

    pub fn get_peers_and_metadata(&self) -> Arc<PeersAndMetadata> {
        self.network_client.get_peers_and_metadata()
    }
}

// Latency buckets for network latencies (seconds)
const REQUEST_LATENCY_BUCKETS_SECS: &[f64] = &[
    0.05, 0.1, 0.2, 0.3, 0.5, 0.75, 1.0, 1.5, 2.0, 3.0, 5.0, 7.5, 10.0, 15.0, 20.0, 30.0, 40.0,
    60.0, 120.0, 180.0, 240.0, 300.0,
];

pub static REQUEST_LATENCIES_C: Lazy<HistogramVec> = Lazy::new(|| {
    let histogram_opts = histogram_opts!(
        "aptos_data_client_request_latencies_c",
        "Counters related to request latencies",
        REQUEST_LATENCY_BUCKETS_SECS.to_vec()
    );
    register_histogram_vec!(histogram_opts, &["request_type", "network"]).unwrap()
});
