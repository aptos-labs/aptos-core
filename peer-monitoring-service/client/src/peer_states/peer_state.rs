// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    metrics, network,
    peer_states::{
        key_value::{PeerStateKey, PeerStateValue, StateValueInterface},
        latency_info::LatencyInfoState,
        network_info::NetworkInfoState,
        node_info::NodeInfoState,
        request_tracker::RequestTracker,
    },
    Error, PeerMonitoringServiceClient,
};
use velor_config::{
    config::{NodeConfig, PeerMonitoringServiceConfig},
    network_id::PeerNetworkId,
};
use velor_id_generator::{IdGenerator, U64IdGenerator};
use velor_infallible::RwLock;
use velor_network::application::{interface::NetworkClient, metadata::PeerMetadata};
use velor_peer_monitoring_service_types::{
    response::PeerMonitoringServiceResponse, PeerMonitoringMetadata, PeerMonitoringServiceMessage,
};
use velor_time_service::{TimeService, TimeServiceTrait};
use rand::{rngs::OsRng, Rng};
use std::{
    collections::HashMap,
    fmt,
    fmt::{Display, Formatter},
    ops::Deref,
    sync::Arc,
    time::Duration,
};
use tokio::{runtime::Handle, task::JoinHandle, time::sleep};

#[derive(Clone, Debug)]
pub struct PeerState {
    state_entries: Arc<RwLock<HashMap<PeerStateKey, Arc<RwLock<PeerStateValue>>>>>, // The state entries for the peer
}

impl PeerState {
    pub fn new(node_config: NodeConfig, time_service: TimeService) -> Self {
        // Create a state entry for each peer state key
        let state_entries = Arc::new(RwLock::new(HashMap::new()));
        for peer_state_key in PeerStateKey::get_all_keys() {
            let peer_state_value =
                PeerStateValue::new(node_config.clone(), time_service.clone(), &peer_state_key);
            state_entries
                .write()
                .insert(peer_state_key, Arc::new(RwLock::new(peer_state_value)));
        }

        Self { state_entries }
    }

    /// Returns the request tracker for the given peer state key
    pub fn get_request_tracker(
        &self,
        peer_state_key: &PeerStateKey,
    ) -> Result<Arc<RwLock<RequestTracker>>, Error> {
        self.get_peer_state_value(peer_state_key)
            .map(|peer_state_value| peer_state_value.read().get_request_tracker())
    }

    /// Refreshes the peer state key by sending a request to the peer
    pub fn refresh_peer_state_key(
        &self,
        monitoring_service_config: &PeerMonitoringServiceConfig,
        peer_state_key: &PeerStateKey,
        peer_monitoring_client: PeerMonitoringServiceClient<
            NetworkClient<PeerMonitoringServiceMessage>,
        >,
        peer_network_id: PeerNetworkId,
        peer_metadata: PeerMetadata,
        request_id_generator: Arc<U64IdGenerator>,
        time_service: TimeService,
        runtime: Option<Handle>,
    ) -> Result<JoinHandle<()>, Error> {
        // Mark the request as having started. We do this here to prevent
        // the monitor loop from selecting the same peer state key concurrently.
        let request_tracker = self.get_request_tracker(peer_state_key)?;
        request_tracker.write().request_started();

        // Create the monitoring service request for the peer
        let peer_state_value = self.get_peer_state_value(peer_state_key)?;
        let monitoring_service_request =
            peer_state_value.write().create_monitoring_service_request();

        // Get the jitter and timeout for the request
        let request_jitter_ms = OsRng.gen_range(0, monitoring_service_config.max_request_jitter_ms);
        let request_timeout_ms = peer_state_value.read().get_request_timeout_ms();

        // Get the max message size for the response
        let max_num_response_bytes = monitoring_service_config.max_num_response_bytes;

        // Create the request task
        let request_task = async move {
            // Add some amount of jitter before sending the request.
            // This helps to prevent requests from becoming too bursty.
            sleep(Duration::from_millis(request_jitter_ms)).await;

            // Start the request timer
            let start_time = time_service.now();

            // Send the request to the peer and wait for a response
            let request_id = request_id_generator.next();
            let monitoring_service_response = network::send_request_to_peer(
                peer_monitoring_client,
                &peer_network_id,
                request_id,
                monitoring_service_request.clone(),
                request_timeout_ms,
            )
            .await;

            // Stop the timer and calculate the duration
            let request_duration_secs = start_time.elapsed().as_secs_f64();

            // Mark the in-flight request as now complete
            request_tracker.write().request_completed();

            // Process any response errors
            let monitoring_service_response = match monitoring_service_response {
                Ok(monitoring_service_response) => monitoring_service_response,
                Err(error) => {
                    peer_state_value
                        .write()
                        .handle_monitoring_service_response_error(&peer_network_id, error);
                    return;
                },
            };

            // Verify the response respects the message size limits
            if let Err(error) =
                sanity_check_response_size(max_num_response_bytes, &monitoring_service_response)
            {
                peer_state_value
                    .write()
                    .handle_monitoring_service_response_error(&peer_network_id, error);
                return;
            }

            // Handle the monitoring service response
            peer_state_value.write().handle_monitoring_service_response(
                &peer_network_id,
                peer_metadata,
                monitoring_service_request.clone(),
                monitoring_service_response,
                request_duration_secs,
            );

            // Update the latency ping metrics
            metrics::observe_value_with_label(
                &metrics::REQUEST_LATENCIES,
                monitoring_service_request.get_label(),
                &peer_network_id,
                request_duration_secs,
            );
        };

        // Spawn the request task
        let join_handle = if let Some(runtime) = runtime {
            runtime.spawn(request_task)
        } else {
            tokio::spawn(request_task)
        };

        Ok(join_handle)
    }

    /// Updates the peer metrics for the given peer state key
    pub fn update_peer_state_metrics(
        &self,
        peer_network_id: &PeerNetworkId,
        peer_state_key: &PeerStateKey,
    ) -> Result<(), Error> {
        let peer_state_value = self.get_peer_state_value(peer_state_key)?;
        peer_state_value
            .read()
            .update_peer_state_metrics(peer_network_id);

        Ok(())
    }

    /// Extracts peer monitoring metadata from the overall peer state
    pub fn extract_peer_monitoring_metadata(&self) -> Result<PeerMonitoringMetadata, Error> {
        // Create an empty metadata entry for the peer
        let mut peer_monitoring_metadata = PeerMonitoringMetadata::default();

        // Get and store the average latency ping
        let latency_info_state = self.get_latency_info_state()?;
        let average_latency_ping_secs = latency_info_state.get_average_latency_ping_secs();
        peer_monitoring_metadata.average_ping_latency_secs = average_latency_ping_secs;

        let latest_ping_latency_secs = latency_info_state.get_latest_latency_ping_secs();
        peer_monitoring_metadata.latest_ping_latency_secs = latest_ping_latency_secs;

        // Get and store the detailed monitoring metadata
        let internal_client_state = self.get_internal_client_state()?;
        peer_monitoring_metadata.internal_client_state = internal_client_state;

        // Get and store the latest network info response
        let network_info_state = self.get_network_info_state()?;
        let network_info_response = network_info_state.get_latest_network_info_response();
        peer_monitoring_metadata.latest_network_info_response = network_info_response;

        // Get and store the latest node info response
        let node_info_state = self.get_node_info_state()?;
        let node_info_response = node_info_state.get_latest_node_info_response();
        peer_monitoring_metadata.latest_node_info_response = node_info_response;

        Ok(peer_monitoring_metadata)
    }

    /// Returns the peer state value associated with the given key
    pub(crate) fn get_peer_state_value(
        &self,
        peer_state_key: &PeerStateKey,
    ) -> Result<Arc<RwLock<PeerStateValue>>, Error> {
        let peer_state_value = self.state_entries.read().get(peer_state_key).cloned();
        peer_state_value.ok_or_else(|| {
            Error::UnexpectedError(format!(
                "Failed to find the peer state value for the peer state key: {:?} This shouldn't happen!",
                peer_state_key
            ))
        })
    }

    /// Returns a copy of the latency ping state
    pub(crate) fn get_latency_info_state(&self) -> Result<LatencyInfoState, Error> {
        let peer_state_value = self
            .get_peer_state_value(&PeerStateKey::LatencyInfo)?
            .read()
            .clone();
        match peer_state_value {
            PeerStateValue::LatencyInfoState(latency_info_state) => Ok(latency_info_state),
            peer_state_value => Err(Error::UnexpectedError(format!(
                "Invalid peer state value found! Expected latency_info_state but got: {:?}",
                peer_state_value
            ))),
        }
    }

    /// Returns a copy of the network info state
    pub(crate) fn get_network_info_state(&self) -> Result<NetworkInfoState, Error> {
        let peer_state_value = self
            .get_peer_state_value(&PeerStateKey::NetworkInfo)?
            .read()
            .clone();
        match peer_state_value {
            PeerStateValue::NetworkInfoState(network_info_state) => Ok(network_info_state),
            peer_state_value => Err(Error::UnexpectedError(format!(
                "Invalid peer state value found! Expected network_info_state but got: {:?}",
                peer_state_value
            ))),
        }
    }

    /// Returns a copy of the node info state
    pub(crate) fn get_node_info_state(&self) -> Result<NodeInfoState, Error> {
        let peer_state_value = self
            .get_peer_state_value(&PeerStateKey::NodeInfo)?
            .read()
            .clone();
        match peer_state_value {
            PeerStateValue::NodeInfoState(node_info_state) => Ok(node_info_state),
            peer_state_value => Err(Error::UnexpectedError(format!(
                "Invalid peer state value found! Expected node_info_state but got: {:?}",
                peer_state_value
            ))),
        }
    }

    /// Returns a detailed internal state string (for logging and debugging purposes)
    fn get_internal_client_state(&self) -> Result<Option<String>, Error> {
        // Construct a string map for each of the state entries
        let mut client_state_strings = HashMap::new();
        for (state_key, state_value) in self.state_entries.read().iter() {
            let peer_state_label = state_key.get_label().to_string();
            let peer_state_value = format!("{}", state_value.read().deref());
            client_state_strings.insert(peer_state_label, peer_state_value);
        }

        // Pretty print and return the client state string
        let client_state_string =
            serde_json::to_string_pretty(&client_state_strings).map_err(|error| {
                Error::UnexpectedError(format!(
                    "Failed to serialize the client state string: {:?}",
                    error
                ))
            })?;
        Ok(Some(client_state_string))
    }
}

impl Display for PeerState {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // Display format the monitoring metadata
        let peer_monitoring_metadata = self.extract_peer_monitoring_metadata();
        let output_string = match peer_monitoring_metadata {
            Ok(peer_monitoring_metadata) => format!("{}", peer_monitoring_metadata),
            Err(error) => format!("{:?}", error),
        };

        // Write the string to the formatter
        write!(f, "PeerState {{ {} }}", output_string)
    }
}

/// Sanity checks that the monitoring service response size
/// is valid (i.e., it respects the max message size).
fn sanity_check_response_size(
    max_num_response_bytes: u64,
    monitoring_service_response: &PeerMonitoringServiceResponse,
) -> Result<(), Error> {
    // Calculate the number of bytes in the response
    let num_response_bytes = monitoring_service_response.get_num_bytes()?;

    // Verify the response respects the max message sizes
    if num_response_bytes > max_num_response_bytes {
        return Err(Error::UnexpectedError(format!(
            "The monitoring service response ({:?}) is too large: {:?}. Maximum allowed: {:?}",
            monitoring_service_response.get_label(),
            num_response_bytes,
            max_num_response_bytes
        )));
    }

    Ok(())
}
