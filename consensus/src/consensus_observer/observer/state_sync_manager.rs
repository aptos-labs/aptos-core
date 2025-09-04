// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    consensus_observer::{
        common::{
            logging::{LogEntry, LogSchema},
            metrics,
        },
        network::observer_message::CommitDecision,
    },
    pipeline::execution_client::TExecutionClient,
};
use velor_config::config::ConsensusObserverConfig;
use velor_logger::{error, info};
use velor_reliable_broadcast::DropGuard;
use velor_types::ledger_info::LedgerInfoWithSignatures;
use futures::future::{AbortHandle, Abortable};
use std::{sync::Arc, time::Duration};
use tokio::sync::mpsc::UnboundedSender;

/// A simple state sync notification to notify consensus
/// observer that state syncing has completed.
pub enum StateSyncNotification {
    FallbackSyncCompleted(LedgerInfoWithSignatures),
    CommitSyncCompleted(LedgerInfoWithSignatures),
}

impl StateSyncNotification {
    /// Returns a new state sync notification that fallback syncing has completed
    pub fn fallback_sync_completed(ledger_info: LedgerInfoWithSignatures) -> Self {
        Self::FallbackSyncCompleted(ledger_info)
    }

    /// Returns a new state sync notification that syncing to a commit has completed
    pub fn commit_sync_completed(ledger_info: LedgerInfoWithSignatures) -> Self {
        Self::CommitSyncCompleted(ledger_info)
    }
}

/// The manager for interacting with state sync
pub struct StateSyncManager {
    // The configuration of the consensus observer
    consensus_observer_config: ConsensusObserverConfig,

    // The execution client to the buffer manager
    execution_client: Arc<dyn TExecutionClient>,

    // The sender to notify consensus observer that state syncing to
    // the specified ledger info has completed.
    state_sync_notification_sender: UnboundedSender<StateSyncNotification>,

    // The active fallback sync handle. If this is set, it means that
    // we've fallen back to state sync, and we should wait for it to complete.
    fallback_sync_handle: Option<DropGuard>,

    // The active sync to commit handle. If this is set, it means that
    // we're waiting for state sync to synchronize to a known commit decision.
    // The flag indicates if the commit will transition us to a new epoch.
    sync_to_commit_handle: Option<(DropGuard, bool)>,
}

impl StateSyncManager {
    pub fn new(
        consensus_observer_config: ConsensusObserverConfig,
        execution_client: Arc<dyn TExecutionClient>,
        state_sync_notification_sender: UnboundedSender<StateSyncNotification>,
    ) -> Self {
        Self {
            consensus_observer_config,
            execution_client,
            state_sync_notification_sender,
            fallback_sync_handle: None,
            sync_to_commit_handle: None,
        }
    }

    /// Resets the handle for any active sync to a commit decision
    pub fn clear_active_commit_sync(&mut self) {
        // If we're not actively syncing to a commit, log an error
        if !self.is_syncing_to_commit() {
            error!(LogSchema::new(LogEntry::ConsensusObserver)
                .message("Failed to clear sync to commit decision! No active sync handle found!"));
        }

        self.sync_to_commit_handle = None;
    }

    /// Resets the handle for any active fallback sync
    pub fn clear_active_fallback_sync(&mut self) {
        // If we're not actively syncing in fallback mode, log an error
        if !self.in_fallback_mode() {
            error!(LogSchema::new(LogEntry::ConsensusObserver)
                .message("Failed to clear fallback sync! No active sync handle found!"));
        }

        self.fallback_sync_handle = None;
    }

    /// Returns true iff state sync is currently executing in fallback mode
    pub fn in_fallback_mode(&self) -> bool {
        self.fallback_sync_handle.is_some()
    }

    /// Returns true iff we are waiting for state sync to synchronize
    /// to a commit decision that will transition us to a new epoch
    pub fn is_syncing_through_epoch(&self) -> bool {
        matches!(self.sync_to_commit_handle, Some((_, true)))
    }

    /// Returns true iff state sync is currently syncing to a commit decision
    pub fn is_syncing_to_commit(&self) -> bool {
        self.sync_to_commit_handle.is_some()
    }

    /// Invokes state sync to synchronize in fallback mode
    pub fn sync_for_fallback(&mut self) {
        // Log that we're starting to sync in fallback mode
        info!(
            LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                "Started syncing in fallback mode! Syncing duration: {:?} ms!",
                self.consensus_observer_config.observer_fallback_duration_ms
            ))
        );

        // Update the state sync fallback counter
        metrics::increment_counter_without_labels(&metrics::OBSERVER_STATE_SYNC_FALLBACK_COUNTER);

        // Clone the required components for the state sync task
        let consensus_observer_config = self.consensus_observer_config;
        let execution_client = self.execution_client.clone();
        let sync_notification_sender = self.state_sync_notification_sender.clone();

        // Spawn a task to sync for the fallback
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        tokio::spawn(Abortable::new(
            async move {
                // Update the state sync metrics now that we're syncing for the fallback
                metrics::set_gauge_with_label(
                    &metrics::OBSERVER_STATE_SYNC_EXECUTING,
                    metrics::STATE_SYNCING_FOR_FALLBACK,
                    1, // We're syncing for the fallback
                );

                // Get the fallback duration
                let fallback_duration =
                    Duration::from_millis(consensus_observer_config.observer_fallback_duration_ms);

                // Sync for the fallback duration
                let latest_synced_ledger_info = match execution_client
                    .clone()
                    .sync_for_duration(fallback_duration)
                    .await
                {
                    Ok(latest_synced_ledger_info) => latest_synced_ledger_info,
                    Err(error) => {
                        error!(LogSchema::new(LogEntry::ConsensusObserver)
                            .message(&format!("Failed to sync for fallback! Error: {:?}", error)));
                        return;
                    },
                };

                // Notify consensus observer that we've synced for the fallback
                let state_sync_notification =
                    StateSyncNotification::fallback_sync_completed(latest_synced_ledger_info);
                if let Err(error) = sync_notification_sender.send(state_sync_notification) {
                    error!(
                        LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                            "Failed to send state sync notification for fallback! Error: {:?}",
                            error
                        ))
                    );
                }

                // Clear the state sync metrics now that we're done syncing
                metrics::set_gauge_with_label(
                    &metrics::OBSERVER_STATE_SYNC_EXECUTING,
                    metrics::STATE_SYNCING_FOR_FALLBACK,
                    0, // We're no longer syncing for the fallback
                );
            },
            abort_registration,
        ));

        // Save the sync task handle
        self.fallback_sync_handle = Some(DropGuard::new(abort_handle));
    }

    /// Invokes state sync to synchronize to a new commit decision
    pub fn sync_to_commit(&mut self, commit_decision: CommitDecision, epoch_changed: bool) {
        // Log that we're starting to sync to the commit decision
        info!(
            LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                "Started syncing to commit: {}!",
                commit_decision.proof_block_info()
            ))
        );

        // Get the commit decision epoch and round
        let commit_epoch = commit_decision.epoch();
        let commit_round = commit_decision.round();

        // Clone the required components for the state sync task
        let execution_client = self.execution_client.clone();
        let sync_notification_sender = self.state_sync_notification_sender.clone();

        // Spawn a task to sync to the commit decision
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        tokio::spawn(Abortable::new(
            async move {
                // Update the state sync metrics now that we're syncing to a commit
                metrics::set_gauge_with_label(
                    &metrics::OBSERVER_STATE_SYNC_EXECUTING,
                    metrics::STATE_SYNCING_TO_COMMIT,
                    1, // We're syncing to a commit decision
                );

                // Sync to the commit decision
                if let Err(error) = execution_client
                    .clone()
                    .sync_to_target(commit_decision.commit_proof().clone())
                    .await
                {
                    error!(
                        LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                            "Failed to sync to commit decision: {:?}! Error: {:?}",
                            commit_decision, error
                        ))
                    );
                    return;
                }

                // Notify consensus observer that we've synced to the commit decision
                let state_sync_notification = StateSyncNotification::commit_sync_completed(
                    commit_decision.commit_proof().clone(),
                );
                if let Err(error) = sync_notification_sender.send(state_sync_notification) {
                    error!(
                        LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                            "Failed to send state sync notification for commit decision epoch: {:?}, round: {:?}! Error: {:?}",
                            commit_epoch, commit_round, error
                        ))
                    );
                }

                // Clear the state sync metrics now that we're done syncing
                metrics::set_gauge_with_label(
                    &metrics::OBSERVER_STATE_SYNC_EXECUTING,
                    metrics::STATE_SYNCING_TO_COMMIT,
                    0, // We're no longer syncing to a commit decision
                );
            },
            abort_registration,
        ));

        // Save the sync task handle
        self.sync_to_commit_handle = Some((DropGuard::new(abort_handle), epoch_changed));
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::pipeline::execution_client::DummyExecutionClient;
    use velor_types::{aggregate_signature::AggregateSignature, ledger_info::LedgerInfo};

    #[tokio::test]
    async fn test_clear_active_sync() {
        // Create a new state sync manager
        let consensus_observer_config = ConsensusObserverConfig::default();
        let (state_sync_notification_sender, _) = tokio::sync::mpsc::unbounded_channel();
        let mut state_sync_manager = StateSyncManager::new(
            consensus_observer_config,
            Arc::new(DummyExecutionClient),
            state_sync_notification_sender,
        );

        // Verify that there are no active sync handles
        assert!(!state_sync_manager.in_fallback_mode());
        assert!(!state_sync_manager.is_syncing_to_commit());

        // Sync to a commit and verify that the active sync handle is set
        let commit_decision = CommitDecision::new(LedgerInfoWithSignatures::new(
            LedgerInfo::dummy(),
            AggregateSignature::empty(),
        ));
        state_sync_manager.sync_to_commit(commit_decision, false);
        assert!(state_sync_manager.is_syncing_to_commit());

        // Clear the active sync handle and verify that it's reset
        state_sync_manager.clear_active_commit_sync();
        assert!(!state_sync_manager.is_syncing_to_commit());

        // Sync for the fallback and verify that the active sync handle is set
        state_sync_manager.sync_for_fallback();
        assert!(state_sync_manager.in_fallback_mode());

        // Clear the active sync handle and verify that it's reset
        state_sync_manager.clear_active_fallback_sync();
        assert!(!state_sync_manager.in_fallback_mode());
    }

    #[tokio::test]
    async fn test_is_syncing_through_epoch() {
        // Create a new state sync manager
        let consensus_observer_config = ConsensusObserverConfig::default();
        let (state_sync_notification_sender, _) = tokio::sync::mpsc::unbounded_channel();
        let mut state_sync_manager = StateSyncManager::new(
            consensus_observer_config,
            Arc::new(DummyExecutionClient),
            state_sync_notification_sender,
        );

        // Verify that we're not syncing through an epoch
        assert!(!state_sync_manager.is_syncing_through_epoch());

        // Sync to a commit that doesn't transition us to a new epoch
        let commit_decision = CommitDecision::new(LedgerInfoWithSignatures::new(
            LedgerInfo::dummy(),
            AggregateSignature::empty(),
        ));
        state_sync_manager.sync_to_commit(commit_decision, false);

        // Verify that we're not syncing through an epoch
        assert!(!state_sync_manager.is_syncing_through_epoch());

        // Clear the active sync handle and verify that it's reset
        state_sync_manager.clear_active_commit_sync();
        assert!(!state_sync_manager.is_syncing_through_epoch());

        // Sync to a commit that transitions us to a new epoch
        let commit_decision = CommitDecision::new(LedgerInfoWithSignatures::new(
            LedgerInfo::dummy(),
            AggregateSignature::empty(),
        ));
        state_sync_manager.sync_to_commit(commit_decision, true);

        // Verify that we're syncing through an epoch
        assert!(state_sync_manager.is_syncing_through_epoch());

        // Clear the active sync handle and verify that it's reset
        state_sync_manager.clear_active_commit_sync();
        assert!(!state_sync_manager.is_syncing_through_epoch());

        // Sync for the fallback and verify that we're not syncing through an epoch
        state_sync_manager.sync_for_fallback();
        assert!(!state_sync_manager.is_syncing_through_epoch());
    }
}
