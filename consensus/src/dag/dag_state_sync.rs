// Copyright © Aptos Foundation

use super::{
    adapter::Notifier,
    dag_fetcher::{DagFetcher, TDagFetcher},
    dag_store::Dag,
    storage::DAGStorage,
    types::{CertifiedNodeMessage, RemoteFetchRequest},
    TDAGNetworkSender,
};
use crate::state_replication::StateComputer;
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

pub(super) struct StateSyncManager {
    epoch_state: Arc<EpochState>,
    network: Arc<dyn TDAGNetworkSender>,
    notifier: Arc<dyn Notifier>,
    time_service: TimeService,
    state_computer: Arc<dyn StateComputer>,
    storage: Arc<dyn DAGStorage>,
    dag_store: Arc<RwLock<Dag>>,
}

impl StateSyncManager {
    pub fn new(
        epoch_state: Arc<EpochState>,
        network: Arc<dyn TDAGNetworkSender>,
        notifier: Arc<dyn Notifier>,
        time_service: TimeService,
        state_computer: Arc<dyn StateComputer>,
        storage: Arc<dyn DAGStorage>,
        dag_store: Arc<RwLock<Dag>>,
    ) -> Self {
        Self {
            epoch_state,
            network,
            notifier,
            time_service,
            state_computer,
            storage,
            dag_store,
        }
    }

    pub async fn sync_to(
        &self,
        node: &CertifiedNodeMessage,
    ) -> anyhow::Result<Option<Arc<RwLock<Dag>>>> {
        self.sync_to_highest_commit_cert(node.ledger_info()).await;
        self.try_sync_to_highest_ordered_anchor(node).await
    }

    /// Fast forward in the decoupled-execution pipeline if the block exists there
    pub async fn sync_to_highest_commit_cert(&self, ledger_info: &LedgerInfoWithSignatures) {
        let send_commit_proof = {
            let dag_reader = self.dag_store.read();
            dag_reader.highest_committed_anchor_round() < ledger_info.commit_info().round()
                && dag_reader
                    .highest_ordered_anchor_round()
                    .unwrap_or_default()
                    >= ledger_info.commit_info().round()
        };

        // if the anchor exists between ledger info round and highest ordered round
        // Note: ledger info round <= highest ordered round
        if send_commit_proof {
            self.notifier.send_commit_proof(ledger_info.clone()).await
        }
    }

    /// Check if we're far away from this ledger info and need to sync.
    /// This ensures that the block referred by the ledger info is not in buffer manager.
    pub fn need_sync_for_ledger_info(&self, li: &LedgerInfoWithSignatures) -> bool {
        let dag_reader = self.dag_store.read();
        // check whether if DAG order round is behind the given ledger info round
        // (meaning consensus is behind) or
        // the highest committed anchor round is 2*DAG_WINDOW behind the given ledger info round
        // (meaning execution is behind the DAG window)
        (dag_reader
            .highest_ordered_anchor_round()
            .unwrap_or_default()
            < li.commit_info().round())
            || dag_reader.highest_committed_anchor_round()
                + ((STATE_SYNC_WINDOW_MULTIPLIER * DAG_WINDOW) as Round)
                < li.commit_info().round()
    }

    pub async fn try_sync_to_highest_ordered_anchor(
        &self,
        node: &CertifiedNodeMessage,
    ) -> anyhow::Result<Option<Arc<RwLock<Dag>>>> {
        // Check whether to actually sync
        let commit_li = node.ledger_info();
        if !self.need_sync_for_ledger_info(commit_li) {
            return Ok(None);
        }

        let dag_fetcher = Arc::new(DagFetcher::new(
            self.epoch_state.clone(),
            self.network.clone(),
            self.time_service.clone(),
        ));

        self.sync_to_highest_ordered_anchor(node, dag_fetcher).await
    }

    /// Note: Assumes that the sync checks have been done
    pub async fn sync_to_highest_ordered_anchor(
        &self,
        node: &CertifiedNodeMessage,
        dag_fetcher: Arc<impl TDagFetcher>,
    ) -> anyhow::Result<Option<Arc<RwLock<Dag>>>> {
        let commit_li = node.ledger_info();

        if commit_li.ledger_info().ends_epoch() {
            self.notifier
                .send_epoch_change(EpochChangeProof::new(
                    vec![commit_li.clone()],
                    /* more = */ false,
                ))
                .await;
            // TODO: make sure to terminate DAG and yield to epoch manager
            return Ok(None);
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
            commit_li.commit_info().round(),
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

        // State sync
        self.state_computer.sync_to(commit_li.clone()).await?;

        // TODO: the caller should rebootstrap the order rule

        Ok(Some(sync_dag_store))
    }
}
