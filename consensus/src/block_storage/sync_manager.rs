// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block_storage::{
        pending_blocks::PendingBlocks,
        tracing::{observe_block, BlockStage},
        BlockReader, BlockStore,
    },
    counters::{
        BLOCKS_FETCHED_FROM_NETWORK_IN_BLOCK_RETRIEVER,
        BLOCKS_FETCHED_FROM_NETWORK_WHILE_FAST_FORWARD_SYNC,
        BLOCKS_FETCHED_FROM_NETWORK_WHILE_INSERTING_QUORUM_CERT, LATE_EXECUTION_WITH_ORDER_VOTE_QC,
        SUCCESSFUL_EXECUTED_WITH_ORDER_VOTE_QC, SUCCESSFUL_EXECUTED_WITH_REGULAR_QC,
    },
    epoch_manager::LivenessStorageData,
    logging::{LogEvent, LogSchema},
    monitor,
    network::{IncomingBlockRetrievalRequest, NetworkSender},
    network_interface::ConsensusMsg,
    payload_manager::TPayloadManager,
    persistent_liveness_storage::{LedgerRecoveryData, PersistentLivenessStorage, RecoveryData},
    pipeline::execution_client::TExecutionClient,
    util::calculate_window_start_round,
};
use anyhow::{anyhow, bail, ensure, Context};
use aptos_consensus_types::{
    block::Block,
    block_retrieval::{
        BlockRetrievalRequest, BlockRetrievalRequestV1, BlockRetrievalRequestV2,
        BlockRetrievalResponse, BlockRetrievalStatus, NUM_PEERS_PER_RETRY, NUM_RETRIES,
        RETRY_INTERVAL_MSEC, RPC_TIMEOUT_MSEC,
    },
    common::Author,
    quorum_cert::QuorumCert,
    sync_info::SyncInfo,
    wrapped_ledger_info::WrappedLedgerInfo,
};
use aptos_crypto::HashValue;
use aptos_infallible::Mutex;
use aptos_logger::prelude::*;
use aptos_types::{
    account_address::AccountAddress, epoch_change::EpochChangeProof,
    ledger_info::LedgerInfoWithSignatures,
};
use fail::fail_point;
use futures::{stream::FuturesUnordered, FutureExt, StreamExt};
use futures_channel::oneshot;
use rand::{prelude::*, Rng};
use std::{clone::Clone, cmp::min, fmt::Display, sync::Arc, time::Duration};
use tokio::{time, time::timeout};

#[derive(Debug, PartialEq, Eq)]
/// Whether we need to do block retrieval if we want to insert a Quorum Cert.
pub enum NeedFetchResult {
    QCRoundBeforeRoot,
    QCAlreadyExist,
    QCBlockExist,
    NeedFetch,
}

impl BlockStore {
    /// Check if we're far away from this ledger info and need to sync.
    /// This ensures that the block referred by the ledger info is not in buffer manager.
    pub fn need_sync_for_ledger_info(&self, li: &LedgerInfoWithSignatures) -> bool {
        // TODO move min gap to fallback (30) to config, and if configurable make sure the value is
        // larger than buffer manager MAX_BACKLOG (20)
        (self.ordered_root().round() < li.commit_info().round()
            && !self.block_exists(li.commit_info().id()))
            || self.commit_root().round() + 30.max(2 * self.vote_back_pressure_limit)
                < li.commit_info().round()
    }

    /// Checks if quorum certificate can be inserted in block store without RPC
    /// Returns the enum to indicate the detailed status.
    pub fn need_fetch_for_quorum_cert(&self, qc: &QuorumCert) -> NeedFetchResult {
        if qc.certified_block().round() < self.ordered_root().round() {
            return NeedFetchResult::QCRoundBeforeRoot;
        }
        if self
            .get_quorum_cert_for_block(qc.certified_block().id())
            .is_some()
        {
            return NeedFetchResult::QCAlreadyExist;
        }
        if self.block_exists(qc.certified_block().id()) {
            return NeedFetchResult::QCBlockExist;
        }
        NeedFetchResult::NeedFetch
    }

    /// Fetches dependencies for given sync_info.quorum_cert
    /// If gap is large, performs state sync using sync_to_highest_ordered_cert
    /// Inserts sync_info.quorum_cert into block store as the last step
    pub async fn add_certs(
        &self,
        sync_info: &SyncInfo,
        mut retriever: BlockRetriever,
    ) -> anyhow::Result<()> {
        self.sync_to_highest_commit_cert(
            sync_info.highest_commit_cert().ledger_info(),
            retriever.network.clone(),
        )
        .await;

        // When the local ordered round is very old than the received sync_info, this function will
        // (1) resets the block store with highest commit cert = sync_info.highest_quorum_cert()
        // (2) insert all the blocks between (inclusive) highest_commit_cert.commit_info().id() to
        // highest_quorum_cert.certified_block().id() into the block store and storage
        // (3) insert the quorum cert for all the above blocks into the block store and storage
        // (4) executes all the blocks that are ordered while inserting the above quorum certs
        self.sync_to_highest_quorum_cert(
            sync_info.highest_quorum_cert().clone(),
            sync_info.highest_commit_cert().clone(),
            &mut retriever,
        )
        .await?;

        // The insert_ordered_cert(order_cert) function call expects that order_cert.commit_info().id() block
        // is already stored in block_store. So, we first call insert_quorum_cert(highest_quorum_cert).
        // This call will ensure that the highest ceritified block along with all its ancestors are inserted
        // into the block store.
        self.insert_quorum_cert(sync_info.highest_quorum_cert(), &mut retriever)
            .await?;

        // Even though we inserted the highest_quorum_cert (and its ancestors) in the above step,
        // we still need to insert ordered cert explicitly. This will send the highest ordered block
        // to execution.
        if self.order_vote_enabled {
            self.insert_ordered_cert(&sync_info.highest_ordered_cert())
                .await?;
        } else {
            // When order votes are disabled, the highest_ordered_cert().certified_block().id() need not be
            // one of the ancestors of highest_quorum_cert.certified_block().id() due to forks. So, we call
            // insert_quorum_cert instead of insert_ordered_cert as in the above case. This will ensure that
            // highest_ordered_cert().certified_block().id() is inserted the block store.
            self.insert_quorum_cert(
                &self
                    .highest_ordered_cert()
                    .as_ref()
                    .clone()
                    .into_quorum_cert(self.order_vote_enabled)?,
                &mut retriever,
            )
            .await?;
        }

        if let Some(tc) = sync_info.highest_2chain_timeout_cert() {
            self.insert_2chain_timeout_certificate(Arc::new(tc.clone()))?;
        }
        Ok(())
    }

    pub async fn insert_quorum_cert(
        &self,
        qc: &QuorumCert,
        retriever: &mut BlockRetriever,
    ) -> anyhow::Result<()> {
        match self.need_fetch_for_quorum_cert(qc) {
            NeedFetchResult::NeedFetch => self.fetch_quorum_cert(qc.clone(), retriever).await?,
            NeedFetchResult::QCBlockExist => self.insert_single_quorum_cert(qc.clone())?,
            NeedFetchResult::QCAlreadyExist => return Ok(()),
            _ => (),
        }
        if self.ordered_root().round() < qc.commit_info().round() {
            SUCCESSFUL_EXECUTED_WITH_REGULAR_QC.inc();
            self.send_for_execution(qc.into_wrapped_ledger_info())
                .await?;
            if qc.ends_epoch() {
                retriever
                    .network
                    .broadcast_epoch_change(EpochChangeProof::new(
                        vec![qc.ledger_info().clone()],
                        /* more = */ false,
                    ))
                    .await;
            }
        }
        Ok(())
    }

    // Before calling this function, we need to maintain an invariant that ordered_cert.commit_info().id()
    // is already in the block store. So, currently insert_ordered_cert calls are preceded by insert_quorum_cert calls
    // to ensure this.
    pub async fn insert_ordered_cert(
        &self,
        ordered_cert: &WrappedLedgerInfo,
    ) -> anyhow::Result<()> {
        if self.ordered_root().round() < ordered_cert.ledger_info().ledger_info().round() {
            if let Some(ordered_block) = self.get_block(ordered_cert.commit_info().id()) {
                if !ordered_block.block().is_nil_block() {
                    observe_block(
                        ordered_block.block().timestamp_usecs(),
                        BlockStage::OC_ADDED,
                    );
                }
                SUCCESSFUL_EXECUTED_WITH_ORDER_VOTE_QC.inc();
                self.send_for_execution(ordered_cert.clone()).await?;
            } else {
                bail!("Ordered block not found in block store when inserting ordered cert");
            }
        } else {
            LATE_EXECUTION_WITH_ORDER_VOTE_QC.inc();
        }
        Ok(())
    }

    /// Insert the quorum certificate separately from the block, used to split the processing of
    /// updating the consensus state(with qc) and deciding whether to vote(with block)
    /// The missing ancestors are going to be retrieved from the given peer. If a given peer
    /// fails to provide the missing ancestors, the qc is not going to be added.
    async fn fetch_quorum_cert(
        &self,
        qc: QuorumCert,
        retriever: &mut BlockRetriever,
    ) -> anyhow::Result<()> {
        let mut pending = vec![];
        let mut retrieve_qc = qc.clone();
        loop {
            if self.block_exists(retrieve_qc.certified_block().id()) {
                break;
            }
            BLOCKS_FETCHED_FROM_NETWORK_WHILE_INSERTING_QUORUM_CERT.inc_by(1);
            let target_block_retrieval_payload = match &self.window_size {
                None => TargetBlockRetrieval::TargetBlockId(retrieve_qc.certified_block().id()),
                Some(_) => TargetBlockRetrieval::TargetRound(retrieve_qc.certified_block().round()),
            };
            let mut blocks = retriever
                .retrieve_blocks_in_range(
                    retrieve_qc.certified_block().id(),
                    1,
                    target_block_retrieval_payload,
                    qc.ledger_info()
                        .get_voters(&retriever.validator_addresses()),
                )
                .await?;
            // retrieve_blocks_in_range guarantees that blocks has exactly 1 element
            let block = blocks.remove(0);
            retrieve_qc = block.quorum_cert().clone();
            pending.push(block);
        }
        // insert the qc <- block pair
        while let Some(block) = pending.pop() {
            let block_qc = block.quorum_cert().clone();
            self.insert_single_quorum_cert(block_qc)?;
            self.insert_block(block).await?;
        }
        self.insert_single_quorum_cert(qc)
    }

    /// Check the highest ordered cert sent by peer to see if we're behind and start a fast
    /// forward sync if the committed block doesn't exist in our tree.
    /// It works as follows:
    /// 1. request the gap blocks from the peer (from highest_ledger_info to highest_ordered_cert)
    /// 2. We persist the gap blocks to storage before start sync to ensure we could restart if we
    /// crash in the middle of the sync.
    /// 3. We prune the old tree and replace with a new tree built with the 3-chain.
    async fn sync_to_highest_quorum_cert(
        &self,
        highest_quorum_cert: QuorumCert,
        highest_commit_cert: WrappedLedgerInfo,
        retriever: &mut BlockRetriever,
    ) -> anyhow::Result<()> {
        if !self.need_sync_for_ledger_info(highest_commit_cert.ledger_info()) {
            return Ok(());
        }

        let (root, root_metadata, blocks, quorum_certs) = Self::fast_forward_sync(
            &highest_quorum_cert,
            &highest_commit_cert,
            retriever,
            self.storage.clone(),
            self.execution_client.clone(),
            self.payload_manager.clone(),
            self.order_vote_enabled,
            self.window_size,
        )
        .await?
        .take();
        info!(
            LogSchema::new(LogEvent::CommitViaSync).round(self.ordered_root().round()),
            committed_round = root.commit_root_block.round(),
            block_id = root.commit_root_block.id(),
        );
        self.rebuild(root, root_metadata, blocks, quorum_certs)
            .await;

        if highest_commit_cert.ledger_info().ledger_info().ends_epoch() {
            retriever
                .network
                .send_epoch_change(EpochChangeProof::new(
                    vec![highest_quorum_cert.ledger_info().clone()],
                    /* more = */ false,
                ))
                .await;
        }
        Ok(())
    }

    // If execution pool is enabled, use round based block retrieval, else use target block id
    pub(crate) fn generate_target_block_retrieval_payload_and_num_blocks<'a>(
        highest_quorum_cert: &'a QuorumCert,
        highest_commit_cert: &'a WrappedLedgerInfo,
        window_size: Option<u64>,
    ) -> (TargetBlockRetrieval, u64) {
        match window_size {
            None => {
                let num_blocks = highest_quorum_cert.certified_block().round()
                    - highest_commit_cert.ledger_info().ledger_info().round()
                    + 1;
                let target_block_id = highest_commit_cert.commit_info().id();
                info!(
                    "[FastForwardSync] with window_size: None, target_block_id: {}, num_blocks: {}",
                    target_block_id, num_blocks
                );
                (
                    TargetBlockRetrieval::TargetBlockId(target_block_id),
                    num_blocks,
                )
            },
            Some(window_size) => {
                let target_round = calculate_window_start_round(
                    highest_commit_cert.ledger_info().ledger_info().round(),
                    window_size,
                )
                .max(1); // Never retrieve genesis block
                let num_blocks = highest_quorum_cert.certified_block().round() - target_round + 1;
                info!(
                    "[FastForwardSync] with window_size: {}, target_round: {}, num_blocks: {}",
                    window_size, target_round, num_blocks
                );
                (TargetBlockRetrieval::TargetRound(target_round), num_blocks)
            },
        }
    }

    pub async fn fast_forward_sync<'a>(
        highest_quorum_cert: &'a QuorumCert,
        highest_commit_cert: &'a WrappedLedgerInfo,
        retriever: &'a mut BlockRetriever,
        storage: Arc<dyn PersistentLivenessStorage>,
        execution_client: Arc<dyn TExecutionClient>,
        payload_manager: Arc<dyn TPayloadManager>,
        order_vote_enabled: bool,
        window_size: Option<u64>,
    ) -> anyhow::Result<RecoveryData> {
        info!(
            LogSchema::new(LogEvent::StateSync).remote_peer(retriever.preferred_peer),
            "Start state sync to commit cert: {}, quorum cert: {}",
            highest_commit_cert,
            highest_quorum_cert,
        );

        let (target_block_retrieval_payload, num_blocks) =
            Self::generate_target_block_retrieval_payload_and_num_blocks(
                highest_quorum_cert,
                highest_commit_cert,
                window_size,
            );

        // although unlikely, we might wrap num_blocks around on a 32-bit machine
        assert!(num_blocks < std::usize::MAX as u64);

        BLOCKS_FETCHED_FROM_NETWORK_WHILE_FAST_FORWARD_SYNC.inc_by(num_blocks);
        let mut blocks = retriever
            .retrieve_blocks_in_range(
                highest_quorum_cert.certified_block().id(),
                num_blocks,
                target_block_retrieval_payload,
                highest_quorum_cert
                    .ledger_info()
                    .get_voters(&retriever.validator_addresses()),
            )
            .await?;

        let mut quorum_certs = vec![highest_quorum_cert.clone()];
        quorum_certs.extend(
            blocks
                .iter()
                .take(blocks.len() - 1)
                .map(|block| block.quorum_cert().clone()),
        );

        if !order_vote_enabled {
            // TODO: this is probably still necessary, but need to think harder, it's pretty subtle
            // check if highest_commit_cert comes from a fork
            // if so, we need to fetch it's block as well, to have a proof of commit.
            let highest_commit_certified_block =
                highest_commit_cert.certified_block(order_vote_enabled)?;
            if !blocks
                .iter()
                .any(|block| block.id() == highest_commit_certified_block.id())
            {
                info!(
                    "Found forked QC {}, fetching it as well",
                    highest_commit_cert
                );
                BLOCKS_FETCHED_FROM_NETWORK_WHILE_FAST_FORWARD_SYNC.inc_by(1);

                // Only retrieving one block here, we can simply use TargetBlockRetrieval::TargetBlockId
                let target_block_retrieval_payload =
                    TargetBlockRetrieval::TargetBlockId(highest_commit_certified_block.id());
                let mut additional_blocks = retriever
                    .retrieve_blocks_in_range(
                        highest_commit_certified_block.id(),
                        1,
                        target_block_retrieval_payload,
                        highest_commit_cert
                            .ledger_info()
                            .get_voters(&retriever.validator_addresses()),
                    )
                    .await?;

                assert_eq!(additional_blocks.len(), 1);
                let block = additional_blocks.pop().expect("blocks are empty");
                assert_eq!(
                    block.id(),
                    highest_commit_certified_block.id(),
                    "Expecting in the retrieval response, for commit certificate fork, first block should be {}, but got {}",
                    highest_commit_certified_block.id(),
                    block.id(),
                );
                blocks.push(block);
                quorum_certs.push(
                    highest_commit_cert
                        .clone()
                        .into_quorum_cert(order_vote_enabled)?,
                );
            }
        }

        assert_eq!(blocks.len(), quorum_certs.len());
        info!("[FastForwardSync] Fetched {} blocks. Requested num_blocks {}. Initial block hash {:?}, target block hash {:?}",
            blocks.len(), num_blocks, highest_quorum_cert.certified_block().id(), highest_commit_cert.commit_info().id()
        );
        for (i, block) in blocks.iter().enumerate() {
            assert_eq!(block.id(), quorum_certs[i].certified_block().id());
            if let Some(payload) = block.payload() {
                payload_manager.prefetch_payload_data(
                    payload,
                    block.author().expect("payload block must have author"),
                    block.timestamp_usecs(),
                );
            }
        }

        // Check early that recovery will succeed, and return before corrupting our state in case it will not.
        LedgerRecoveryData::new(highest_commit_cert.ledger_info().clone())
            .find_root(
                &mut blocks.clone(),
                &mut quorum_certs.clone(),
                order_vote_enabled,
                window_size,
            )
            .with_context(|| {
                // for better readability
                quorum_certs.sort_by_key(|qc| qc.certified_block().round());
                format!(
                    "\nRoot: {:?}\nBlocks in db: {}\nQuorum Certs in db: {}\n",
                    highest_commit_cert.commit_info(),
                    blocks
                        .iter()
                        .map(|b| format!("\n\t{}", b))
                        .collect::<Vec<String>>()
                        .concat(),
                    quorum_certs
                        .iter()
                        .map(|qc| format!("\n\t{}", qc))
                        .collect::<Vec<String>>()
                        .concat(),
                )
            })?;

        storage.save_tree(blocks.clone(), quorum_certs.clone())?;

        execution_client
            .sync_to_target(highest_commit_cert.ledger_info().clone())
            .await?;

        // we do not need to update block_tree.highest_commit_decision_ledger_info here
        // because the block_tree is going to rebuild itself.

        let recovery_data = match storage.start(order_vote_enabled, window_size) {
            LivenessStorageData::FullRecoveryData(recovery_data) => recovery_data,
            _ => panic!("Failed to construct recovery data after fast forward sync"),
        };

        Ok(recovery_data)
    }

    /// Fast forward in the decoupled-execution pipeline if the block exists there
    async fn sync_to_highest_commit_cert(
        &self,
        ledger_info: &LedgerInfoWithSignatures,
        network: Arc<NetworkSender>,
    ) {
        // if the block exists between commit root and ordered root
        if self.commit_root().round() < ledger_info.commit_info().round()
            && self.block_exists(ledger_info.commit_info().id())
            && self.ordered_root().round() >= ledger_info.commit_info().round()
        {
            let proof = ledger_info.clone();
            tokio::spawn(async move { network.send_commit_proof(proof).await });
        }
    }

    pub async fn process_block_retrieval_inner(
        &self,
        request: &BlockRetrievalRequest,
    ) -> Box<BlockRetrievalResponse> {
        let mut blocks = vec![];
        let mut status = BlockRetrievalStatus::Succeeded;
        let mut id = request.block_id();

        match &request {
            BlockRetrievalRequest::V1(req) => {
                while (blocks.len() as u64) < req.num_blocks() {
                    if let Some(executed_block) = self.get_block(id) {
                        blocks.push(executed_block.block().clone());
                        if req.match_target_id(id) {
                            status = BlockRetrievalStatus::SucceededWithTarget;
                            break;
                        }
                        id = executed_block.parent_id();
                    } else {
                        status = BlockRetrievalStatus::NotEnoughBlocks;
                        break;
                    }
                }
            },
            BlockRetrievalRequest::V2(req) => {
                while (blocks.len() as u64) < req.num_blocks() {
                    if let Some(executed_block) = self.get_block(id) {
                        if !executed_block.block().is_genesis_block() {
                            blocks.push(executed_block.block().clone());
                        }
                        if req.is_window_start_block(executed_block.block()) {
                            status = BlockRetrievalStatus::SucceededWithTarget;
                            break;
                        }
                        id = executed_block.parent_id();
                    } else {
                        status = BlockRetrievalStatus::NotEnoughBlocks;
                        break;
                    }
                }
            },
        }

        if blocks.is_empty() {
            status = BlockRetrievalStatus::IdNotFound;
        }

        Box::new(BlockRetrievalResponse::new(status, blocks))
    }

    /// Retrieve a n chained blocks from the block store starting from
    /// an initial parent id, returning with <n (as many as possible) if
    /// id or its ancestors can not be found.
    ///
    /// The current version of the function is not really async, but keeping it this way for
    /// future possible changes.
    pub async fn process_block_retrieval(
        &self,
        request: IncomingBlockRetrievalRequest,
    ) -> anyhow::Result<()> {
        fail_point!("consensus::process_block_retrieval", |_| {
            Err(anyhow::anyhow!("Injected error in process_block_retrieval"))
        });
        let response = self.process_block_retrieval_inner(&request.req).await;
        let response_bytes = request
            .protocol
            .to_bytes(&ConsensusMsg::BlockRetrievalResponse(response))?;
        request
            .response_sender
            .send(Ok(response_bytes.into()))
            .map_err(|_| anyhow::anyhow!("Failed to send block retrieval response"))
    }
}

/// BlockRetriever is used internally to retrieve blocks
pub struct BlockRetriever {
    network: Arc<NetworkSender>,
    preferred_peer: Author,
    validator_addresses: Vec<AccountAddress>,
    max_blocks_to_request: u64,
    pending_blocks: Arc<Mutex<PendingBlocks>>,
}

/// When execution pool is on, use `TargetRound` variant, otherwise use `TargetBlockId`
#[derive(Clone, Copy, Debug)]
pub enum TargetBlockRetrieval {
    TargetBlockId(HashValue),
    TargetRound(u64),
}

impl Display for TargetBlockRetrieval {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            TargetBlockRetrieval::TargetBlockId(id) => {
                write!(f, "TargetBlockRetrieval with id {}", id)
            },
            TargetBlockRetrieval::TargetRound(round) => {
                write!(f, "TargetBlockRetrieval with round {}", round)
            },
        }
    }
}

impl BlockRetriever {
    pub fn new(
        network: Arc<NetworkSender>,
        preferred_peer: Author,
        validator_addresses: Vec<AccountAddress>,
        max_blocks_to_request: u64,
        pending_blocks: Arc<Mutex<PendingBlocks>>,
    ) -> Self {
        Self {
            network,
            preferred_peer,
            validator_addresses,
            max_blocks_to_request,
            pending_blocks,
        }
    }

    pub fn validator_addresses(&self) -> Vec<AccountAddress> {
        self.validator_addresses.clone()
    }

    async fn retrieve_block_chunk(
        &mut self,
        block_id: HashValue,
        target_block_retrieval_payload: TargetBlockRetrieval,
        retrieve_batch_size: u64,
        mut peers: Vec<AccountAddress>,
    ) -> anyhow::Result<BlockRetrievalResponse> {
        let mut failed_attempt = 0_u32;
        let mut cur_retry = 0;

        let num_retries = NUM_RETRIES;
        let request_num_peers = NUM_PEERS_PER_RETRY;
        let retry_interval = Duration::from_millis(RETRY_INTERVAL_MSEC);
        let rpc_timeout = Duration::from_millis(RPC_TIMEOUT_MSEC);

        monitor!("retrieve_block_for_id_chunk", {
            let mut interval = time::interval(retry_interval);
            let mut futures = FuturesUnordered::new();
            if retrieve_batch_size == 1 {
                let (tx, rx) = oneshot::channel();
                self.pending_blocks
                    .lock()
                    .insert_request(target_block_retrieval_payload, tx);
                let author = self.network.author();
                futures.push(
                    async move {
                        let response = match timeout(rpc_timeout, rx).await {
                            Ok(Ok(block)) => Ok(BlockRetrievalResponse::new(
                                BlockRetrievalStatus::SucceededWithTarget,
                                vec![block],
                            )),
                            Ok(Err(_)) => Err(anyhow!("self retrieval cancelled")),
                            Err(_) => Err(anyhow!("self retrieval timeout")),
                        };
                        (author, response)
                    }
                    .boxed(),
                )
            }
            let request = match target_block_retrieval_payload {
                TargetBlockRetrieval::TargetBlockId(target_block_id) => {
                    BlockRetrievalRequest::V1(BlockRetrievalRequestV1::new_with_target_block_id(
                        block_id,
                        retrieve_batch_size,
                        target_block_id,
                    ))
                },
                TargetBlockRetrieval::TargetRound(target_round) => {
                    BlockRetrievalRequest::V2(BlockRetrievalRequestV2::new_with_target_round(
                        block_id,
                        retrieve_batch_size,
                        target_round,
                    ))
                },
            };

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        // send batch request to a set of peers of size request_num_peers (or 1 for the first time)
                        let next_peers = if cur_retry < num_retries {
                            let first_attempt = cur_retry == 0;
                            cur_retry += 1;
                            self.pick_peers(
                                first_attempt,
                                &mut peers,
                                if first_attempt { 1 } else {request_num_peers}
                            )
                        } else {
                            Vec::new()
                        };

                        if next_peers.is_empty() && futures.is_empty() {
                            bail!("Couldn't fetch block")
                        }

                        for peer in next_peers {
                            debug!(
                                LogSchema::new(LogEvent::RetrieveBlock).remote_peer(peer),
                                block_id = block_id,
                                "Fetching {} blocks, retry {}, failed attempts {}",
                                retrieve_batch_size,
                                cur_retry,
                                failed_attempt
                            );
                            let remote_peer = peer;
                            let future = self.network.request_block(
                                request.clone(),
                                peer,
                                rpc_timeout,
                            );
                            futures.push(async move { (remote_peer, future.await) }.boxed());
                        }
                    }
                    Some((peer, response)) = futures.next() => {
                        match response {
                            Ok(result) => return Ok(result),
                            e => {
                                warn!(
                                    remote_peer = peer,
                                    block_id = block_id,
                                    "{:?}, Failed to fetch block",
                                    e,
                                );
                                failed_attempt += 1;
                            },
                        }
                    },
                }
            }
        })
    }

    /// Retrieve n blocks for given block_id from peers
    ///
    /// Returns Result with Vec that if succeeded. This method will
    /// continue until the quorum certificate members all fail to return the missing chain.
    ///
    /// The first attempt of block retrieval will always be sent to preferred_peer to allow the
    /// leader to drive quorum certificate creation The other peers from the quorum certificate
    /// will be randomly tried next.  If all members of the quorum certificate are exhausted, an
    /// error is returned
    async fn retrieve_blocks(
        &mut self,
        block_id: HashValue,
        target_block_retrieval_payload: TargetBlockRetrieval,
        peers: Vec<AccountAddress>,
        num_blocks: u64,
    ) -> anyhow::Result<Vec<Block>> {
        match &target_block_retrieval_payload {
            TargetBlockRetrieval::TargetBlockId(target_block_id) => {
                info!(
                    "Retrieving {} blocks starting from {} with target_block_id {}",
                    num_blocks, block_id, target_block_id
                );
            },
            TargetBlockRetrieval::TargetRound(target_round) => {
                info!(
                    "Retrieving {} blocks starting from {} with target_round {}",
                    num_blocks, block_id, target_round
                );
            },
        }

        let mut progress = 0;
        let mut last_block_id = block_id;
        let mut result_blocks: Vec<Block> = vec![];
        let mut retrieve_batch_size = self.max_blocks_to_request;
        if peers.is_empty() {
            bail!("Failed to fetch block {}: no peers available", block_id);
        }
        while progress < num_blocks {
            // in case this is the last retrieval
            retrieve_batch_size = min(retrieve_batch_size, num_blocks - progress);

            info!(
                "Retrieving chunk: {} blocks starting from {}, original start {}",
                retrieve_batch_size, last_block_id, block_id
            );

            let response = self
                .retrieve_block_chunk(
                    last_block_id,
                    target_block_retrieval_payload,
                    retrieve_batch_size,
                    peers.clone(),
                )
                .await;
            match response {
                Ok(result) if matches!(result.status(), BlockRetrievalStatus::Succeeded) => {
                    // extend the result blocks
                    let batch = result.blocks().clone();
                    progress += batch.len() as u64;
                    last_block_id = batch.last().expect("Batch should not be empty").parent_id();
                    result_blocks.extend(batch);
                },
                Ok(result)
                    if matches!(result.status(), BlockRetrievalStatus::SucceededWithTarget) =>
                {
                    // if we found the target, end the loop
                    let batch = result.blocks().clone();
                    result_blocks.extend(batch);
                    break;
                },
                res => {
                    bail!(
                        "Failed to fetch block {}, for original start {}, returned status {:?}",
                        last_block_id,
                        block_id,
                        res
                    );
                },
            }
        }

        // Confirm retrieval hit the first block we care about
        assert_eq!(
            result_blocks.first().expect("blocks are empty").id(),
            block_id,
            "Expecting in the retrieval response, first block should be {}, but got {}",
            block_id,
            result_blocks.first().expect("blocks are empty").id(),
        );

        // Confirm retrieval hit the last block/round we care about
        // Slightly different logic if using execution pool and not
        match target_block_retrieval_payload {
            TargetBlockRetrieval::TargetBlockId(target_block_id) => {
                ensure!(
                    result_blocks
                        .last()
                        .expect("Expected at least a result_block")
                        .id()
                        == target_block_id
                );
            },
            TargetBlockRetrieval::TargetRound(target_round) => {
                let last_block = result_blocks.last().expect("blocks are empty");
                ensure!(
                    last_block.round() == target_round || last_block.quorum_cert().certified_block().round() < target_round,
                    "Expecting in the retrieval response, last block should be == {} or its parent should be < {}, but got {} and parent {}",
                    target_round,
                    target_round,
                    last_block.round(),
                    last_block.quorum_cert().certified_block().round(),
                );
            },
        }

        Ok(result_blocks)
    }

    /// Retrieve chain of n blocks for given QC
    async fn retrieve_blocks_in_range(
        &mut self,
        initial_block_id: HashValue,
        num_blocks: u64,
        target_block_retrieval_payload: TargetBlockRetrieval,
        peers: Vec<AccountAddress>,
    ) -> anyhow::Result<Vec<Block>> {
        BLOCKS_FETCHED_FROM_NETWORK_IN_BLOCK_RETRIEVER.inc_by(num_blocks);
        self.retrieve_blocks(
            initial_block_id,
            target_block_retrieval_payload,
            peers,
            num_blocks,
        )
        .await
    }

    fn pick_peer(&self, first_atempt: bool, peers: &mut Vec<AccountAddress>) -> AccountAddress {
        assert!(!peers.is_empty(), "pick_peer on empty peer list");

        if first_atempt {
            // remove preferred_peer if its in list of peers
            // (strictly speaking it is not required to be there)
            for i in 0..peers.len() {
                if peers[i] == self.preferred_peer {
                    peers.remove(i);
                    break;
                }
            }
            return self.preferred_peer;
        }

        let peer_idx = thread_rng().gen_range(0, peers.len());
        peers.remove(peer_idx)
    }

    fn pick_peers(
        &self,
        first_atempt: bool,
        peers: &mut Vec<AccountAddress>,
        request_num_peers: usize,
    ) -> Vec<AccountAddress> {
        let mut result = Vec::new();
        while !peers.is_empty() && result.len() < request_num_peers {
            result.push(self.pick_peer(first_atempt && result.is_empty(), peers));
        }
        result
    }
}
