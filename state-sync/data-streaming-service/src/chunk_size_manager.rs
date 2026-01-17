// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::metrics;
use aptos_infallible::Mutex;
use aptos_logger::info;
use std::{collections::VecDeque, sync::Arc};

/// Tracks truncation history for a specific chunk type
#[derive(Clone, Debug)]
pub struct ChunkTruncationTracker {
    /// Ring buffer of truncation events (true = truncated, false = not truncated)
    truncation_history: VecDeque<bool>,

    /// Maximum size of the history buffer
    max_history_size: usize,

    /// The current adjusted chunk size
    current_chunk_size: u64,

    /// The maximum chunk size (from config)
    max_chunk_size: u64,

    /// The minimum chunk size (should never go below this)
    min_chunk_size: u64,

    /// The chunk type label for metrics
    chunk_type_label: &'static str,

    /// Counter for recordings since last adjustment
    recordings_since_last_adjustment: usize,

    /// The truncation percentage threshold (0.0-1.0) that triggers chunk size reduction
    truncation_percentage_threshold: f64,

    /// The percentage at which to reduce chunk sizes (0.0-1.0)
    chunk_size_reduction_percentage: f64,

    /// The percentage at which to increase chunk sizes (0.0-1.0)
    chunk_size_increase_percentage: f64,
}

impl ChunkTruncationTracker {
    /// Creates a new chunk truncation tracker with the specified parameters
    pub fn new(
        max_chunk_size: u64,
        chunk_type_label: &'static str,
        truncation_percentage_threshold: f64,
        chunk_size_reduction_percentage: f64,
        chunk_size_increase_percentage: f64,
        chunk_history_size: usize,
    ) -> Self {
        // Initialize metrics
        metrics::set_gauge(
            &metrics::ADJUSTED_CHUNK_SIZE,
            chunk_type_label,
            max_chunk_size,
        );
        metrics::set_gauge(&metrics::TRUNCATION_RATE, chunk_type_label, 0);

        Self {
            truncation_history: VecDeque::with_capacity(chunk_history_size),
            max_history_size: chunk_history_size,
            current_chunk_size: max_chunk_size,
            max_chunk_size,
            min_chunk_size: max_chunk_size / 16, // Never go below 1/16th of max
            chunk_type_label,
            recordings_since_last_adjustment: 0,
            truncation_percentage_threshold,
            chunk_size_reduction_percentage,
            chunk_size_increase_percentage,
        }
    }

    /// Records a chunk request outcome
    pub fn record_chunk_outcome(&mut self, was_truncated: bool) {
        // Add to history
        if self.truncation_history.len() >= self.max_history_size {
            self.truncation_history.pop_front();
        }
        self.truncation_history.push_back(was_truncated);

        // Update truncation rate metric
        let truncation_rate = self.get_truncation_rate();
        metrics::set_gauge(
            &metrics::TRUNCATION_RATE,
            self.chunk_type_label,
            (truncation_rate * 100.0) as u64,
        );

        // Increment recordings counter
        self.recordings_since_last_adjustment += 1;

        // Adjust chunk size after accumulating max_history_size new recordings
        if self.recordings_since_last_adjustment >= self.max_history_size {
            self.adjust_chunk_size();
            self.recordings_since_last_adjustment = 0;
        }
    }

    /// Adjusts chunk size based on truncation rate
    fn adjust_chunk_size(&mut self) {
        let num_truncated = self.truncation_history.iter().filter(|&&x| x).count();
        let truncation_rate = num_truncated as f64 / self.truncation_history.len() as f64;

        // If truncation rate exceeds threshold, reduce the chunk size
        if truncation_rate > self.truncation_percentage_threshold {
            let new_size = ((self.current_chunk_size as f64
                * (1.0 - self.chunk_size_reduction_percentage)) as u64)
                .max(self.min_chunk_size);
            if new_size != self.current_chunk_size {
                info!(
                    "Reducing chunk size from {} to {} due to {:.1}% truncation rate (threshold: {:.1}%)",
                    self.current_chunk_size,
                    new_size,
                    truncation_rate * 100.0,
                    self.truncation_percentage_threshold * 100.0
                );
                self.current_chunk_size = new_size;

                // Update metrics
                metrics::set_gauge(
                    &metrics::ADJUSTED_CHUNK_SIZE,
                    self.chunk_type_label,
                    new_size,
                );
                metrics::CHUNK_SIZE_ADJUSTMENTS
                    .with_label_values(&[self.chunk_type_label, "decrease"])
                    .inc();
            }
        }
        // If 100% success (0% truncation), gradually increase back to max
        else if truncation_rate == 0.0 && self.current_chunk_size < self.max_chunk_size {
            let new_size = ((self.current_chunk_size as f64
                * (1.0 + self.chunk_size_increase_percentage)) as u64)
                .min(self.max_chunk_size);
            if new_size != self.current_chunk_size {
                info!(
                    "Increasing chunk size from {} to {} due to 100% success rate",
                    self.current_chunk_size, new_size
                );
                self.current_chunk_size = new_size;

                // Update metrics
                metrics::set_gauge(
                    &metrics::ADJUSTED_CHUNK_SIZE,
                    self.chunk_type_label,
                    new_size,
                );
                metrics::CHUNK_SIZE_ADJUSTMENTS
                    .with_label_values(&[self.chunk_type_label, "increase"])
                    .inc();
            }
        }
    }

    /// Gets the current adjusted chunk size
    pub fn get_current_chunk_size(&self) -> u64 {
        self.current_chunk_size
    }

    /// Gets the current truncation rate
    pub fn get_truncation_rate(&self) -> f64 {
        if self.truncation_history.is_empty() {
            0.0
        } else {
            let num_truncated = self.truncation_history.iter().filter(|&&x| x).count();
            num_truncated as f64 / self.truncation_history.len() as f64
        }
    }
}

/// Manages dynamic chunk sizing for all chunk types
#[derive(Clone, Debug)]
pub struct ChunkSizeManager {
    epoch_tracker: Arc<Mutex<ChunkTruncationTracker>>,
    state_tracker: Arc<Mutex<ChunkTruncationTracker>>,
    transaction_tracker: Arc<Mutex<ChunkTruncationTracker>>,
    transaction_output_tracker: Arc<Mutex<ChunkTruncationTracker>>,
}

impl ChunkSizeManager {
    /// Creates a new chunk size manager with the specified parameters
    pub fn new(
        max_epoch_chunk_size: u64,
        max_state_chunk_size: u64,
        max_transaction_chunk_size: u64,
        max_transaction_output_chunk_size: u64,
        truncation_percentage_threshold: f64,
        chunk_size_reduction_percentage: f64,
        chunk_size_increase_percentage: f64,
        chunk_history_size: usize,
    ) -> Self {
        Self {
            epoch_tracker: Arc::new(Mutex::new(ChunkTruncationTracker::new(
                max_epoch_chunk_size,
                "epoch",
                truncation_percentage_threshold,
                chunk_size_reduction_percentage,
                chunk_size_increase_percentage,
                chunk_history_size,
            ))),
            state_tracker: Arc::new(Mutex::new(ChunkTruncationTracker::new(
                max_state_chunk_size,
                "state",
                truncation_percentage_threshold,
                chunk_size_reduction_percentage,
                chunk_size_increase_percentage,
                chunk_history_size,
            ))),
            transaction_tracker: Arc::new(Mutex::new(ChunkTruncationTracker::new(
                max_transaction_chunk_size,
                "transaction",
                truncation_percentage_threshold,
                chunk_size_reduction_percentage,
                chunk_size_increase_percentage,
                chunk_history_size,
            ))),
            transaction_output_tracker: Arc::new(Mutex::new(ChunkTruncationTracker::new(
                max_transaction_output_chunk_size,
                "transaction_output",
                truncation_percentage_threshold,
                chunk_size_reduction_percentage,
                chunk_size_increase_percentage,
                chunk_history_size,
            ))),
        }
    }

    /// Record epoch chunk outcome
    pub fn record_epoch_chunk(&self, was_truncated: bool) {
        self.epoch_tracker
            .lock()
            .record_chunk_outcome(was_truncated);
    }

    /// Record state chunk outcome
    pub fn record_state_chunk(&self, was_truncated: bool) {
        self.state_tracker
            .lock()
            .record_chunk_outcome(was_truncated);
    }

    /// Record transaction chunk outcome
    pub fn record_transaction_chunk(&self, was_truncated: bool) {
        self.transaction_tracker
            .lock()
            .record_chunk_outcome(was_truncated);
    }

    /// Record transaction output chunk outcome
    pub fn record_transaction_output_chunk(&self, was_truncated: bool) {
        self.transaction_output_tracker
            .lock()
            .record_chunk_outcome(was_truncated);
    }

    /// Get current adjusted chunk sizes
    pub fn get_adjusted_chunk_sizes(&self) -> (u64, u64, u64, u64) {
        (
            self.epoch_tracker.lock().get_current_chunk_size(),
            self.state_tracker.lock().get_current_chunk_size(),
            self.transaction_tracker.lock().get_current_chunk_size(),
            self.transaction_output_tracker
                .lock()
                .get_current_chunk_size(),
        )
    }

    /// Get current truncation rates for observability
    pub fn get_truncation_rates(&self) -> (f64, f64, f64, f64) {
        (
            self.epoch_tracker.lock().get_truncation_rate(),
            self.state_tracker.lock().get_truncation_rate(),
            self.transaction_tracker.lock().get_truncation_rate(),
            self.transaction_output_tracker.lock().get_truncation_rate(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_truncation_tracker_halves_on_high_truncation() {
        let mut tracker = ChunkTruncationTracker::new(
            1000,   // max_chunk_size
            "test", // chunk_type_label
            0.10,   // truncation_percentage_threshold (10%)
            0.5,    // chunk_size_reduction_percentage (halve)
            0.25,   // chunk_size_increase_percentage (25%)
            100,    // chunk_history_size
        );

        // Record 89 successes and 11 truncations (11% truncation rate)
        for _ in 0..89 {
            tracker.record_chunk_outcome(false);
        }
        for _ in 0..11 {
            tracker.record_chunk_outcome(true);
        }

        // Chunk size should be halved
        assert_eq!(tracker.get_current_chunk_size(), 500);
    }

    #[test]
    fn test_chunk_truncation_tracker_increases_on_success() {
        let mut tracker = ChunkTruncationTracker::new(
            1000,   // max_chunk_size
            "test", // chunk_type_label
            0.10,   // truncation_percentage_threshold (10%)
            0.5,    // chunk_size_reduction_percentage (halve)
            0.25,   // chunk_size_increase_percentage (25%)
            100,    // chunk_history_size
        );

        // First, reduce the chunk size by recording truncations
        for _ in 0..89 {
            tracker.record_chunk_outcome(false);
        }
        for _ in 0..11 {
            tracker.record_chunk_outcome(true);
        }
        assert_eq!(tracker.get_current_chunk_size(), 500);

        // Now record 100 successes
        for _ in 0..100 {
            tracker.record_chunk_outcome(false);
        }

        // Chunk size should increase by 25% (500 * 1.25 = 625)
        assert_eq!(tracker.get_current_chunk_size(), 625);
    }

    #[test]
    fn test_chunk_truncation_tracker_respects_min_size() {
        let mut tracker = ChunkTruncationTracker::new(
            1000,   // max_chunk_size
            "test", // chunk_type_label
            0.10,   // truncation_percentage_threshold (10%)
            0.5,    // chunk_size_reduction_percentage (halve)
            0.25,   // chunk_size_increase_percentage (25%)
            100,    // chunk_history_size
        );

        // Keep halving until we hit the minimum (1000 / 16 = 62.5, rounds to 62)
        for _ in 0..10 {
            for _ in 0..89 {
                tracker.record_chunk_outcome(false);
            }
            for _ in 0..11 {
                tracker.record_chunk_outcome(true);
            }
        }

        // Should never go below min_chunk_size (1000 / 16 = 62.5, rounds to 62)
        assert!(tracker.get_current_chunk_size() >= 62);
    }

    #[test]
    fn test_chunk_truncation_tracker_respects_max_size() {
        let mut tracker = ChunkTruncationTracker::new(
            1000,   // max_chunk_size
            "test", // chunk_type_label
            0.10,   // truncation_percentage_threshold (10%)
            0.5,    // chunk_size_reduction_percentage (halve)
            0.25,   // chunk_size_increase_percentage (25%)
            100,    // chunk_history_size
        );

        // Record 100 successes - should stay at max
        for _ in 0..100 {
            tracker.record_chunk_outcome(false);
        }

        assert_eq!(tracker.get_current_chunk_size(), 1000);
    }

    #[test]
    fn test_chunk_size_manager_independent_tracking() {
        let manager = ChunkSizeManager::new(
            200,  // max_epoch_chunk_size
            4000, // max_state_chunk_size
            3000, // max_transaction_chunk_size
            3000, // max_transaction_output_chunk_size
            0.10, // truncation_percentage_threshold (10%)
            0.5,  // chunk_size_reduction_percentage (halve)
            0.25, // chunk_size_increase_percentage (25%)
            100,  // chunk_history_size
        );

        // Record truncations for state chunks only
        for _ in 0..100 {
            manager.record_state_chunk(true);
        }

        let (epoch, state, txn, txn_output) = manager.get_adjusted_chunk_sizes();

        // State should be halved, others should remain at max
        assert_eq!(epoch, 200);
        assert!(state < 4000); // Should be reduced
        assert_eq!(txn, 3000);
        assert_eq!(txn_output, 3000);
    }

    #[test]
    fn test_truncation_rate_calculation() {
        let mut tracker = ChunkTruncationTracker::new(
            1000,   // max_chunk_size
            "test", // chunk_type_label
            0.10,   // truncation_percentage_threshold (10%)
            0.5,    // chunk_size_reduction_percentage (halve)
            0.25,   // chunk_size_increase_percentage (25%)
            100,    // chunk_history_size
        );

        // Record 25 truncations and 75 successes
        for _ in 0..25 {
            tracker.record_chunk_outcome(true);
        }
        for _ in 0..75 {
            tracker.record_chunk_outcome(false);
        }

        let rate = tracker.get_truncation_rate();
        assert!((rate - 0.25).abs() < 0.01); // Should be approximately 25%
    }
}
