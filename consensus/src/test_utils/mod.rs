// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::unwrap_used)]
use crate::{
    block_storage::{BlockReader, BlockStore},
    liveness::{
        proposal_status_tracker::{TOptQSPullParamsProvider, TPastProposalStatusTracker},
        round_state::NewRoundReason,
    },
    payload_manager::DirectMempoolPayloadManager,
};
use aptos_consensus_types::{
    block::{block_test_utils::certificate_for_genesis, Block},
    common::{Author, Round},
    payload_pull_params::OptQSPayloadPullParams,
    pipelined_block::PipelinedBlock,
    quorum_cert::QuorumCert,
    sync_info::SyncInfo,
};
use aptos_crypto::{HashValue, PrivateKey, Uniform};
use aptos_logger::Level;
use aptos_types::{ledger_info::LedgerInfo, validator_signer::ValidatorSigner};
use std::{future::Future, sync::Arc, time::Duration};
use tokio::{runtime, time::timeout};

#[cfg(test)]
pub mod mock_execution_client;
#[cfg(any(test, feature = "fuzzing"))]
mod mock_payload_manager;
pub mod mock_quorum_store_sender;
mod mock_state_computer;
mod mock_storage;

use crate::{
    block_storage::pending_blocks::PendingBlocks,
    pipeline::{execution_client::DummyExecutionClient, pipeline_builder::PipelineBuilder},
    util::mock_time_service::SimulatedTimeService,
};
use aptos_consensus_types::{block::block_test_utils::gen_test_certificate, common::Payload};
use aptos_crypto::ed25519::{Ed25519PrivateKey, Ed25519Signature};
use aptos_infallible::Mutex;
use aptos_types::{
    block_info::BlockInfo,
    chain_id::ChainId,
    on_chain_config::DEFAULT_ENABLED_WINDOW_SIZE,
    transaction::{RawTransaction, Script, SignedTransaction, TransactionPayload},
};
pub use mock_payload_manager::MockPayloadManager;
#[cfg(test)]
pub use mock_state_computer::EmptyStateComputer;
#[cfg(test)]
pub use mock_state_computer::RandomComputeResultStateComputer;
pub use mock_storage::{EmptyStorage, MockStorage};
use move_core_types::account_address::AccountAddress;

pub const TEST_TIMEOUT: Duration = Duration::from_secs(60);

pub async fn build_simple_tree() -> (Vec<Arc<PipelinedBlock>>, Arc<BlockStore>) {
    let mut inserter = TreeInserter::default();
    let block_store = inserter.block_store();
    let genesis = block_store.ordered_root();
    let genesis_block_id = genesis.id();
    let genesis_block = block_store
        .get_block(genesis_block_id)
        .expect("genesis block must exist");
    assert_eq!(block_store.len(), 1);
    assert_eq!(block_store.child_links(), block_store.len() - 1);
    assert!(block_store.block_exists(genesis_block.id()));

    //       ╭--> A1--> A2--> A3
    // Genesis--> B1--> B2
    //             ╰--> C1
    let a1 = inserter
        .insert_block_with_qc(certificate_for_genesis(), &genesis_block, 1)
        .await;
    let a2 = inserter.insert_block(&a1, 2, None).await;
    let a3 = inserter.insert_block(&a2, 3, None).await;
    let b1 = inserter
        .insert_block_with_qc(certificate_for_genesis(), &genesis_block, 4)
        .await;
    let b2 = inserter.insert_block(&b1, 5, None).await;
    let c1 = inserter.insert_block(&b1, 6, None).await;

    assert_eq!(block_store.len(), 7);
    assert_eq!(block_store.child_links(), block_store.len() - 1);

    (vec![genesis_block, a1, a2, a3, b1, b2, c1], block_store)
}

fn build_empty_tree_inner(
    window_size: Option<u64>,
    max_pruned_blocks_in_mem: usize,
    pipeline_builder: Option<PipelineBuilder>,
) -> BlockStore {
    let (initial_data, storage) = EmptyStorage::start_for_testing();
    BlockStore::new(
        storage,
        initial_data,
        Arc::new(DummyExecutionClient),
        max_pruned_blocks_in_mem, // max pruned blocks in mem
        Arc::new(SimulatedTimeService::new()),
        10,
        Arc::from(DirectMempoolPayloadManager::new()),
        false,
        window_size,
        Arc::new(Mutex::new(PendingBlocks::new())),
        pipeline_builder,
    )
}

pub fn build_default_empty_tree() -> Arc<BlockStore> {
    let window_size = DEFAULT_ENABLED_WINDOW_SIZE;
    let max_pruned_blocks_in_mem: usize = 10;
    Arc::new(build_empty_tree_inner(
        window_size,
        max_pruned_blocks_in_mem,
        None,
    ))
}

pub fn build_custom_empty_tree(
    window_size: Option<u64>,
    max_pruned_blocks_in_mem: usize,
    pipeline_builder: Option<PipelineBuilder>,
) -> Arc<BlockStore> {
    let block_store =
        build_empty_tree_inner(window_size, max_pruned_blocks_in_mem, pipeline_builder);
    Arc::new(block_store)
}

pub struct TreeInserter {
    signer: ValidatorSigner,
    block_store: Arc<BlockStore>,
}

impl TreeInserter {
    pub fn default() -> Self {
        Self::new(ValidatorSigner::random(None))
    }

    pub fn new(signer: ValidatorSigner) -> Self {
        let block_store = build_default_empty_tree();
        Self {
            signer,
            block_store,
        }
    }

    pub fn new_with_params(
        signer: ValidatorSigner,
        window_size: Option<u64>,
        max_pruned_blocks_in_mem: usize,
        pipeline_builder: Option<PipelineBuilder>,
    ) -> Self {
        let block_store =
            build_custom_empty_tree(window_size, max_pruned_blocks_in_mem, pipeline_builder);
        Self {
            signer,
            block_store,
        }
    }

    pub fn new_with_store(signer: ValidatorSigner, block_store: Arc<BlockStore>) -> Self {
        Self {
            signer,
            block_store,
        }
    }

    pub fn signer(&self) -> &ValidatorSigner {
        &self.signer
    }

    pub fn block_store(&self) -> Arc<BlockStore> {
        Arc::clone(&self.block_store)
    }

    /// This function is generating a placeholder QC for a block's parent that is signed by a single
    /// signer kept by the block store. If more sophisticated QC required, please use
    /// `insert_block_with_qc`.
    pub async fn insert_block(
        &mut self,
        parent: &PipelinedBlock,
        round: Round,
        committed_block: Option<BlockInfo>,
    ) -> Arc<PipelinedBlock> {
        // Node must carry a QC to its parent
        let parent_qc = self.create_qc_for_block(parent, committed_block);
        self.insert_block_with_qc(parent_qc, parent, round).await
    }

    pub async fn insert_nil_block(
        &mut self,
        parent: &PipelinedBlock,
        round: Round,
        committed_block: Option<BlockInfo>,
    ) -> Arc<PipelinedBlock> {
        // Node must carry a QC to its parent
        let parent_qc = self.create_qc_for_block(parent, committed_block);
        self.insert_nil_block_with_qc(parent_qc, round).await
    }

    pub async fn insert_block_with_qc(
        &mut self,
        parent_qc: QuorumCert,
        parent: &PipelinedBlock,
        round: Round,
    ) -> Arc<PipelinedBlock> {
        self.block_store
            .insert_block_with_qc(self.create_block_with_qc(
                parent_qc,
                parent.timestamp_usecs() + 1,
                round,
                Payload::empty(false, true),
                vec![],
            ))
            .await
            .unwrap()
    }

    pub async fn insert_nil_block_with_qc(
        &mut self,
        parent_qc: QuorumCert,
        round: Round,
    ) -> Arc<PipelinedBlock> {
        self.block_store
            .insert_block_with_qc(self.create_nil_block_with_qc(round, parent_qc, vec![]))
            .await
            .unwrap()
    }

    pub fn create_qc_for_block(
        &self,
        block: &PipelinedBlock,
        committed_block: Option<BlockInfo>,
    ) -> QuorumCert {
        gen_test_certificate(
            &[self.signer.clone()],
            block.block_info(),
            block.quorum_cert().certified_block().clone(),
            committed_block,
        )
    }

    pub fn insert_qc_for_block(&self, block: &PipelinedBlock, committed_block: Option<BlockInfo>) {
        self.block_store
            .insert_single_quorum_cert(self.create_qc_for_block(block, committed_block))
            .unwrap()
    }

    pub fn create_block_with_qc(
        &self,
        parent_qc: QuorumCert,
        timestamp_usecs: u64,
        round: Round,
        payload: Payload,
        failed_authors: Vec<(Round, Author)>,
    ) -> Block {
        Block::new_proposal(
            payload,
            round,
            timestamp_usecs,
            parent_qc,
            &self.signer,
            failed_authors,
        )
        .unwrap()
    }

    pub fn create_nil_block_with_qc(
        &self,
        round: Round,
        quorum_cert: QuorumCert,
        failed_authors: Vec<(Round, Author)>,
    ) -> Block {
        Block::new_nil(round, quorum_cert, failed_authors)
    }
}

pub fn placeholder_ledger_info() -> LedgerInfo {
    LedgerInfo::new(BlockInfo::empty(), HashValue::zero())
}

pub fn placeholder_sync_info() -> SyncInfo {
    SyncInfo::new(
        certificate_for_genesis(),
        certificate_for_genesis().into_wrapped_ledger_info(),
        None,
    )
}

fn nocapture() -> bool {
    ::std::env::args().any(|arg| arg == "--nocapture")
}

pub fn consensus_runtime() -> runtime::Runtime {
    if nocapture() {
        ::aptos_logger::Logger::new().level(Level::Debug).init();
    }

    aptos_runtimes::spawn_named_runtime("consensus".into(), None)
}

pub fn timed_block_on<F>(runtime: &runtime::Runtime, f: F) -> <F as Future>::Output
where
    F: Future,
{
    runtime
        .block_on(async { timeout(TEST_TIMEOUT, f).await })
        .expect("test timed out")
}

// Creates a single test transaction for a random account
pub(crate) fn create_signed_transaction(gas_unit_price: u64) -> SignedTransaction {
    let private_key = Ed25519PrivateKey::generate_for_testing();
    let public_key = private_key.public_key();

    // TODO[Orderless]: Change this to transaction payload v2 format.
    let transaction_payload = TransactionPayload::Script(Script::new(vec![], vec![], vec![]));
    let raw_transaction = RawTransaction::new(
        AccountAddress::random(),
        0,
        transaction_payload,
        0,
        gas_unit_price,
        0,
        ChainId::new(10),
    );
    SignedTransaction::new(
        raw_transaction,
        public_key,
        Ed25519Signature::dummy_signature(),
    )
}

pub(crate) fn create_vec_signed_transactions(size: u64) -> Vec<SignedTransaction> {
    (0..size).map(|_| create_signed_transaction(1)).collect()
}

pub(crate) fn create_vec_signed_transactions_with_gas(
    size: u64,
    gas_unit_price: u64,
) -> Vec<SignedTransaction> {
    (0..size)
        .map(|_| create_signed_transaction(gas_unit_price))
        .collect()
}

pub struct MockOptQSPayloadProvider {}

impl TOptQSPullParamsProvider for MockOptQSPayloadProvider {
    fn get_params(&self) -> Option<OptQSPayloadPullParams> {
        None
    }
}

pub struct MockPastProposalStatusTracker {}

impl TPastProposalStatusTracker for MockPastProposalStatusTracker {
    fn push(&self, _status: NewRoundReason) {}
}
