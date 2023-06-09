// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::response::{NetworkInformationResponse, NodeInformationResponse};
use request::PeerMonitoringServiceRequest;
use response::PeerMonitoringServiceResponse;
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    fmt::{Debug, Display},
};
use thiserror::Error;

pub mod request;
pub mod response;

pub type Result<T, E = PeerMonitoringServiceError> = ::std::result::Result<T, E>;

/// Useful global constants
pub const MAX_DISTANCE_FROM_VALIDATORS: u64 = 100; // Nodes that aren't connected to the network

/// An error that can be returned to the client on a failure to
/// process a request.
#[derive(Clone, Debug, Deserialize, Error, PartialEq, Eq, Serialize)]
pub enum PeerMonitoringServiceError {
    #[error("Internal service error: {0}")]
    InternalError(String),
    #[error("Invalid service request: {0}")]
    InvalidRequest(String),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[allow(clippy::large_enum_variant)]
pub enum PeerMonitoringServiceMessage {
    /// A request to the peer monitoring service
    Request(PeerMonitoringServiceRequest),
    /// A response from the peer monitoring service
    Response(Result<PeerMonitoringServiceResponse>),
    DirectNetPerformance(DirectNetPerformanceMessage),
}

/// The peer monitoring metadata for a peer
#[derive(Clone, Default, Deserialize, PartialEq, Serialize)]
pub struct PeerMonitoringMetadata {
    pub average_ping_latency_secs: Option<f64>, // The average latency ping for the peer
    pub latest_network_info_response: Option<NetworkInformationResponse>, // The latest network info response
    pub latest_node_info_response: Option<NodeInformationResponse>, // The latest node info response
    pub internal_client_state: Option<String>, // A detailed client state string for debugging and logging
}

/// We must manually define this because f64 doesn't implement Eq. Instead,
/// we rely on PartialEq (which is sufficient for our use-cases).
impl Eq for PeerMonitoringMetadata {}

impl PeerMonitoringMetadata {
    pub fn new(
        average_ping_latency_secs: Option<f64>,
        latest_network_info_response: Option<NetworkInformationResponse>,
        latest_node_info_response: Option<NodeInformationResponse>,
        internal_client_state: Option<String>,
    ) -> Self {
        PeerMonitoringMetadata {
            average_ping_latency_secs,
            latest_network_info_response,
            latest_node_info_response,
            internal_client_state,
        }
    }
}

// Display formatting includes basic monitoring metadata
impl Display for PeerMonitoringMetadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{ average_ping_latency_secs: {}, latest_network_info_response: {}, latest_node_info_response: {} }}",
            display_format_option(&self.average_ping_latency_secs),
            display_format_option(&self.latest_network_info_response),
            display_format_option(&self.latest_node_info_response),
        )
    }
}

// Debug formatting includes more detailed monitoring metadata
// (but not the internal client state string).
impl Debug for PeerMonitoringMetadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{ average_ping_latency_secs: {}, latest_network_info_response: {}, latest_node_info_response: {} }}",
            debug_format_option(&self.average_ping_latency_secs),
            debug_format_option(&self.latest_network_info_response),
            debug_format_option(&self.latest_node_info_response),
        )
    }
}

/// A simple utility function for debug formatting an optional value
fn debug_format_option<T: Debug>(option: &Option<T>) -> String {
    option
        .as_ref()
        .map(|value| format!("{:?}", value))
        .unwrap_or_else(|| "None".to_string())
}

/// A simple utility function for display formatting an optional value
fn display_format_option<T: Display>(option: &Option<T>) -> String {
    option
        .as_ref()
        .map(|value| format!("{}", value))
        .unwrap_or_else(|| "None".to_string())
}

/// Common to Client and Server tasks.
/// Probably wrapped Arc<RwLock<PeerMonitoringSharedState>>
pub struct PeerMonitoringSharedState {

    // Circular buffer of sent records
    sent: Vec<SendRecord>,
    // sent[sent_pos] is the next index to write
    sent_pos: usize,
}

impl PeerMonitoringSharedState {
    pub fn new() -> Self {
        PeerMonitoringSharedState {
            sent: Vec::with_capacity(10000), // TODO: constant or config
            sent_pos: 0,
        }
    }

    pub fn set(&mut self, sent: SendRecord) {
        if self.sent.len() < self.sent.capacity() {
            self.sent.push(sent);
        } else {
            self.sent[self.sent_pos] = sent;
        }
        self.sent_pos = (self.sent_pos + 1) % self.sent.capacity();
    }

    /// return the record for the request_counter, or {0, oldest send_micros}
    pub fn find(&self, request_counter: u64) -> SendRecord {
        if self.sent.len() == 0 {
            return SendRecord{request_counter: 0, send_micros: 0, bytes_sent: 0};
        }
        let mut oldest = self.sent[0].send_micros;
        let capacity = self.sent.len();
        for i in 0..capacity {
            let pos = (self.sent_pos + capacity - (1+i)) % capacity;
            let rec = self.sent[pos].clone();
            if rec.request_counter == request_counter {
                return rec;
            }
            if rec.send_micros < oldest {
                oldest = rec.send_micros;
            }
        }
        SendRecord{request_counter: 0, send_micros: oldest, bytes_sent: 0}
    }
}

#[derive(Clone)]
pub struct SendRecord {
    pub request_counter: u64,
    pub send_micros: i64,
    pub bytes_sent: usize,
}

#[derive(Clone,Debug,Deserialize,Serialize)]
pub struct DirectNetPerformanceMessage {
    pub request_counter: u64, // A monotonically increasing counter to verify responses
    pub send_micros: i64, //
    pub data: Vec<u8>, // A vector of bytes to send in the request
}
