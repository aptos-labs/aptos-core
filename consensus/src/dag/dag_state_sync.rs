// Copyright © Aptos Foundation

use super::{
    adapter::TLedgerInfoProvider,
    dag_fetcher::TDagFetcher,
    dag_store::Dag,
    storage::DAGStorage,
    types::{CertifiedNodeMessage, RemoteFetchRequest},
    ProofNotifier,
};
use crate::state_replication::StateComputer;
use anyhow::ensure;
use aptos_consensus_types::common::Round;
use aptos_infallible::RwLock;
use aptos_logger::error;
use aptos_time_service::TimeService;
use aptos_types::{
    epoch_change::EpochChangeProof, epoch_state::EpochState, ledger_info::LedgerInfoWithSignatures,
};
use itertools::Itertools;
use std::sync::Arc;

// TODO: move this to onchain config
// TODO: temporarily setting DAG_WINDOW to 1 to maintain Shoal safety
pub const DAG_WINDOW: usize = 1;
pub const STATE_SYNC_WINDOW_MULTIPLIER: usize = 30;

#[derive(Debug)]
pub enum StateSyncStatus {
    NeedsSync(CertifiedNodeMessage),
    Synced(Option<CertifiedNodeMessage>),
    EpochEnds,
}

pub(super) struct StateSyncTrigger {
    epoch_state: Arc<EpochState>,
    ledger_info_provider: Arc<dyn TLedgerInfoProvider>,
    dag_store: Arc<RwLock<Dag>>,
    proof_notifier: Arc<dyn ProofNotifier>,
}

impl StateSyncTrigger {
    pub(super) fn new(
        epoch_state: Arc<EpochState>,
        ledger_info_provider: Arc<dyn TLedgerInfoProvider>,
        dag_store: Arc<RwLock<Dag>>,
        proof_notifier: Arc<dyn ProofNotifier>,
    ) -> Self {
        Self {
            epoch_state,
            ledger_info_provider,
            dag_store,
            proof_notifier,
        }
    }

    fn verify_ledger_info(&self, ledger_info: &LedgerInfoWithSignatures) -> anyhow::Result<()> {
        ensure!(ledger_info.commit_info().epoch() == self.epoch_state.epoch);

        if ledger_info.commit_info().round() > 0 {
            ledger_info
                .verify_signatures(&self.epoch_state.verifier)
                .map_err(|e| anyhow::anyhow!("unable to verify ledger info: {}", e))?;
        }

        Ok(())
    }

    /// This method checks if a state sync is required, and if so,
    /// notifies the bootstraper, to let the bootstraper can abort this task.
    pub(super) async fn check(
        &self,
        node: CertifiedNodeMessage,
    ) -> anyhow::Result<StateSyncStatus> {
        let ledger_info_with_sigs = node.ledger_info();

        if !self.need_sync_for_ledger_info(ledger_info_with_sigs) {
            return Ok(StateSyncStatus::Synced(Some(node)));
        }

        // Only verify the certificate if we need to sync
        self.verify_ledger_info(ledger_info_with_sigs)?;

        self.notify_commit_proof(ledger_info_with_sigs).await;

        if ledger_info_with_sigs.ledger_info().ends_epoch() {
            self.proof_notifier
                .send_epoch_change(EpochChangeProof::new(
                    vec![ledger_info_with_sigs.clone()],
                    /* more = */ false,
                ))
                .await;
            return Ok(StateSyncStatus::EpochEnds);
        }

        Ok(StateSyncStatus::NeedsSync(node))
    }

    /// Fast forward in the decoupled-execution pipeline if the block exists there
    async fn notify_commit_proof(&self, ledger_info: &LedgerInfoWithSignatures) {
        // if the anchor exists between ledger info round and highest ordered round
        // Note: ledger info round <= highest ordered round
        if self
            .ledger_info_provider
            .get_highest_committed_anchor_round()
            < ledger_info.commit_info().round()
            && self
                .dag_store
                .read()
                .highest_ordered_anchor_round()
                .unwrap_or_default()
                >= ledger_info.commit_info().round()
        {
            self.proof_notifier
                .send_commit_proof(ledger_info.clone())
                .await
        }
    }

    /// Check if we're far away from this ledger info and need to sync.
    /// This ensures that the block referred by the ledger info is not in buffer manager.
    fn need_sync_for_ledger_info(&self, li: &LedgerInfoWithSignatures) -> bool {
        if li.commit_info().round()
            <= self
                .ledger_info_provider
                .get_highest_committed_anchor_round()
        {
            return false;
        }

        let dag_reader = self.dag_store.read();
        // check whether if DAG order round is behind the given ledger info round
        // (meaning consensus is behind) or
        // the highest committed anchor round is 2*DAG_WINDOW behind the given ledger info round
        // (meaning execution is behind the DAG window)
        dag_reader
            .highest_ordered_anchor_round()
            .is_some_and(|r| r < li.commit_info().round())
            || self
                .ledger_info_provider
                .get_highest_committed_anchor_round()
                + ((STATE_SYNC_WINDOW_MULTIPLIER * DAG_WINDOW) as Round)
                < li.commit_info().round()
    }
}

pub(super) struct DagStateSynchronizer {
    epoch_state: Arc<EpochState>,
    time_service: TimeService,
    state_computer: Arc<dyn StateComputer>,
    storage: Arc<dyn DAGStorage>,
}

impl DagStateSynchronizer {
    pub fn new(
        epoch_state: Arc<EpochState>,
        time_service: TimeService,
        state_computer: Arc<dyn StateComputer>,
        storage: Arc<dyn DAGStorage>,
    ) -> Self {
        Self {
            epoch_state,
            time_service,
            state_computer,
            storage,
        }
    }

    /// Note: Assumes that the sync checks have been done
    pub async fn sync_dag_to(
        &self,
        node: &CertifiedNodeMessage,
        dag_fetcher: impl TDagFetcher,
        current_dag_store: Arc<RwLock<Dag>>,
        highest_committed_anchor_round: Round,
    ) -> anyhow::Result<Option<Dag>> {
        let commit_li = node.ledger_info();

        {
            let dag_reader = current_dag_store.read();
            assert!(
                dag_reader
                    .highest_ordered_anchor_round()
                    .unwrap_or_default()
                    < commit_li.commit_info().round()
                    || highest_committed_anchor_round
                        + ((STATE_SYNC_WINDOW_MULTIPLIER * DAG_WINDOW) as Round)
                        < commit_li.commit_info().round()
            );
        }

        // TODO: there is a case where DAG fetches missing nodes in window and a crash happens and when we restart,
        // we end up with a gap between the DAG and we need to be smart enough to clean up the DAG before the gap.

        // Create a new DAG store and Fetch blocks
        let target_round = node.round();
        let start_round = commit_li
            .commit_info()
            .round()
            .saturating_sub(DAG_WINDOW as Round);
        let sync_dag_store = Arc::new(RwLock::new(Dag::new_empty(
            self.epoch_state.clone(),
            self.storage.clone(),
            start_round,
        )));
        let bitmask = { sync_dag_store.read().bitmask(target_round) };
        let request = RemoteFetchRequest::new(
            self.epoch_state.epoch,
            node.parents_metadata().cloned().collect_vec(),
            bitmask,
        );

        let responders = node
            .certificate()
            .signatures()
            .get_signers_addresses(&self.epoch_state.verifier.get_ordered_account_addresses());

        match dag_fetcher
            .fetch(request, responders, sync_dag_store.clone())
            .await
        {
            Ok(_) => {},
            Err(err) => {
                error!("error fetching nodes {}", err);
                return Err(err);
            },
        }

        self.state_computer.sync_to(commit_li.clone()).await?;

        Ok(Arc::into_inner(sync_dag_store).map(|r| r.into_inner()))
    }
}
