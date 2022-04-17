// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::protocols::wire::handshake::v1::ProtocolId;
use aptos_config::network_id::NetworkContext;
use aptos_metrics::{
    register_histogram_vec, register_int_counter_vec, register_int_gauge, register_int_gauge_vec,
    Histogram, HistogramVec, IntCounter, IntCounterVec, IntGauge, IntGaugeVec,
};
use aptos_types::PeerId;
use flate2::{
    write::{DeflateDecoder, DeflateEncoder, GzDecoder, GzEncoder, ZlibDecoder, ZlibEncoder},
    Compression,
};
use miniz_oxide::{deflate::compress_to_vec, inflate::decompress_to_vec};
use netcore::transport::ConnectionOrigin;
use once_cell::sync::Lazy;
use short_hex_str::AsShortHexStr;
use snap::{read, write};
use std::{
    io::{Read, Write},
    time::{Duration, Instant},
};

// some type labels
pub const REQUEST_LABEL: &str = "request";
pub const RESPONSE_LABEL: &str = "response";

// some state labels
pub const CANCELED_LABEL: &str = "canceled";
pub const DECLINED_LABEL: &str = "declined";
pub const RECEIVED_LABEL: &str = "received";
pub const SENT_LABEL: &str = "sent";
pub const SUCCEEDED_LABEL: &str = "succeeded";
pub const FAILED_LABEL: &str = "failed";

pub static APTOS_CONNECTIONS: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "aptos_connections",
        "Number of current connections and their direction",
        &["role_type", "network_id", "peer_id", "direction"]
    )
    .unwrap()
});

pub fn connections(network_context: &NetworkContext, origin: ConnectionOrigin) -> IntGauge {
    APTOS_CONNECTIONS.with_label_values(&[
        network_context.role().as_str(),
        network_context.network_id().as_str(),
        network_context.peer_id().short_str().as_str(),
        origin.as_str(),
    ])
}

pub static APTOS_CONNECTIONS_REJECTED: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_connections_rejected",
        "Number of connections rejected per interface",
        &["role_type", "network_id", "peer_id", "direction"]
    )
    .unwrap()
});

pub fn connections_rejected(
    network_context: &NetworkContext,
    origin: ConnectionOrigin,
) -> IntCounter {
    APTOS_CONNECTIONS_REJECTED.with_label_values(&[
        network_context.role().as_str(),
        network_context.network_id().as_str(),
        network_context.peer_id().short_str().as_str(),
        origin.as_str(),
    ])
}

pub static APTOS_NETWORK_PEER_CONNECTED: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "aptos_network_peer_connected",
        "Indicates if we are connected to a particular peer",
        &["role_type", "network_id", "peer_id", "remote_peer_id"]
    )
    .unwrap()
});

pub fn peer_connected(network_context: &NetworkContext, remote_peer_id: &PeerId, v: i64) {
    if network_context.network_id().is_validator_network() {
        APTOS_NETWORK_PEER_CONNECTED
            .with_label_values(&[
                network_context.role().as_str(),
                network_context.network_id().as_str(),
                network_context.peer_id().short_str().as_str(),
                remote_peer_id.short_str().as_str(),
            ])
            .set(v)
    }
}

/// Increments the counter based on `NetworkContext`
pub fn inc_by_with_context(
    counter: &IntCounterVec,
    network_context: &NetworkContext,
    label: &str,
    val: u64,
) {
    counter
        .with_label_values(&[
            network_context.role().as_str(),
            network_context.network_id().as_str(),
            network_context.peer_id().short_str().as_str(),
            label,
        ])
        .inc_by(val)
}

pub static APTOS_NETWORK_PENDING_CONNECTION_UPGRADES: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "aptos_network_pending_connection_upgrades",
        "Number of concurrent inbound or outbound connections we're currently negotiating",
        &["role_type", "network_id", "peer_id", "direction"]
    )
    .unwrap()
});

pub fn pending_connection_upgrades(
    network_context: &NetworkContext,
    direction: ConnectionOrigin,
) -> IntGauge {
    APTOS_NETWORK_PENDING_CONNECTION_UPGRADES.with_label_values(&[
        network_context.role().as_str(),
        network_context.network_id().as_str(),
        network_context.peer_id().short_str().as_str(),
        direction.as_str(),
    ])
}

pub static APTOS_NETWORK_CONNECTION_UPGRADE_TIME: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_network_connection_upgrade_time_seconds",
        "Time to complete a new inbound or outbound connection upgrade",
        &["role_type", "network_id", "peer_id", "direction", "state"]
    )
    .unwrap()
});

pub fn connection_upgrade_time(
    network_context: &NetworkContext,
    direction: ConnectionOrigin,
    state: &'static str,
) -> Histogram {
    APTOS_NETWORK_CONNECTION_UPGRADE_TIME.with_label_values(&[
        network_context.role().as_str(),
        network_context.network_id().as_str(),
        network_context.peer_id().short_str().as_str(),
        direction.as_str(),
        state,
    ])
}

pub static APTOS_NETWORK_DISCOVERY_NOTES: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "aptos_network_discovery_notes",
        "Aptos network discovery notes",
        &["role_type"]
    )
    .unwrap()
});

pub static APTOS_NETWORK_RPC_MESSAGES: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_network_rpc_messages",
        "Number of RPC messages",
        &["role_type", "network_id", "peer_id", "type", "state"]
    )
    .unwrap()
});

pub fn rpc_messages(
    network_context: &NetworkContext,
    type_label: &'static str,
    state_label: &'static str,
) -> IntCounter {
    APTOS_NETWORK_RPC_MESSAGES.with_label_values(&[
        network_context.role().as_str(),
        network_context.network_id().as_str(),
        network_context.peer_id().short_str().as_str(),
        type_label,
        state_label,
    ])
}

pub static APTOS_NETWORK_RPC_BYTES: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_network_rpc_bytes",
        "Number of RPC bytes transferred",
        &["role_type", "network_id", "peer_id", "type", "state"]
    )
    .unwrap()
});

pub fn rpc_bytes(
    network_context: &NetworkContext,
    type_label: &'static str,
    state_label: &'static str,
) -> IntCounter {
    APTOS_NETWORK_RPC_BYTES.with_label_values(&[
        network_context.role().as_str(),
        network_context.network_id().as_str(),
        network_context.peer_id().short_str().as_str(),
        type_label,
        state_label,
    ])
}

pub static INVALID_NETWORK_MESSAGES: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_network_invalid_messages",
        "Number of invalid messages (RPC/direct_send)",
        &["role_type", "network_id", "peer_id", "type"]
    )
    .unwrap()
});

pub static PEER_SEND_FAILURES: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_network_peer_send_failures",
        "Number of messages failed to send to peer",
        &["role_type", "network_id", "peer_id", "protocol_id"]
    )
    .unwrap()
});

pub static APTOS_NETWORK_OUTBOUND_RPC_REQUEST_LATENCY: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_network_outbound_rpc_request_latency_seconds",
        "Outbound RPC request latency in seconds",
        &["role_type", "network_id", "peer_id", "protocol_id"]
    )
    .unwrap()
});

pub fn outbound_rpc_request_latency(
    network_context: &NetworkContext,
    protocol_id: ProtocolId,
) -> Histogram {
    APTOS_NETWORK_OUTBOUND_RPC_REQUEST_LATENCY.with_label_values(&[
        network_context.role().as_str(),
        network_context.network_id().as_str(),
        network_context.peer_id().short_str().as_str(),
        protocol_id.as_str(),
    ])
}

pub static APTOS_NETWORK_INBOUND_RPC_HANDLER_LATENCY: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_network_inbound_rpc_handler_latency_seconds",
        "Inbound RPC request application handler latency in seconds",
        &["role_type", "network_id", "peer_id", "protocol_id"]
    )
    .unwrap()
});

pub fn inbound_rpc_handler_latency(
    network_context: &NetworkContext,
    protocol_id: ProtocolId,
) -> Histogram {
    APTOS_NETWORK_INBOUND_RPC_HANDLER_LATENCY.with_label_values(&[
        network_context.role().as_str(),
        network_context.network_id().as_str(),
        network_context.peer_id().short_str().as_str(),
        protocol_id.as_str(),
    ])
}

pub static APTOS_NETWORK_DIRECT_SEND_MESSAGES: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_network_direct_send_messages",
        "Number of direct send messages",
        &["role_type", "network_id", "peer_id", "state"]
    )
    .unwrap()
});

pub fn direct_send_messages(
    network_context: &NetworkContext,
    state_label: &'static str,
) -> IntCounter {
    APTOS_NETWORK_DIRECT_SEND_MESSAGES.with_label_values(&[
        network_context.role().as_str(),
        network_context.network_id().as_str(),
        network_context.peer_id().short_str().as_str(),
        state_label,
    ])
}

pub static APTOS_NETWORK_DIRECT_SEND_BYTES: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_network_direct_send_bytes",
        "Number of direct send bytes transferred",
        &["role_type", "network_id", "peer_id", "state"]
    )
    .unwrap()
});

pub fn direct_send_bytes(
    network_context: &NetworkContext,
    state_label: &'static str,
) -> IntCounter {
    APTOS_NETWORK_DIRECT_SEND_BYTES.with_label_values(&[
        network_context.role().as_str(),
        network_context.network_id().as_str(),
        network_context.peer_id().short_str().as_str(),
        state_label,
    ])
}

/// Counters(queued,dequeued,dropped) related to inbound network notifications for RPCs and
/// DirectSends.
pub static PENDING_NETWORK_NOTIFICATIONS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_network_pending_network_notifications",
        "Number of pending inbound network notifications by state",
        &["state"]
    )
    .unwrap()
});

/// Counter of pending requests in Network Provider
pub static PENDING_NETWORK_REQUESTS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_network_pending_requests",
        "Number of pending outbound network requests by state",
        &["state"]
    )
    .unwrap()
});

/// Counter of pending network events to Health Checker.
pub static PENDING_HEALTH_CHECKER_NETWORK_EVENTS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_network_pending_health_check_events",
        "Number of pending health check events by state",
        &["state"]
    )
    .unwrap()
});

/// Counter of pending network events to Discovery.
pub static PENDING_DISCOVERY_NETWORK_EVENTS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_network_pending_discovery_events",
        "Number of pending discovery events by state",
        &["state"]
    )
    .unwrap()
});

/// Counter of pending requests in Peer Manager
pub static PENDING_PEER_MANAGER_REQUESTS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_network_pending_peer_manager_requests",
        "Number of pending peer manager requests by state",
        &["state"]
    )
    .unwrap()
});

///
/// Channel Counters
///

/// Counter of pending requests in Connectivity Manager
pub static PENDING_CONNECTIVITY_MANAGER_REQUESTS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_network_pending_connectivity_manager_requests",
        "Number of pending connectivity manager requests"
    )
    .unwrap()
});

/// Counter of pending Connection Handler notifications to PeerManager.
pub static PENDING_CONNECTION_HANDLER_NOTIFICATIONS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_network_pending_connection_handler_notifications",
        "Number of pending connection handler notifications"
    )
    .unwrap()
});

/// Counter of pending dial requests in Peer Manager
pub static PENDING_PEER_MANAGER_DIAL_REQUESTS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_network_pending_peer_manager_dial_requests",
        "Number of pending peer manager dial requests"
    )
    .unwrap()
});

/// Counter of messages pending in queue to be sent out on the wire.
pub static PENDING_WIRE_MESSAGES: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_network_pending_wire_messages",
        "Number of pending wire messages"
    )
    .unwrap()
});

/// Counter of pending requests in Direct Send
pub static PENDING_DIRECT_SEND_REQUESTS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_network_pending_direct_send_requests",
        "Number of pending direct send requests"
    )
    .unwrap()
});

/// Counter of pending Direct Send notifications to Network Provider
pub static PENDING_DIRECT_SEND_NOTIFICATIONS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_network_pending_direct_send_notifications",
        "Number of pending direct send notifications"
    )
    .unwrap()
});

/// Counter of pending requests in RPC
pub static PENDING_RPC_REQUESTS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_network_pending_rpc_requests",
        "Number of pending rpc requests"
    )
    .unwrap()
});

/// Counter of pending RPC notifications to Network Provider
pub static PENDING_RPC_NOTIFICATIONS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_network_pending_rpc_notifications",
        "Number of pending rpc notifications"
    )
    .unwrap()
});

/// Counter of pending requests for each remote peer
pub static PENDING_PEER_REQUESTS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_network_pending_peer_requests",
        "Number of pending peer requests"
    )
    .unwrap()
});

/// Counter of pending RPC events from Peer to Rpc actor.
pub static PENDING_PEER_RPC_NOTIFICATIONS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_network_pending_peer_rpc_notifications",
        "Number of pending peer rpc notifications"
    )
    .unwrap()
});

/// Counter of pending DirectSend events from Peer to DirectSend actor..
pub static PENDING_PEER_DIRECT_SEND_NOTIFICATIONS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_network_pending_peer_direct_send_notifications",
        "Number of pending peer direct send notifications"
    )
    .unwrap()
});

/// Counter of pending connection notifications from Peer to NetworkProvider.
pub static PENDING_PEER_NETWORK_NOTIFICATIONS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_network_pending_peer_network_notifications",
        "Number of pending peer network notifications"
    )
    .unwrap()
});

pub static NETWORK_RATE_LIMIT_METRICS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_network_rate_limit",
        "Network Rate Limiting Metrics",
        &["direction", "metric"]
    )
    .unwrap()
});

pub static NETWORK_APPLICATION_INBOUND_METRIC: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_network_app_inbound_traffic",
        "Network Inbound Traffic by application",
        &[
            "role_type",
            "network_id",
            "peer_id",
            "protocol_id",
            "metric"
        ]
    )
    .unwrap()
});

pub fn network_application_inbound_traffic(
    network_context: NetworkContext,
    protocol_id: ProtocolId,
    data: Vec<u8>,
    size: u64,
) {
    analyze_compression(data);

    NETWORK_APPLICATION_INBOUND_METRIC
        .with_label_values(&[
            network_context.role().as_str(),
            network_context.network_id().as_str(),
            network_context.peer_id().short_str().as_str(),
            protocol_id.as_str(),
            "size",
        ])
        .observe(size as f64);
}

/// Compresses and decompresses data using different libraries
/// to identify operational time and data size reduction.
fn analyze_compression(data: Vec<u8>) {
    // This is a cheap and dirty hack for emulating larger data sizes
    let data_amplifier = 1;
    let mut amplified_data = data.clone();
    for _ in 0..data_amplifier - 1 {
        amplified_data.append(&mut data.clone());
    }
    let data = amplified_data;

    let raw_data_length = data.len();
    println!("RAW DATA LENGTH: {:?}", raw_data_length);

    // Analyze minize oxide
    for compression_level in 1..10 {
        let (compress_time, decompress_time, relative_size) =
            analyze_miniz_oxide_compression(data.clone(), compression_level);
        println!("{:?} >>>>> Minize oxide level: {:?}, Compress time: {:?}, Decompress time: {:?}, Resulting data size (%): {:?}", raw_data_length, compression_level, compress_time, decompress_time, relative_size);
    }

    // Analyze zlib
    for compression_level in 1..9 {
        let (compress_time, decompress_time, relative_size) =
            analyze_flate_2_zlib(data.clone(), compression_level);
        println!(
            "{:?} >>>>> Flate 2 zlib level: {:?}, Compress time: {:?}, Decompress time: {:?}, Resulting data size (%): {:?}",
            raw_data_length, compression_level, compress_time, decompress_time, relative_size
        );
    }

    // Analyze deflate
    for compression_level in 1..9 {
        let (compress_time, decompress_time, relative_size) =
            analyze_flate_2_deflate(data.clone(), compression_level);
        println!(
            "{:?} >>>>> Flate 2 deflate level: {:?}, Compress time: {:?}, Decompress time: {:?}, Resulting data size (%): {:?}",
            raw_data_length, compression_level, compress_time, decompress_time, relative_size
        );
    }

    // Analyze gz
    for compression_level in 1..9 {
        let (compress_time, decompress_time, relative_size) =
            analyze_flate_2_gz(data.clone(), compression_level);
        println!(
            "{:?} >>>>> Flate 2 gz level: {:?}, Compress time: {:?}, Decompress time: {:?}, Resulting data size (%) : {:?}",
            raw_data_length, compression_level, compress_time, decompress_time, relative_size
        );
    }

    // Analyze snappy
    let (compress_time, decompress_time, relative_size) = analyze_snappy(data);
    println!(
        "{:?} >>>>> Snappy: Compress time: {:?}, Decompress time: {:?}, Resulting data size (%): {:?}",
        raw_data_length, compress_time, decompress_time, relative_size
    );
}

fn analyze_miniz_oxide_compression(
    raw_data: Vec<u8>,
    compression_level: u8,
) -> (Duration, Duration, f64) {
    let raw_data_length = raw_data.len();

    // Compress the input
    let compress_start = Instant::now();
    let compressed = compress_to_vec(&raw_data, compression_level);
    let compress_duration = compress_start.elapsed();
    let compressed_data_length = compressed.len();

    // println!("COMPRESSED DATA LENGTH: {:?}", compressed.len());

    // Decompress the compressed input
    let decompress_start = Instant::now();
    let decompressed = decompress_to_vec(compressed.as_slice()).expect("Failed to decompress!");
    let decompress_duration = decompress_start.elapsed();
    let relative_size = (compressed_data_length as f64 / raw_data_length as f64) * 100.0;

    // Verify operations
    assert_eq!(raw_data, decompressed);

    (compress_duration, decompress_duration, relative_size)
}

fn analyze_flate_2_zlib(raw_data: Vec<u8>, compression_level: u32) -> (Duration, Duration, f64) {
    let raw_data_length = raw_data.len();

    // Compress the input
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::new(compression_level));
    let compress_start = Instant::now();
    encoder.write_all(&raw_data).unwrap();
    let compressed = encoder.finish().unwrap();
    let compress_duration = compress_start.elapsed();
    let compressed_data_length = compressed.len();

    // println!("COMPRESSED DATA LENGTH: {:?}", compressed.len());

    // Decompress the compressed input
    let mut decompressed = Vec::new();
    let mut decoder = ZlibDecoder::new(decompressed);
    let decompress_start = Instant::now();
    decoder.write_all(&compressed).unwrap();
    decompressed = decoder.finish().unwrap();
    let decompress_duration = decompress_start.elapsed();
    let relative_size = (compressed_data_length as f64 / raw_data_length as f64) * 100.0;

    // Verify operations
    assert_eq!(raw_data, decompressed);

    (compress_duration, decompress_duration, relative_size)
}

fn analyze_flate_2_deflate(raw_data: Vec<u8>, compression_level: u32) -> (Duration, Duration, f64) {
    let raw_data_length = raw_data.len();

    // Compress the input
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::new(compression_level));
    let compress_start = Instant::now();
    encoder.write_all(&raw_data).unwrap();
    let compressed = encoder.finish().unwrap();
    let compress_duration = compress_start.elapsed();
    let compressed_data_length = compressed.len();

    // println!("COMPRESSED DATA LENGTH: {:?}", compressed.len());

    // Decompress the compressed input
    let mut decompressed = Vec::new();
    let mut decoder = DeflateDecoder::new(decompressed);
    let decompress_start = Instant::now();
    decoder.write_all(&compressed).unwrap();
    decompressed = decoder.finish().unwrap();
    let decompress_duration = decompress_start.elapsed();
    let relative_size = (compressed_data_length as f64 / raw_data_length as f64) * 100.0;

    // Verify operations
    assert_eq!(raw_data, decompressed);

    (compress_duration, decompress_duration, relative_size)
}

fn analyze_flate_2_gz(raw_data: Vec<u8>, compression_level: u32) -> (Duration, Duration, f64) {
    let raw_data_length = raw_data.len();

    // Compress the input
    let mut encoder = GzEncoder::new(Vec::new(), Compression::new(compression_level));
    let compress_start = Instant::now();
    encoder.write_all(&raw_data).unwrap();
    let compressed = encoder.finish().unwrap();
    let compress_duration = compress_start.elapsed();
    let compressed_data_length = compressed.len();

    // println!("COMPRESSED DATA LENGTH: {:?}", compressed.len());

    // Decompress the compressed input
    let mut decompressed = Vec::new();
    let mut decoder = GzDecoder::new(decompressed);
    let decompress_start = Instant::now();
    decoder.write_all(&compressed).unwrap();
    decompressed = decoder.finish().unwrap();
    let decompress_duration = decompress_start.elapsed();
    let relative_size = (compressed_data_length as f64 / raw_data_length as f64) * 100.0;

    // Verify operations
    assert_eq!(raw_data, decompressed);

    (compress_duration, decompress_duration, relative_size)
}

fn analyze_snappy(raw_data: Vec<u8>) -> (Duration, Duration, f64) {
    let raw_data_length = raw_data.len();

    // Compress the input
    let mut encoder = write::FrameEncoder::new(vec![]);
    let compress_start = Instant::now();
    encoder.write_all(&raw_data).unwrap();
    let compressed = encoder.into_inner().unwrap();
    let compress_duration = compress_start.elapsed();
    let compressed_data_length = compressed.len();

    // println!("COMPRESSED DATA LENGTH: {:?}", compressed.len());

    // Decompress the compressed input
    let mut decompressed = vec![];
    let mut decoder = read::FrameDecoder::new(compressed.as_slice());
    let decompress_start = Instant::now();
    decoder.read_to_end(&mut decompressed).unwrap();
    let decompress_duration = decompress_start.elapsed();
    let relative_size = (compressed_data_length as f64 / raw_data_length as f64) * 100.0;

    // Verify operations
    assert_eq!(raw_data, decompressed);

    (compress_duration, decompress_duration, relative_size)
}

pub static NETWORK_APPLICATION_OUTBOUND_METRIC: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_network_app_outbound_traffic",
        "Network Outbound Traffic by application",
        &[
            "role_type",
            "network_id",
            "peer_id",
            "protocol_id",
            "metric"
        ]
    )
    .unwrap()
});

pub fn network_application_outbound_traffic(
    network_context: NetworkContext,
    protocol_id: ProtocolId,
    size: u64,
) {
    NETWORK_APPLICATION_OUTBOUND_METRIC
        .with_label_values(&[
            network_context.role().as_str(),
            network_context.network_id().as_str(),
            network_context.peer_id().short_str().as_str(),
            protocol_id.as_str(),
            "size",
        ])
        .observe(size as f64);
}
