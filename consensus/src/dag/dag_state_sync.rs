// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{
    adapter::TLedgerInfoProvider,
    dag_fetcher::TDagFetcher,
    dag_store::DagStore,
    storage::DAGStorage,
    types::{CertifiedNodeMessage, NodeCertificateMessage, RemoteFetchRequest},
    ProofNotifier,
};
use crate::{
    dag::DAGMessage,
    monitor,
    network::{IncomingDAGRequest, RpcResponder},
    payload_manager::TPayloadManager,
    pipeline::execution_client::TExecutionClient,
};
use anyhow::{anyhow, ensure};
use aptos_bounded_executor::{BoundedExecutor, ConcurrentStream};
use aptos_channels::aptos_channel;
use aptos_consensus_types::common::{Author, Round};
use aptos_logger::{debug, error};
use aptos_time_service::TimeService;
use aptos_types::{
    epoch_change::EpochChangeProof, epoch_state::EpochState, ledger_info::LedgerInfoWithSignatures,
};
use core::fmt;
use futures::StreamExt;
use std::sync::Arc;
use tokio::runtime::Handle;

#[derive(Debug)]
pub enum SyncOutcome {
    NeedsSync(NodeCertificateMessage),
    Synced(Option<NodeCertificateMessage>),
    EpochEnds,
}

impl fmt::Display for SyncOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SyncOutcome::NeedsSync(_) => write!(f, "NeedsSync"),
            SyncOutcome::Synced(_) => write!(f, "Synced"),
            SyncOutcome::EpochEnds => write!(f, "EpochEnds"),
        }
    }
}

pub(super) struct StateSyncTrigger {
    dag_id: u8,
    epoch_state: Arc<EpochState>,
    ledger_info_provider: Arc<dyn TLedgerInfoProvider>,
    dag_store: Arc<DagStore>,
    proof_notifier: Arc<dyn ProofNotifier>,
    dag_window_size_config: Round,
}

impl StateSyncTrigger {
    pub(super) fn new(
        dag_id: u8,
        epoch_state: Arc<EpochState>,
        ledger_info_provider: Arc<dyn TLedgerInfoProvider>,
        dag_store: Arc<DagStore>,
        proof_notifier: Arc<dyn ProofNotifier>,
        dag_window_size_config: Round,
    ) -> Self {
        Self {
            dag_id,
            epoch_state,
            ledger_info_provider,
            dag_store,
            proof_notifier,
            dag_window_size_config,
        }
    }

    fn verify_ledger_info(&self, ledger_info: &LedgerInfoWithSignatures) -> anyhow::Result<()> {
        ensure!(ledger_info.commit_info().epoch() == self.epoch_state.epoch);

        if ledger_info.get_highest_committed_rounds_for_shoalpp()[self.dag_id as usize] > 0 {
            ledger_info
                .verify_signatures(&self.epoch_state.verifier)
                .map_err(|e| anyhow::anyhow!("unable to verify ledger info: {}", e))?;
        }

        Ok(())
    }

    /// This method checks if a state sync is required
    pub(super) async fn check(&self, node: NodeCertificateMessage) -> anyhow::Result<SyncOutcome> {
        let ledger_info_with_sigs = node.ledger_info();

        self.notify_commit_proof(ledger_info_with_sigs).await;

        if !self.need_sync_for_ledger_info(ledger_info_with_sigs) {
            return Ok(SyncOutcome::Synced(Some(node)));
        }

        // Only verify the certificate if we need to sync
        self.verify_ledger_info(ledger_info_with_sigs)?;

        if ledger_info_with_sigs.ledger_info().ends_epoch() {
            self.proof_notifier
                .send_epoch_change(EpochChangeProof::new(
                    vec![ledger_info_with_sigs.clone()],
                    /* more = */ false,
                ))
                .await;
            return Ok(SyncOutcome::EpochEnds);
        }

        Ok(SyncOutcome::NeedsSync(node))
    }

    /// Fast forward in the decoupled-execution pipeline if the block exists there
    async fn notify_commit_proof(&self, ledger_info: &LedgerInfoWithSignatures) {
        // if the anchor exists between ledger info round and highest ordered round
        // Note: ledger info round <= highest ordered round
        let commit_info_anchor_round = ledger_info.get_highest_committed_rounds_for_shoalpp()
            [self.dag_id as usize]
            / self.epoch_state.verifier.len() as u64;

        let local_highest_committed_anchor_round = self
            .ledger_info_provider
            .get_highest_committed_anchor_round(self.dag_id);

        let _local_highest_ordered_round = self
            .dag_store
            .read()
            .highest_ordered_anchor_round()
            .unwrap_or_default();

        if local_highest_committed_anchor_round < commit_info_anchor_round
        // && local_highest_ordered_round < ledger_info.commit_info().round()
        {
            self.proof_notifier
                .send_commit_proof(ledger_info.clone())
                .await
        }
    }

    /// Check if we're far away from this ledger info and need to sync.
    /// This ensures that the block referred by the ledger info is not in buffer manager.
    fn need_sync_for_ledger_info(&self, li: &LedgerInfoWithSignatures) -> bool {
        let commit_info_anchor_round = li.get_highest_committed_rounds_for_shoalpp()
            [self.dag_id as usize]
            / self.epoch_state.verifier.len() as u64;
        let local_highest_committed_anchor_round = self
            .ledger_info_provider
            .get_highest_committed_anchor_round(self.dag_id);
        if commit_info_anchor_round <= local_highest_committed_anchor_round {
            return false;
        }

        let dag_reader = self.dag_store.read();
        // check whether if DAG order round is behind the given ledger info committed round
        // (meaning consensus is behind) or
        // the local highest committed anchor round is 2*DAG_WINDOW behind the given ledger info round
        // (meaning execution is behind the DAG window)

        // fetch can't work since nodes are garbage collected
        dag_reader.is_empty()
            || dag_reader.highest_round() + 1 + self.dag_window_size_config * 1000
                < commit_info_anchor_round
            || local_highest_committed_anchor_round + 2 * 10000 * self.dag_window_size_config
                < commit_info_anchor_round
    }
}

pub(super) struct DagStateSynchronizer {
    dag_id: u8,
    epoch_state: Arc<EpochState>,
    time_service: TimeService,
    execution_client: Arc<dyn TExecutionClient>,
    storage: Arc<dyn DAGStorage>,
    payload_manager: Arc<dyn TPayloadManager>,
    dag_window_size_config: Round,
}

impl DagStateSynchronizer {
    pub fn new(
        dag_id: u8,
        epoch_state: Arc<EpochState>,
        time_service: TimeService,
        execution_client: Arc<dyn TExecutionClient>,
        storage: Arc<dyn DAGStorage>,
        payload_manager: Arc<dyn TPayloadManager>,
        dag_window_size_config: Round,
    ) -> Self {
        Self {
            dag_id,
            epoch_state,
            time_service,
            execution_client,
            storage,
            payload_manager,
            dag_window_size_config,
        }
    }

    pub(crate) fn build_request(
        &self,
        node: &NodeCertificateMessage,
        current_dag_store: Arc<DagStore>,
        highest_committed_anchor_round: Round,
    ) -> (RemoteFetchRequest, Vec<Author>, Arc<DagStore>) {
        let commit_li = node.ledger_info();
        let commit_info_anchor_round = commit_li.get_highest_committed_rounds_for_shoalpp()
            [self.dag_id as usize]
            / self.epoch_state.verifier.len() as u64;
        {
            let dag_reader = current_dag_store.read();
            assert!(
                dag_reader
                    .highest_ordered_anchor_round()
                    .unwrap_or_default()
                    < commit_info_anchor_round
                    || highest_committed_anchor_round + self.dag_window_size_config
                        < commit_info_anchor_round
            );
        }

        // TODO: there is a case where DAG fetches missing nodes in window and a crash happens and when we restart,
        // we end up with a gap between the DAG and we need to be smart enough to clean up the DAG before the gap.

        // Create a new DAG store and Fetch blocks
        let target_round = node.round();
        let commit_round = commit_info_anchor_round;
        let start_round = commit_round.saturating_sub(self.dag_window_size_config);
        let sync_dag_store = Arc::new(DagStore::new_empty(
            self.dag_id,
            self.epoch_state.clone(),
            self.storage.clone(),
            self.payload_manager.clone(),
            start_round,
            self.dag_window_size_config,
        ));
        let bitmask = { sync_dag_store.read().bitmask(commit_round, target_round) };
        let request = RemoteFetchRequest::new(
            self.epoch_state.epoch,
            vec![node.metadata().clone()],
            bitmask,
        );

        let responders = node
            .certificate()
            .signatures()
            .get_signers_addresses(&self.epoch_state.verifier.get_ordered_account_addresses());

        (request, responders, sync_dag_store)
    }

    /// Note: Assumes that the sync checks have been done
    pub async fn sync_dag_to(
        &self,
        dag_fetcher: impl TDagFetcher,
        request: RemoteFetchRequest,
        responders: Vec<Author>,
        sync_dag_store: Arc<DagStore>,
        commit_li: LedgerInfoWithSignatures,
    ) -> anyhow::Result<DagStore> {
        let dag_store = sync_dag_store.clone();
        let commit_info = commit_li.commit_info().clone();
        let dag_sync_fut = async move {
            debug!(
                request = request,
                commit_info = commit_info,
                "Syncing DAG. Fetching Nodes"
            );
            monitor!(
                "dag_sync_fetch",
                dag_fetcher
                    .fetch(request, responders, dag_store)
                    .await
                    .map_err(|err| {
                        error!("error fetching nodes {}", err);
                        anyhow!(err)
                    })
            )?;

            Ok(())
        };

        let execution_client = self.execution_client.clone();
        let state_sync_fut = async move {
            debug!(target_ledger_info = commit_li, "Requesting sync to");
            monitor!(
                "dag_sync_state",
                execution_client
                    .sync_to(commit_li)
                    .await
                    .map_err(|err| anyhow!(err))
            )
        };
        // TODO: explain why this is okay
        futures::future::try_join(dag_sync_fut, state_sync_fut).await?;

        Ok(Arc::into_inner(sync_dag_store).unwrap())
    }
}

pub(crate) struct SyncModeMessageHandler {
    epoch_state: Arc<EpochState>,
    start_round: Round,
    target_round: Round,
    window: u64,
}

impl SyncModeMessageHandler {
    pub(crate) fn new(
        epoch_state: Arc<EpochState>,
        start_round: Round,
        target_round: Round,
        window: u64,
    ) -> Self {
        Self {
            epoch_state,
            start_round,
            target_round,
            window,
        }
    }

    pub(crate) async fn run(
        self,
        dag_rpc_rx: &mut aptos_channel::Receiver<Author, IncomingDAGRequest>,
        buffer: &mut Vec<DAGMessage>,
    ) -> Option<NodeCertificateMessage> {
        let executor = BoundedExecutor::new(32, Handle::current());
        let epoch_state = self.epoch_state.clone();
        let mut verified_msg_stream =
            dag_rpc_rx.concurrent_map(executor.clone(), move |rpc_request: IncomingDAGRequest| {
                let epoch_state = epoch_state.clone();
                async move {
                    let epoch = rpc_request.req.epoch();
                    let result = rpc_request
                        .req
                        .try_into()
                        .and_then(|dag_message: DAGMessage| {
                            monitor!(
                                "dag_message_verify",
                                dag_message.verify(rpc_request.sender, &epoch_state.verifier)
                            )?;
                            Ok(dag_message)
                        });
                    (result, epoch, rpc_request.sender, rpc_request.responder)
                }
            });

        while let Some((msg, epoch, author, responder)) = verified_msg_stream.next().await {
            match self.process_verified_message(msg, epoch, author, responder, buffer) {
                Ok(may_be_cert_node) => {
                    if let Some(next_sync_msg) = may_be_cert_node {
                        return Some(next_sync_msg);
                    }
                },
                Err(err) => {
                    error!("error processing {}", err);
                },
            }
        }
        None
    }

    fn process_verified_message(
        &self,
        dag_message_result: anyhow::Result<DAGMessage>,
        epoch: u64,
        author: Author,
        responder: RpcResponder,
        buffer: &mut Vec<DAGMessage>,
    ) -> anyhow::Result<Option<NodeCertificateMessage>> {
        match dag_message_result {
            Ok(dag_message) => {
                debug!(
                    epoch = epoch,
                    author = author,
                    message = dag_message,
                    "Verified DAG message"
                );
                match dag_message {
                    DAGMessage::NodeMsg(_) => {
                        debug!("ignoring node msg");
                    },
                    DAGMessage::NodeCertificateMsg(ref cert_node_msg) => {
                        if cert_node_msg.round() < self.start_round {
                            debug!("ignoring stale certified node msg");
                        } else if cert_node_msg.round() > self.target_round + (2 * self.window) {
                            debug!("cancelling current sync");
                            return Ok(Some(cert_node_msg.clone()));
                        } else {
                            buffer.push(dag_message);
                        }
                    },
                    DAGMessage::FetchRequest(_) => {
                        debug!("ignoring fetch msg");
                    },
                    _ => unreachable!("verification must catch this error"),
                };
            },
            Err(err) => {
                error!(error = ?err, "error verifying message");
                return Err(err);
            },
        };
        Ok(None)
    }
}
