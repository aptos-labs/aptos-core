// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::consensus_observer::error::Error;
use aptos_config::{config::ConsensusObserverConfig, network_id::PeerNetworkId};
use aptos_storage_interface::DbReader;
use aptos_time_service::{TimeService, TimeServiceTrait};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

/// A single consensus observer subscription
pub struct ConsensusObserverSubscription {
    // The configuration of the consensus observer
    consensus_observer_config: ConsensusObserverConfig,

    // A handle to storage (used to read the latest state and check progress)
    db_reader: Arc<dyn DbReader>,

    // The peer network id of the active subscription
    peer_network_id: PeerNetworkId,

    // The timestamp of the last message received from the peer
    last_message_receive_time: Instant,

    // The highest synced version we've seen from storage (along with the time at which it was seen)
    highest_synced_version: (u64, Instant),

    // The time service to check the last message receive time
    time_service: TimeService,
}

impl ConsensusObserverSubscription {
    pub fn new(
        consensus_observer_config: ConsensusObserverConfig,
        db_reader: Arc<dyn DbReader>,
        peer_network_id: PeerNetworkId,
        time_service: TimeService,
    ) -> Self {
        let time_now = time_service.now();

        Self {
            consensus_observer_config,
            db_reader,
            peer_network_id,
            last_message_receive_time: time_now,
            highest_synced_version: (0, time_now),
            time_service,
        }
    }

    /// Verifies that the subscription has not timed out based on the last
    /// received message time. Otherwise, an error is returned.
    pub fn check_subscription_timeout(&self) -> Result<(), Error> {
        // Calculate the duration since the last message
        let time_now = self.time_service.now();
        let duration_since_last_message = time_now.duration_since(self.last_message_receive_time);

        // Check if the subscription has timed out
        if duration_since_last_message
            > Duration::from_millis(self.consensus_observer_config.max_subscription_timeout_ms)
        {
            return Err(Error::SubscriptionTimeout(format!(
                "Subscription to peer: {} has timed out! No message received for: {:?}",
                self.peer_network_id, duration_since_last_message
            )));
        }

        Ok(())
    }

    /// Verifies that the DB is continuing to sync and commit new data
    pub fn check_syncing_progress(&mut self) -> Result<(), Error> {
        // Get the current synced version from storage
        let current_synced_version =
            self.db_reader
                .get_latest_ledger_info_version()
                .map_err(|error| {
                    Error::UnexpectedError(format!(
                        "Failed to read highest synced version: {:?}",
                        error
                    ))
                })?;

        // Verify that the synced version is increasing appropriately
        let (highest_synced_version, highest_version_timestamp) = self.highest_synced_version;
        if current_synced_version <= highest_synced_version {
            // The synced version hasn't increased. Check if we should terminate
            // the subscription based on the last time the highest synced version was seen.
            let duration_since_highest_seen = highest_version_timestamp.elapsed();
            if duration_since_highest_seen
                > Duration::from_millis(
                    self.consensus_observer_config.max_synced_version_timeout_ms,
                )
            {
                return Err(Error::SubscriptionTimeout(format!(
                    "The DB is not making sync progress! Highest synced version: {}, elapsed: {:?}",
                    highest_synced_version, duration_since_highest_seen
                )));
            }
        }

        // Update the highest synced version and time
        self.highest_synced_version = (current_synced_version, self.time_service.now());

        Ok(())
    }

    /// Returns the peer network id of the subscription
    pub fn get_peer_network_id(&self) -> PeerNetworkId {
        self.peer_network_id
    }

    /// Verifies the given message is from the expected peer
    pub fn verify_message_sender(&mut self, peer_network_id: &PeerNetworkId) -> Result<(), Error> {
        // Verify the message is from the expected peer
        if self.peer_network_id != *peer_network_id {
            return Err(Error::UnexpectedError(format!(
                "Received message from unexpected peer: {}! Subscribed to: {}",
                peer_network_id, self.peer_network_id
            )));
        }

        // Update the last message receive time
        self.last_message_receive_time = self.time_service.now();

        Ok(())
    }
}
