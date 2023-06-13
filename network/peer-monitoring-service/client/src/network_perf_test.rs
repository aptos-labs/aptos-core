// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// only mod imported if #[cfg(feature = "network-perf-test")]

use crate::logging::{LogEntry, LogEvent, LogSchema};
use std::ops::DerefMut;
use std::sync::Arc;
use std::time::Duration;
use futures::StreamExt;
use rand::Rng;
use rand::rngs::OsRng;
use aptos_config::config::NodeConfig;
use aptos_logger::{info, warn};
use aptos_network::application::interface::NetworkClient;
use aptos_peer_monitoring_service_types::{DirectNetPerformanceMessage, PeerMonitoringServiceMessage, PeerMonitoringSharedState, SendRecord};
use aptos_time_service::{TimeService, TimeServiceTrait};
use crate::network::PeerMonitoringServiceClient;

pub async fn directsend_sentwork_perf_worker(
    node_config: NodeConfig,
    peer_monitoring_client: PeerMonitoringServiceClient<NetworkClient<PeerMonitoringServiceMessage>>,
    time_service: TimeService,
    shared: Arc<std::sync::RwLock<PeerMonitoringSharedState>>,
) {
    let peers_and_metadata = peer_monitoring_client.get_peers_and_metadata();
    let data_size= node_config.peer_monitoring_service.performance_monitoring.direct_send_data_size;
    let interval = Duration::from_micros(node_config.peer_monitoring_service.performance_monitoring.direct_send_interval_usec);
    let ticker = time_service.interval(interval);
    futures::pin_mut!(ticker);

    let mut counter : u64 = 0;

    // random payload filler
    let mut blob = Vec::<u8>::with_capacity(data_size as usize);
    let mut rng = OsRng;
    for _ in 0..data_size {
        blob.push(rng.gen());
    }

    loop {
        ticker.next().await;

        // Get all peers (TODO: limit to 10 times per second? TODO: change API to be generational and a fast NOP when no change)
        let all_peers = match peers_and_metadata.get_all_peers() {
            Ok(all_peers) => all_peers,
            Err(error) => {
                warn!(LogSchema::new(LogEntry::MetadataUpdateLoop)
                        .event(LogEvent::UnexpectedErrorEncountered)
                        .error(&error.into())
                        .message("Failed to get all peers!"));
                continue; // Move to the next loop iteration
            },
        };

        // let now = time_service.now();
        let nowu = time_service.now_unix_time();
        for peer_network_id in all_peers {
            counter += 1;

            {
                // tweak the random payload a little on every send
                let counter_bytes: [u8; 8] = counter.to_le_bytes();
                let (dest, _) = blob.deref_mut().split_at_mut(8);
                dest.copy_from_slice(&counter_bytes);
            }

            let msg = PeerMonitoringServiceMessage::DirectNetPerformance(DirectNetPerformanceMessage{
                request_counter: counter,
                send_micros: nowu.as_micros() as i64,
                data: blob.clone(),
            });
            // TODO: log & count this send?
            {
                shared.write().unwrap().set(SendRecord{
                    request_counter: counter,
                    send_micros: nowu.as_micros() as i64,
                    bytes_sent: blob.len(),
                })
            }
            let result = peer_monitoring_client.send_direct(peer_network_id, msg).await;
            if let Err(err) = result {
                info!("PM direct send err: {}", err);
            }
        }
        info!("PM direct sent counter={}", counter);
    }
}
