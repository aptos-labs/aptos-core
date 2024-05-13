// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block_storage::{
        block_retriever::{BlockRetriever, RetrieverResult},
        BlockReader, BlockStore,
    },
    epoch_manager::LivenessStorageData,
    logging::{LogEvent, LogSchema},
    network::{IncomingBlockRetrievalRequest, NetworkSender},
    network_interface::ConsensusMsg,
    payload_manager::PayloadManager,
    persistent_liveness_storage::{LedgerRecoveryData, PersistentLivenessStorage, RecoveryData},
    pipeline::execution_client::TExecutionClient,
    round_manager::SyncResult,
};
use anyhow::Context;
use aptos_consensus_types::{
    block_retrieval::{BlockRetrievalResponse, BlockRetrievalStatus},
    quorum_cert::QuorumCert,
    sync_info::SyncInfo,
};
use aptos_logger::prelude::*;
use aptos_types::{epoch_change::EpochChangeProof, ledger_info::LedgerInfoWithSignatures};
use fail::fail_point;
use std::{clone::Clone, sync::Arc};

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
        // TODO move min gap to fallback (30) to config.
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
    ) -> anyhow::Result<SyncResult> {
        self.sync_to_highest_commit_cert(
            sync_info.highest_commit_cert().ledger_info(),
            &retriever.network(),
        )
        .await;
        if SyncResult::Fetching
            == self
                .sync_to_highest_ordered_cert(
                    sync_info.highest_ordered_cert().clone(),
                    sync_info.highest_commit_cert().clone(),
                    &mut retriever,
                )
                .await?
        {
            return Ok(SyncResult::Fetching);
        }

        if SyncResult::Fetching
            == self
                .insert_quorum_cert(sync_info.highest_ordered_cert(), &mut retriever)
                .await?
        {
            return Ok(SyncResult::Fetching);
        }

        if SyncResult::Fetching
            == self
                .insert_quorum_cert(sync_info.highest_quorum_cert(), &mut retriever)
                .await?
        {
            return Ok(SyncResult::Fetching);
        }

        if let Some(tc) = sync_info.highest_2chain_timeout_cert() {
            self.insert_2chain_timeout_certificate(Arc::new(tc.clone()))?;
        }
        Ok(SyncResult::Success)
    }

    pub async fn insert_quorum_cert(
        &self,
        qc: &QuorumCert,
        retriever: &mut BlockRetriever,
    ) -> anyhow::Result<SyncResult> {
        match self.need_fetch_for_quorum_cert(qc) {
            NeedFetchResult::NeedFetch => {
                if SyncResult::Fetching == self.fetch_quorum_cert(qc.clone(), retriever).await? {
                    return Ok(SyncResult::Fetching);
                }
            },
            NeedFetchResult::QCBlockExist => self.insert_single_quorum_cert(qc.clone())?,
            NeedFetchResult::QCAlreadyExist => return Ok(SyncResult::Success),
            _ => (),
        }
        if self.ordered_root().round() < qc.commit_info().round() {
            self.send_for_execution(qc.clone()).await?;
            if qc.ends_epoch() {
                retriever
                    .network()
                    .broadcast_epoch_change(EpochChangeProof::new(
                        vec![qc.ledger_info().clone()],
                        /* more = */ false,
                    ))
                    .await;
            }
        }
        Ok(SyncResult::Success)
    }

    /// Insert the quorum certificate separately from the block, used to split the processing of
    /// updating the consensus state(with qc) and deciding whether to vote(with block)
    /// The missing ancestors are going to be retrieved from the given peer. If a given peer
    /// fails to provide the missing ancestors, the qc is not going to be added.
    async fn fetch_quorum_cert(
        &self,
        qc: QuorumCert,
        retriever: &mut BlockRetriever,
    ) -> anyhow::Result<SyncResult> {
        let mut pending = vec![];
        let mut retrieve_qc = qc.clone();
        loop {
            if self.block_exists(retrieve_qc.certified_block().id()) {
                break;
            }
            match retriever
                .retrieve_block_for_qc(&retrieve_qc, 1, retrieve_qc.certified_block().id())
                .await?
            {
                RetrieverResult::Blocks(mut blocks) => {
                    // retrieve_block_for_qc guarantees that blocks has exactly 1 element
                    let block = blocks.remove(0);
                    retrieve_qc = block.quorum_cert().clone();
                    pending.push(block);
                },
                RetrieverResult::Fetching => {
                    return Ok(SyncResult::Fetching);
                },
            }
        }
        // insert the qc <- block pair
        while let Some(block) = pending.pop() {
            let block_qc = block.quorum_cert().clone();
            self.insert_single_quorum_cert(block_qc)?;
            self.insert_ordered_block(block).await?;
        }
        self.insert_single_quorum_cert(qc)?;
        Ok(SyncResult::Success)
    }

    /// Check the highest ordered cert sent by peer to see if we're behind and start a fast
    /// forward sync if the committed block doesn't exist in our tree.
    /// It works as follows:
    /// 1. request the gap blocks from the peer (from highest_ledger_info to highest_ordered_cert)
    /// 2. We persist the gap blocks to storage before start sync to ensure we could restart if we
    /// crash in the middle of the sync.
    /// 3. We prune the old tree and replace with a new tree built with the 3-chain.
    async fn sync_to_highest_ordered_cert(
        &self,
        highest_ordered_cert: QuorumCert,
        highest_commit_cert: QuorumCert,
        retriever: &mut BlockRetriever,
    ) -> anyhow::Result<SyncResult> {
        if !self.need_sync_for_ledger_info(highest_commit_cert.ledger_info()) {
            return Ok(SyncResult::Success);
        }
        match Self::fast_forward_sync(
            &highest_ordered_cert,
            &highest_commit_cert,
            retriever,
            self.storage.clone(),
            self.execution_client.clone(),
            self.payload_manager.clone(),
        )
        .await?
        {
            Some(recovery_data) => {
                let (root, root_metadata, blocks, quorum_certs) = recovery_data.take();
                info!(
                    LogSchema::new(LogEvent::CommitViaSync).round(self.ordered_root().round()),
                    committed_round = root.0.round(),
                    block_id = root.0.id(),
                );
                self.rebuild(root, root_metadata, blocks, quorum_certs)
                    .await;

                if highest_commit_cert.ledger_info().ledger_info().ends_epoch() {
                    retriever
                        .network()
                        .send_epoch_change(EpochChangeProof::new(
                            // Question: If highest_commit_cert has ends_epoch() == true, why are we sending
                            // highest_ordered_cert instead of highest_commit_cert?
                            vec![highest_ordered_cert.ledger_info().clone()],
                            /* more = */ false,
                        ))
                        .await;
                }
                Ok(SyncResult::Success)
            },
            None => Ok(SyncResult::Fetching),
        }
    }

    pub async fn fast_forward_sync<'a>(
        highest_ordered_cert: &'a QuorumCert,
        highest_commit_cert: &'a QuorumCert,
        retriever: &'a mut BlockRetriever,
        storage: Arc<dyn PersistentLivenessStorage>,
        execution_client: Arc<dyn TExecutionClient>,
        payload_manager: Arc<PayloadManager>,
    ) -> anyhow::Result<Option<RecoveryData>> {
        info!(
            LogSchema::new(LogEvent::StateSync).remote_peer(retriever.preferred_peer()),
            "Start state sync to commit cert: {}, ordered cert: {}",
            highest_commit_cert,
            highest_ordered_cert,
        );

        // we fetch the blocks from
        let num_blocks = highest_ordered_cert.certified_block().round()
            - highest_commit_cert.ledger_info().ledger_info().round()
            + 1;

        // although unlikely, we might wrap num_blocks around on a 32-bit machine
        assert!(num_blocks < std::usize::MAX as u64);

        if let RetrieverResult::Blocks(blocks) = retriever
            .retrieve_block_for_qc(
                highest_ordered_cert,
                num_blocks,
                highest_commit_cert.commit_info().id(),
            )
            .await?
        {
            assert_eq!(
                blocks.first().expect("blocks are empty").id(),
                highest_ordered_cert.certified_block().id(),
                "Expecting in the retrieval response, first block should be {}, but got {}",
                highest_ordered_cert.certified_block().id(),
                blocks.first().expect("blocks are empty").id(),
            );

            // Confirm retrieval ended when it hit the last block we care about, even if it didn't reach all num_blocks blocks.
            assert_eq!(
                blocks.last().expect("blocks are empty").id(),
                highest_commit_cert.commit_info().id()
            );

            let mut quorum_certs = vec![highest_ordered_cert.clone()];
            quorum_certs.extend(
                blocks
                    .iter()
                    .take(blocks.len() - 1)
                    .map(|block| block.quorum_cert().clone()),
            );

            // TODO: Is it okay to comment this block of code?

            // // check if highest_commit_cert comes from a fork
            // // if so, we need to fetch it's block as well, to have a proof of commit.
            // if !blocks
            //     .iter()
            //     .any(|block| block.id() == highest_commit_cert.certified_block().id())
            // {
            //     info!(
            //         "Found forked QC {}, fetching it as well",
            //         highest_commit_cert
            //     );
            //     let mut additional_blocks = retriever
            //         .retrieve_block_for_qc(
            //             highest_commit_cert,
            //             1,
            //             highest_commit_cert.certified_block().id(),
            //         )
            //         .await?;

            //     assert_eq!(additional_blocks.len(), 1);
            //     let block = additional_blocks.pop().expect("blocks are empty");
            //     assert_eq!(
            //         block.id(),
            //         highest_commit_cert.certified_block().id(),
            //         "Expecting in the retrieval response, for commit certificate fork, first block should be {}, but got {}",
            //         highest_commit_cert.certified_block().id(),
            //         block.id(),
            //     );

            //     blocks.push(block);
            //     quorum_certs.push(highest_commit_cert.clone());
            // }

            assert_eq!(blocks.len(), quorum_certs.len());
            for (i, block) in blocks.iter().enumerate() {
                assert_eq!(block.id(), quorum_certs[i].certified_block().id());
                if let Some(payload) = block.payload() {
                    payload_manager.prefetch_payload_data(payload, block.timestamp_usecs());
                }
            }

            // Check early that recovery will succeed, and return before corrupting our state in case it will not.
            LedgerRecoveryData::new(highest_commit_cert.ledger_info().clone())
                .find_root(&mut blocks.clone(), &mut quorum_certs.clone())
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
                .sync_to(highest_commit_cert.ledger_info().clone())
                .await?;

            // we do not need to update block_tree.highest_commit_decision_ledger_info here
            // because the block_tree is going to rebuild itself.

            let recovery_data: RecoveryData = match storage.start() {
                LivenessStorageData::FullRecoveryData(recovery_data) => recovery_data,
                _ => panic!("Failed to construct recovery data after fast forward sync"),
            };

            Ok(Some(recovery_data))
        } else {
            Ok(None)
        }
    }

    /// Fast forward in the decoupled-execution pipeline if the block exists there
    async fn sync_to_highest_commit_cert(
        &self,
        ledger_info: &LedgerInfoWithSignatures,
        network: &Arc<NetworkSender>,
    ) {
        // if the block exists between commit root and ordered root
        if self.commit_root().round() < ledger_info.commit_info().round()
            && self.block_exists(ledger_info.commit_info().id())
            && self.ordered_root().round() >= ledger_info.commit_info().round()
        {
            network.send_commit_proof(ledger_info.clone()).await
        }
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
        let mut blocks = vec![];
        let mut status = BlockRetrievalStatus::Succeeded;
        let mut id = request.req.block_id();
        while (blocks.len() as u64) < request.req.num_blocks() {
            if let Some(executed_block) = self.get_block(id) {
                blocks.push(executed_block.block().clone());
                if request.req.match_target_id(id) {
                    status = BlockRetrievalStatus::SucceededWithTarget;
                    break;
                }
                id = executed_block.parent_id();
            } else {
                status = BlockRetrievalStatus::NotEnoughBlocks;
                break;
            }
        }

        if blocks.is_empty() {
            status = BlockRetrievalStatus::IdNotFound;
        }

        let response = Box::new(BlockRetrievalResponse::new(status, blocks));
        let response_bytes = request
            .protocol
            .to_bytes(&ConsensusMsg::BlockRetrievalResponse(response))?;
        request
            .response_sender
            .send(Ok(response_bytes.into()))
            .map_err(|_| anyhow::anyhow!("Failed to send block retrieval response"))
    }
}
