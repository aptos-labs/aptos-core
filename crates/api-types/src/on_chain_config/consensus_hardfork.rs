// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Generic consensus-layer hardfork framework.
//!
//! Modeled after reth's `ChainHardforks` + `ForkCondition` pattern.
//! Consensus-layer forks activate based on **epoch** or **timestamp**.
//!
//! # Architecture
//!
//! ```text
//! ┌────────────────────────────┐
//! │  genesis.json              │  Config: extra_fields["consensusAlpha"]
//! ├────────────────────────────┤
//! │  ConsensusHardfork enum    │  Definition: add variants for new forks
//! ├────────────────────────────┤
//! │  ConsensusHardforks        │  Runtime: HashMap<Fork, ForkCondition>
//! │  + is_active_at_epoch()    │  Query API
//! └────────────────────────────┘
//! ```
//!
//! # Usage
//!
//! At node startup, read hardfork activation values from `genesis.json`,
//! build a [`ConsensusHardforks`], and call [`init_consensus_hardforks`] once.
//! Then use [`is_consensus_fork_active_at_epoch`] anywhere in consensus code.
//!
//! ## Reading from genesis.json
//!
//! In `genesis.json`, add fields under `.config`:
//!
//! ```json
//! {
//!   "config": {
//!     "consensusAlpha": 100
//!   }
//! }
//! ```
//!
//! At node startup (gravity-sdk side):
//!
//! ```ignore
//! let hardforks = ConsensusHardforks::from_genesis_extra_fields(|key| {
//!     extra.get(key).and_then(|v| v.as_u64())
//! });
//! init_consensus_hardforks(hardforks);
//! ```

use std::collections::HashMap;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Fork identifiers
// ---------------------------------------------------------------------------

/// Consensus-layer hardfork identifiers.
///
/// To add a new hardfork, simply add a new variant here and register its
/// activation condition from genesis config at startup.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ConsensusHardfork {
    /// Consensus Alpha hardfork.
    /// Activates `epoch_block_info` field in `BlockInfo` BCS serialization.
    /// Before this fork: 7-field format. After: 8-field format.
    ConsensusAlpha,
}

impl std::fmt::Display for ConsensusHardfork {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConsensusHardfork::ConsensusAlpha => write!(f, "ConsensusAlpha"),
        }
    }
}

// ---------------------------------------------------------------------------
// Fork conditions
// ---------------------------------------------------------------------------

/// Activation condition for a consensus hardfork.
///
/// Supports two activation dimensions:
/// - **Timestamp** (microseconds) — matches `BlockInfo.timestamp_usecs`
/// - **Epoch** — matches `BlockInfo.epoch`
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ForkCondition {
    /// Activated at a specific timestamp in microseconds (inclusive).
    Timestamp(u64),
    /// Activated at a specific epoch number (inclusive).
    Epoch(u64),
    /// Never activated.
    Never,
}

impl ForkCondition {
    /// Returns `true` if this condition is satisfied at the given timestamp (microseconds).
    /// Only evaluates `Timestamp` conditions; `Epoch` conditions return `false`.
    pub fn is_active_at_timestamp(&self, timestamp_usecs: u64) -> bool {
        match self {
            ForkCondition::Timestamp(activation) => timestamp_usecs >= *activation,
            _ => false,
        }
    }

    /// Returns `true` if this condition is satisfied at the given epoch.
    /// Only evaluates `Epoch` conditions; `Timestamp` conditions return `false`.
    pub fn is_active_at_epoch(&self, epoch: u64) -> bool {
        match self {
            ForkCondition::Epoch(activation) => epoch >= *activation,
            _ => false,
        }
    }

    /// Returns `true` if this fork transitions exactly at the given timestamp.
    pub fn transitions_at_timestamp(&self, timestamp_usecs: u64) -> bool {
        match self {
            ForkCondition::Timestamp(activation) => timestamp_usecs == *activation,
            _ => false,
        }
    }

    /// Returns `true` if this fork transitions exactly at the given epoch.
    pub fn transitions_at_epoch(&self, epoch: u64) -> bool {
        match self {
            ForkCondition::Epoch(activation) => epoch == *activation,
            _ => false,
        }
    }
}

// ---------------------------------------------------------------------------
// Hardfork container
// ---------------------------------------------------------------------------

/// Container for all consensus-layer hardfork configurations.
///
/// Initialized once at startup from genesis config, then stored in a global
/// [`OnceLock`] for lock-free reads throughout the node's lifetime.
#[derive(Clone, Debug)]
pub struct ConsensusHardforks {
    forks: HashMap<ConsensusHardfork, ForkCondition>,
}

impl ConsensusHardforks {
    /// Create an empty hardforks container (all forks default to `Never`).
    pub fn new() -> Self {
        Self {
            forks: HashMap::new(),
        }
    }

    /// Build hardforks from genesis.json extra_fields.
    ///
    /// Reads known field names and registers corresponding fork conditions.
    ///
    /// Currently recognized fields:
    /// - `consensusAlpha` → `ConsensusAlpha` (Epoch)
    ///
    /// Unknown fields are silently ignored so that new forks can be added
    /// by simply extending this method.
    pub fn from_genesis_extra_fields<F>(get: F) -> Self
    where
        F: Fn(&str) -> Option<u64>,
    {
        let mut hardforks = Self::new();
        if let Some(epoch) = get("consensusAlpha") {
            hardforks.insert(
                ConsensusHardfork::ConsensusAlpha,
                ForkCondition::Epoch(epoch),
            );
        }
        // Future forks: add more `if let` blocks here following the same pattern.
        hardforks
    }

    /// Register a hardfork with its activation condition.
    pub fn insert(&mut self, fork: ConsensusHardfork, condition: ForkCondition) {
        self.forks.insert(fork, condition);
    }

    /// Check if a hardfork is active at the given timestamp (microseconds).
    /// Only matches `ForkCondition::Timestamp` conditions.
    pub fn is_active(&self, fork: ConsensusHardfork, timestamp_usecs: u64) -> bool {
        self.forks
            .get(&fork)
            .map_or(false, |c| c.is_active_at_timestamp(timestamp_usecs))
    }

    /// Check if a hardfork is active at the given epoch.
    /// Only matches `ForkCondition::Epoch` conditions.
    pub fn is_active_at_epoch(&self, fork: ConsensusHardfork, epoch: u64) -> bool {
        self.forks
            .get(&fork)
            .map_or(false, |c| c.is_active_at_epoch(epoch))
    }

    /// Get the activation condition for a hardfork.
    /// Returns `ForkCondition::Never` if the fork is not registered.
    pub fn condition(&self, fork: ConsensusHardfork) -> ForkCondition {
        self.forks
            .get(&fork)
            .copied()
            .unwrap_or(ForkCondition::Never)
    }

    /// Iterate over all registered forks and their conditions.
    pub fn iter(&self) -> impl Iterator<Item = (&ConsensusHardfork, &ForkCondition)> {
        self.forks.iter()
    }
}

impl Default for ConsensusHardforks {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ConsensusHardforks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (fork, condition) in &self.forks {
            match condition {
                ForkCondition::Timestamp(ts) => {
                    writeln!(f, "  {}: timestamp {} us", fork, ts)?
                },
                ForkCondition::Epoch(epoch) => {
                    writeln!(f, "  {}: epoch {}", fork, epoch)?
                },
                ForkCondition::Never => writeln!(f, "  {}: never", fork)?,
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Global access
// ---------------------------------------------------------------------------

/// Global consensus hardforks configuration.
/// Set once at node startup via [`init_consensus_hardforks`], read via
/// [`is_consensus_fork_active`] throughout the node's lifetime.
static CONSENSUS_HARDFORKS: OnceLock<ConsensusHardforks> = OnceLock::new();

/// Initialize the global consensus hardforks configuration.
///
/// Must be called exactly once at node startup. Panics if called more than once.
pub fn init_consensus_hardforks(hardforks: ConsensusHardforks) {
    CONSENSUS_HARDFORKS
        .set(hardforks)
        .expect("consensus hardforks already initialized");
}

/// Query whether a consensus hardfork is active at the given timestamp (microseconds).
///
/// Returns `false` if hardforks have not been initialized yet, or if the
/// specified fork is not registered.
pub fn is_consensus_fork_active(fork: ConsensusHardfork, timestamp_usecs: u64) -> bool {
    CONSENSUS_HARDFORKS
        .get()
        .map_or(false, |h| h.is_active(fork, timestamp_usecs))
}

/// Query whether a consensus hardfork is active at the given epoch.
///
/// Returns `false` if hardforks have not been initialized yet, or if the
/// specified fork is not registered or uses a different condition type.
pub fn is_consensus_fork_active_at_epoch(fork: ConsensusHardfork, epoch: u64) -> bool {
    CONSENSUS_HARDFORKS
        .get()
        .map_or(false, |h| h.is_active_at_epoch(fork, epoch))
}

/// Get a reference to the global consensus hardforks, if initialized.
pub fn consensus_hardforks() -> Option<&'static ConsensusHardforks> {
    CONSENSUS_HARDFORKS.get()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fork_condition_timestamp() {
        let cond = ForkCondition::Timestamp(1_000_000);
        assert!(!cond.is_active_at_timestamp(999_999));
        assert!(cond.is_active_at_timestamp(1_000_000));
        assert!(cond.is_active_at_timestamp(1_000_001));
        assert!(!cond.transitions_at_timestamp(999_999));
        assert!(cond.transitions_at_timestamp(1_000_000));
        assert!(!cond.transitions_at_timestamp(1_000_001));
    }

    #[test]
    fn test_fork_condition_never() {
        let cond = ForkCondition::Never;
        assert!(!cond.is_active_at_timestamp(0));
        assert!(!cond.is_active_at_timestamp(u64::MAX));
    }

    #[test]
    fn test_consensus_hardforks_container_epoch() {
        let mut hardforks = ConsensusHardforks::new();
        hardforks.insert(
            ConsensusHardfork::ConsensusAlpha,
            ForkCondition::Epoch(100),
        );

        assert!(!hardforks.is_active_at_epoch(ConsensusHardfork::ConsensusAlpha, 99));
        assert!(hardforks.is_active_at_epoch(ConsensusHardfork::ConsensusAlpha, 100));
        assert!(hardforks.is_active_at_epoch(ConsensusHardfork::ConsensusAlpha, 101));

        assert_eq!(
            hardforks.condition(ConsensusHardfork::ConsensusAlpha),
            ForkCondition::Epoch(100)
        );
    }

    #[test]
    fn test_unregistered_fork_returns_never() {
        let hardforks = ConsensusHardforks::new();
        assert_eq!(
            hardforks.condition(ConsensusHardfork::ConsensusAlpha),
            ForkCondition::Never
        );
        assert!(!hardforks.is_active(ConsensusHardfork::ConsensusAlpha, 0));
        assert!(!hardforks.is_active(ConsensusHardfork::ConsensusAlpha, u64::MAX));
        assert!(!hardforks.is_active_at_epoch(ConsensusHardfork::ConsensusAlpha, 0));
        assert!(!hardforks.is_active_at_epoch(ConsensusHardfork::ConsensusAlpha, u64::MAX));
    }

    #[test]
    fn test_fork_condition_epoch() {
        let cond = ForkCondition::Epoch(10);
        assert!(!cond.is_active_at_epoch(9));
        assert!(cond.is_active_at_epoch(10));
        assert!(cond.is_active_at_epoch(11));
        assert!(!cond.transitions_at_epoch(9));
        assert!(cond.transitions_at_epoch(10));
        assert!(!cond.transitions_at_epoch(11));
        // Epoch condition should NOT respond to timestamp queries
        assert!(!cond.is_active_at_timestamp(10));
        assert!(!cond.is_active_at_timestamp(u64::MAX));
    }

    #[test]
    fn test_timestamp_condition_does_not_match_epoch() {
        let cond = ForkCondition::Timestamp(1_000_000);
        // Timestamp condition should NOT respond to epoch queries
        assert!(!cond.is_active_at_epoch(1_000_000));
        assert!(!cond.is_active_at_epoch(u64::MAX));
    }

    #[test]
    fn test_never_condition_for_epoch() {
        let cond = ForkCondition::Never;
        assert!(!cond.is_active_at_epoch(0));
        assert!(!cond.is_active_at_epoch(u64::MAX));
    }
}
