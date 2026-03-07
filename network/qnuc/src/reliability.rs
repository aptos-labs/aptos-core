// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Reliability layer: ACK tracking, retransmission, and congestion control.
//!
//! Implements a simplified congestion controller inspired by QUIC's NewReno/Cubic approach:
//! - Slow start until threshold
//! - Congestion avoidance (additive increase)
//! - Multiplicative decrease on loss

use crate::packet::SelectiveAck;
use std::{
    collections::{BTreeMap, HashSet},
    time::{Duration, Instant},
};

/// Configuration for the reliability layer.
#[derive(Debug, Clone)]
pub struct ReliabilityConfig {
    pub initial_rto: Duration,
    pub min_rto: Duration,
    pub max_rto: Duration,
    pub max_retransmissions: u32,
    pub ack_delay: Duration,
    pub initial_window: u64,
    pub min_window: u64,
    pub max_window: u64,
}

impl Default for ReliabilityConfig {
    fn default() -> Self {
        Self {
            initial_rto: Duration::from_millis(200),
            min_rto: Duration::from_millis(50),
            max_rto: Duration::from_secs(10),
            max_retransmissions: 10,
            ack_delay: Duration::from_millis(25),
            initial_window: 32,
            min_window: 4,
            max_window: 1024,
        }
    }
}

/// Metadata for an unacknowledged sent packet.
#[derive(Debug, Clone)]
pub struct SentPacketInfo {
    pub sequence: u64,
    pub send_time: Instant,
    pub retransmit_count: u32,
    pub size: usize,
    pub data: Vec<u8>,
}

/// Tracks RTT using exponentially weighted moving average.
#[derive(Debug, Clone)]
pub struct RttEstimator {
    smoothed_rtt: Duration,
    rtt_variance: Duration,
    min_rtt: Duration,
    latest_rtt: Duration,
}

impl RttEstimator {
    pub fn new(initial_rtt: Duration) -> Self {
        Self {
            smoothed_rtt: initial_rtt,
            rtt_variance: initial_rtt / 2,
            min_rtt: initial_rtt,
            latest_rtt: initial_rtt,
        }
    }

    pub fn update(&mut self, rtt_sample: Duration) {
        self.latest_rtt = rtt_sample;
        if rtt_sample < self.min_rtt {
            self.min_rtt = rtt_sample;
        }

        // EWMA: srtt = 7/8 * srtt + 1/8 * sample
        let diff = self.smoothed_rtt.abs_diff(rtt_sample);
        self.rtt_variance = (self.rtt_variance * 3 + diff) / 4;
        self.smoothed_rtt = (self.smoothed_rtt * 7 + rtt_sample) / 8;
    }

    pub fn rto(&self) -> Duration {
        self.smoothed_rtt + std::cmp::max(Duration::from_millis(1), self.rtt_variance * 4)
    }

    pub fn smoothed_rtt(&self) -> Duration {
        self.smoothed_rtt
    }

    pub fn min_rtt(&self) -> Duration {
        self.min_rtt
    }
}

/// Congestion controller using a simplified NewReno algorithm.
#[derive(Debug, Clone)]
pub struct CongestionController {
    /// Current congestion window (in packets).
    pub window: u64,
    /// Slow start threshold.
    pub ssthresh: u64,
    /// Number of packets acknowledged in the current RTT (for additive increase).
    acked_in_round: u64,
    config: ReliabilityConfig,
}

impl CongestionController {
    pub fn new(config: ReliabilityConfig) -> Self {
        let window = config.initial_window;
        let max_window = config.max_window;
        Self {
            window,
            ssthresh: max_window / 2,
            acked_in_round: 0,
            config,
        }
    }

    /// Called when a packet is acknowledged successfully.
    pub fn on_ack(&mut self) {
        if self.window < self.ssthresh {
            // Slow start: increase by 1 for each ACK (exponential growth)
            self.window = std::cmp::min(self.window + 1, self.config.max_window);
        } else {
            // Congestion avoidance: increase by 1/window for each ACK (additive increase)
            self.acked_in_round += 1;
            if self.acked_in_round >= self.window {
                self.window = std::cmp::min(self.window + 1, self.config.max_window);
                self.acked_in_round = 0;
            }
        }
    }

    /// Called when a packet loss is detected.
    pub fn on_loss(&mut self) {
        // Multiplicative decrease
        self.ssthresh = std::cmp::max(self.window / 2, self.config.min_window);
        self.window = self.ssthresh;
        self.acked_in_round = 0;
    }

    pub fn can_send(&self, in_flight: u64) -> bool {
        in_flight < self.window
    }
}

/// The send-side reliability tracker.
pub struct SendTracker {
    /// Next sequence number to assign.
    next_seq: u64,
    /// Map of unacknowledged packets indexed by sequence number.
    unacked: BTreeMap<u64, SentPacketInfo>,
    /// RTT estimator.
    rtt: RttEstimator,
    /// Congestion controller.
    pub congestion: CongestionController,
    /// Configuration.
    config: ReliabilityConfig,
}

impl SendTracker {
    pub fn new(config: ReliabilityConfig) -> Self {
        let rtt = RttEstimator::new(config.initial_rto);
        let congestion = CongestionController::new(config.clone());
        Self {
            next_seq: 0,
            unacked: BTreeMap::new(),
            rtt,
            congestion,
            config,
        }
    }

    /// Allocate and record a new sequence number for a sent packet.
    pub fn register_sent(&mut self, data: Vec<u8>) -> u64 {
        let seq = self.next_seq;
        self.next_seq += 1;
        let info = SentPacketInfo {
            sequence: seq,
            send_time: Instant::now(),
            retransmit_count: 0,
            size: data.len(),
            data,
        };
        self.unacked.insert(seq, info);
        seq
    }

    pub fn next_seq(&self) -> u64 {
        self.next_seq
    }

    pub fn in_flight(&self) -> u64 {
        self.unacked.len() as u64
    }

    /// Process an incoming ACK and return RTT measurement if available.
    pub fn process_ack(&mut self, sack: &SelectiveAck) -> Vec<u64> {
        let mut newly_acked = Vec::new();

        // Process cumulative ACK
        let cumulative = sack.cumulative_ack;
        let to_remove: Vec<u64> = self
            .unacked
            .range(..=cumulative)
            .map(|(&k, _)| k)
            .collect();
        for seq in to_remove {
            if let Some(info) = self.unacked.remove(&seq) {
                if info.retransmit_count == 0 {
                    let rtt_sample = info.send_time.elapsed();
                    self.rtt.update(rtt_sample);
                }
                self.congestion.on_ack();
                newly_acked.push(seq);
            }
        }

        // Process selective ACKs
        for &seq in &sack.selective_acks {
            if let Some(info) = self.unacked.remove(&seq) {
                if info.retransmit_count == 0 {
                    let rtt_sample = info.send_time.elapsed();
                    self.rtt.update(rtt_sample);
                }
                self.congestion.on_ack();
                newly_acked.push(seq);
            }
        }

        newly_acked
    }

    /// Return packets that need retransmission (exceeded RTO).
    pub fn get_retransmissions(&mut self) -> Vec<(u64, Vec<u8>)> {
        let rto = std::cmp::min(
            std::cmp::max(self.rtt.rto(), self.config.min_rto),
            self.config.max_rto,
        );
        let now = Instant::now();
        let mut retransmits = Vec::new();

        for (_, info) in self.unacked.iter_mut() {
            if now.duration_since(info.send_time) > rto {
                if info.retransmit_count >= self.config.max_retransmissions {
                    continue;
                }
                info.retransmit_count += 1;
                info.send_time = now;
                retransmits.push((info.sequence, info.data.clone()));
                self.congestion.on_loss();
            }
        }

        retransmits
    }

    /// Check if any packet has exceeded max retransmissions.
    pub fn has_failed_packets(&self) -> bool {
        self.unacked
            .values()
            .any(|info| info.retransmit_count >= self.config.max_retransmissions)
    }

    pub fn rtt_estimate(&self) -> Duration {
        self.rtt.smoothed_rtt()
    }
}

/// The receive-side reliability tracker with reorder buffer.
pub struct RecvTracker {
    /// The next expected in-order sequence number.
    next_expected: u64,
    /// Out-of-order packets received but not yet delivered.
    reorder_buffer: BTreeMap<u64, Vec<u8>>,
    /// Set of all received sequence numbers (for selective ACK generation).
    received: HashSet<u64>,
}

impl Default for RecvTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl RecvTracker {
    pub fn new() -> Self {
        Self {
            next_expected: 0,
            reorder_buffer: BTreeMap::new(),
            received: HashSet::new(),
        }
    }

    /// Record a received packet. Returns data ready for in-order delivery.
    pub fn receive(&mut self, seq: u64, data: Vec<u8>) -> Vec<(u64, Vec<u8>)> {
        // Duplicate or already-delivered
        if seq < self.next_expected || self.received.contains(&seq) {
            return Vec::new();
        }

        self.received.insert(seq);
        self.reorder_buffer.insert(seq, data);

        // Drain contiguous packets from the reorder buffer
        let mut delivered = Vec::new();
        while let Some(data) = self.reorder_buffer.remove(&self.next_expected) {
            delivered.push((self.next_expected, data));
            self.next_expected += 1;
        }

        delivered
    }

    /// Generate a selective ACK for the current receive state.
    pub fn generate_ack(&self) -> SelectiveAck {
        let cumulative = if self.next_expected > 0 {
            self.next_expected - 1
        } else {
            0
        };

        let selective: Vec<u64> = self
            .reorder_buffer
            .keys()
            .copied()
            .collect();

        SelectiveAck::new(cumulative, selective)
    }

    pub fn next_expected(&self) -> u64 {
        self.next_expected
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rtt_estimator() {
        let mut rtt = RttEstimator::new(Duration::from_millis(100));
        rtt.update(Duration::from_millis(80));
        assert!(rtt.smoothed_rtt() < Duration::from_millis(100));
        assert!(rtt.min_rtt() == Duration::from_millis(80));
    }

    #[test]
    fn test_congestion_slow_start() {
        let config = ReliabilityConfig {
            initial_window: 4,
            max_window: 64,
            ..Default::default()
        };
        let mut cc = CongestionController::new(config);
        assert_eq!(cc.window, 4);
        cc.on_ack();
        assert_eq!(cc.window, 5);
        cc.on_ack();
        assert_eq!(cc.window, 6);
    }

    #[test]
    fn test_congestion_loss() {
        let config = ReliabilityConfig {
            initial_window: 16,
            min_window: 4,
            max_window: 64,
            ..Default::default()
        };
        let mut cc = CongestionController::new(config);
        cc.on_loss();
        assert_eq!(cc.window, 8);
        assert_eq!(cc.ssthresh, 8);
    }

    #[test]
    fn test_send_tracker_register_and_ack() {
        let mut tracker = SendTracker::new(ReliabilityConfig::default());
        let seq0 = tracker.register_sent(b"hello".to_vec());
        let seq1 = tracker.register_sent(b"world".to_vec());
        assert_eq!(seq0, 0);
        assert_eq!(seq1, 1);
        assert_eq!(tracker.in_flight(), 2);

        let sack = SelectiveAck::new(1, vec![]);
        let acked = tracker.process_ack(&sack);
        assert_eq!(acked.len(), 2);
        assert_eq!(tracker.in_flight(), 0);
    }

    #[test]
    fn test_recv_tracker_in_order() {
        let mut tracker = RecvTracker::new();
        let delivered = tracker.receive(0, b"a".to_vec());
        assert_eq!(delivered.len(), 1);
        let delivered = tracker.receive(1, b"b".to_vec());
        assert_eq!(delivered.len(), 1);
        assert_eq!(tracker.next_expected(), 2);
    }

    #[test]
    fn test_recv_tracker_reorder() {
        let mut tracker = RecvTracker::new();

        // Receive packet 2 first (out of order)
        let delivered = tracker.receive(2, b"c".to_vec());
        assert!(delivered.is_empty());

        // Receive packet 0
        let delivered = tracker.receive(0, b"a".to_vec());
        assert_eq!(delivered.len(), 1);

        // Receive packet 1 -> should deliver 1 and 2
        let delivered = tracker.receive(1, b"b".to_vec());
        assert_eq!(delivered.len(), 2);
        assert_eq!(delivered[0].0, 1);
        assert_eq!(delivered[1].0, 2);
        assert_eq!(tracker.next_expected(), 3);
    }

    #[test]
    fn test_recv_tracker_duplicate() {
        let mut tracker = RecvTracker::new();
        let _ = tracker.receive(0, b"a".to_vec());
        let delivered = tracker.receive(0, b"a".to_vec());
        assert!(delivered.is_empty());
    }

    #[test]
    fn test_selective_ack_generation() {
        let mut tracker = RecvTracker::new();
        let _ = tracker.receive(0, b"a".to_vec());
        let _ = tracker.receive(2, b"c".to_vec());
        let _ = tracker.receive(4, b"e".to_vec());

        let sack = tracker.generate_ack();
        assert_eq!(sack.cumulative_ack, 0);
        assert!(sack.selective_acks.contains(&2));
        assert!(sack.selective_acks.contains(&4));
    }
}
