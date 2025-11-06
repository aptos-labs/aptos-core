// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    peer_states::{key_value::StateValueInterface, request_tracker::RequestTracker},
    Error, LogEntry, LogEvent, LogSchema,
};
use aptos_config::{config::TransactionMonitoringConfig, network_id::PeerNetworkId};
use aptos_infallible::RwLock;
use aptos_logger::warn;
use aptos_network::application::metadata::PeerMetadata;
use aptos_peer_monitoring_service_types::{
    request::PeerMonitoringServiceRequest,
    response::{PeerMonitoringServiceResponse, TransactionInformationResponse},
};
use aptos_time_service::TimeService;
use std::{
    fmt,
    fmt::{Display, Formatter},
    sync::Arc,
};

/// A simple container that holds a single peer's transaction info
#[derive(Clone, Debug)]
pub struct TransactionInfoState {
    transaction_monitoring_config: TransactionMonitoringConfig, // The config for transaction monitoring
    recorded_transaction_info_response: Option<TransactionInformationResponse>, // The last transaction info response
    request_tracker: Arc<RwLock<RequestTracker>>, // The request tracker for transaction info requests
}

impl TransactionInfoState {
    pub fn new(
        transaction_monitoring_config: TransactionMonitoringConfig,
        time_service: TimeService,
    ) -> Self {
        let request_tracker = RequestTracker::new(
            transaction_monitoring_config.transaction_info_request_interval_ms,
            time_service,
        );

        Self {
            transaction_monitoring_config,
            recorded_transaction_info_response: None,
            request_tracker: Arc::new(RwLock::new(request_tracker)),
        }
    }

    /// Records the new transaction info response for the peer
    pub fn record_transaction_info_response(
        &mut self,
        transaction_info_response: TransactionInformationResponse,
    ) {
        // Update the request tracker with a successful response
        self.request_tracker.write().record_response_success();

        // Save the transaction info
        self.recorded_transaction_info_response = Some(transaction_info_response);
    }

    /// Handles a request failure for the specified peer
    fn handle_request_failure(&self) {
        self.request_tracker.write().record_response_failure();
    }

    /// Returns the latest transaction info response
    pub fn get_latest_transaction_info_response(&self) -> Option<TransactionInformationResponse> {
        self.recorded_transaction_info_response.clone()
    }
}

impl StateValueInterface for TransactionInfoState {
    fn create_monitoring_service_request(&mut self) -> PeerMonitoringServiceRequest {
        PeerMonitoringServiceRequest::GetTransactionInformation
    }

    fn get_request_timeout_ms(&self) -> u64 {
        self.transaction_monitoring_config
            .transaction_info_request_timeout_ms
    }

    fn get_request_tracker(&self) -> Arc<RwLock<RequestTracker>> {
        self.request_tracker.clone()
    }

    fn handle_monitoring_service_response(
        &mut self,
        peer_network_id: &PeerNetworkId,
        _peer_metadata: PeerMetadata,
        _monitoring_service_request: PeerMonitoringServiceRequest,
        monitoring_service_response: PeerMonitoringServiceResponse,
        _response_time_secs: f64,
    ) {
        // Verify the response type is valid
        let transaction_info_response = match monitoring_service_response {
            PeerMonitoringServiceResponse::TransactionInformation(
                transaction_information_response,
            ) => transaction_information_response,
            _ => {
                warn!(LogSchema::new(LogEntry::TransactionInfoRequest)
                    .event(LogEvent::ResponseError)
                    .peer(peer_network_id)
                    .message(
                        "An unexpected response was received instead of a transaction info response!"
                    ));
                self.handle_request_failure();
                return;
            },
        };

        // Store the new transaction info response
        self.record_transaction_info_response(transaction_info_response);
    }

    fn handle_monitoring_service_response_error(
        &mut self,
        peer_network_id: &PeerNetworkId,
        error: Error,
    ) {
        // Handle the failure
        self.handle_request_failure();

        // Log the error
        warn!(LogSchema::new(LogEntry::TransactionInfoRequest)
            .event(LogEvent::ResponseError)
            .message("Error encountered when requesting transaction information from the peer!")
            .peer(peer_network_id)
            .error(&error));
    }

    fn update_peer_state_metrics(&self, _peer_network_id: &PeerNetworkId) {
        // This function doesn't need to update any metrics
    }
}

impl Display for TransactionInfoState {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TransactionInfoState {{ recorded_transaction_info_response: {:?} }}",
            self.recorded_transaction_info_response
        )
    }
}
