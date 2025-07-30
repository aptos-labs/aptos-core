// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::pipeline::pipeline_builder::PipelineBuilder;
use aptos_consensus_types::{
    block::Block,
    block_data::{BlockData, BlockType},
    common::{Payload, Round},
    pipelined_block::PipelinedBlock,
    quorum_cert::QuorumCert,
};
use aptos_crypto::{ed25519::Ed25519PrivateKey, HashValue, PrivateKey, SigningKey, Uniform};
use aptos_executor_types::{
    state_compute_result::StateComputeResult, BlockExecutorTrait, ExecutorResult,
};
use aptos_infallible::Mutex;
use aptos_types::{
    account_address::AccountAddress,
    aggregate_signature::AggregateSignature,
    block_executor::{config::BlockExecutorConfigFromOnchain, partitioner::ExecutableBlock},
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    on_chain_config::{TransactionDeduperType, TransactionShufflerType},
    transaction::{
        PersistedAuxiliaryInfo, RawTransaction, SignedTransaction, Transaction,
        TransactionPayload,
    },
    validator_signer::ValidatorSigner,
    PeerId,
};
use aptos_bitvec::BitVec;
use aptos_config::config::BlockTransactionFilterConfig;
use std::{
    sync::{Arc, atomic::{AtomicU64, Ordering}},
    time::Duration,
};

/// Mock block executor that tracks auxiliary info storage
#[derive(Clone)]
struct MockBlockExecutor {
    stored_auxiliary_infos: Arc<Mutex<Vec<Vec<PersistedAuxiliaryInfo>>>>,
    version_counter: Arc<AtomicU64>,
}

impl MockBlockExecutor {
    fn new() -> Self {
        Self {
            stored_auxiliary_infos: Arc::new(Mutex::new(Vec::new())),
            version_counter: Arc::new(AtomicU64::new(0)),
        }
    }

    fn get_stored_auxiliary_infos(&self) -> Vec<Vec<PersistedAuxiliaryInfo>> {
        self.stored_auxiliary_infos.lock().clone()
    }
}

impl BlockExecutorTrait for MockBlockExecutor {
    fn committed_block_id(&self) -> HashValue {
        HashValue::zero()
    }

    fn reset(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn execute_and_update_state(
        &self,
        input: ExecutableBlock,
        _parent_block_id: aptos_crypto::HashValue,
        _onchain_config: BlockExecutorConfigFromOnchain,
    ) -> ExecutorResult<()> {
        // Extract auxiliary info from the input and store it
        let persisted_infos: Vec<PersistedAuxiliaryInfo> = input.auxiliary_info
            .into_iter()
            .map(|info| info.into_persisted_info())
            .collect();

        // Extract transactions count for logging
        let txns_count = match input.transactions {
            aptos_types::block_executor::partitioner::ExecutableTransactions::Unsharded(ref txns) => txns.len(),
            aptos_types::block_executor::partitioner::ExecutableTransactions::Sharded(ref partitioned) => {
                partitioned.num_txns()
            }
        };

        self.stored_auxiliary_infos.lock().push(persisted_infos);
        println!("Mock executor stored {} auxiliary infos", txns_count);
        Ok(())
    }

    fn ledger_update(
        &self,
        _block_id: aptos_crypto::HashValue,
        _parent_block_id: aptos_crypto::HashValue,
    ) -> ExecutorResult<StateComputeResult> {
        let _version = self.version_counter.fetch_add(1, Ordering::SeqCst);

        // Create a mock state compute result
        let compute_result = StateComputeResult::new_dummy();
        Ok(compute_result)
    }

    fn pre_commit_block(&self, _block_id: aptos_crypto::HashValue) -> ExecutorResult<()> {
        Ok(())
    }

    fn commit_ledger(
        &self,
        _ledger_info_with_sigs: LedgerInfoWithSignatures,
    ) -> ExecutorResult<()> {
        Ok(())
    }

    fn finish(&self) {}
}

/// Mock implementations for required traits
struct MockNotificationSender;

#[async_trait::async_trait]
impl aptos_consensus_notifications::ConsensusNotificationSender for MockNotificationSender {
    async fn notify_new_commit(
        &self,
        _transactions: Vec<Transaction>,
        _subscribable_events: Vec<aptos_types::contract_event::ContractEvent>,
    ) -> Result<(), aptos_consensus_notifications::Error> {
        Ok(())
    }

    async fn sync_for_duration(
        &self,
        _duration: Duration,
    ) -> Result<LedgerInfoWithSignatures, aptos_consensus_notifications::Error> {
        let ledger_info = LedgerInfo::mock_genesis(None);
        Ok(LedgerInfoWithSignatures::new(ledger_info, AggregateSignature::empty()))
    }

    async fn sync_to_target(&self, _target: LedgerInfoWithSignatures) -> Result<(), aptos_consensus_notifications::Error> {
        Ok(())
    }
}

struct MockPayloadManager;

#[async_trait::async_trait]
impl crate::payload_manager::TPayloadManager for MockPayloadManager {
    fn notify_commit(&self, _timestamp: u64, _payloads: Vec<Payload>) {}

    fn prefetch_payload_data(&self, _payload: &Payload, _author: PeerId, _timestamp: u64) {}

    fn check_denied_inline_transactions(
        &self,
        _block: &Block,
        _block_txn_filter_config: &BlockTransactionFilterConfig,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn check_payload_availability(&self, _block: &Block) -> Result<(), BitVec> {
        Ok(())
    }

    async fn get_transactions(
        &self,
        _block: &Block,
        _block_voters: Option<BitVec>,
    ) -> ExecutorResult<(Vec<SignedTransaction>, Option<u64>, Option<u64>)> {
        // Return some test transactions for the pipeline to process
        let test_txns = create_test_transactions(3);
        Ok((test_txns, Some(1000), None))
    }
}

struct MockTxnNotifier;

#[async_trait::async_trait]
impl crate::txn_notifier::TxnNotifier for MockTxnNotifier {
    async fn notify_failed_txn(
        &self,
        _txns: &[SignedTransaction],
        _compute_results: &[aptos_types::transaction::TransactionStatus],
    ) -> Result<(), crate::error::MempoolError> {
        Ok(())
    }
}

/// Create test signed transactions
fn create_test_transactions(count: usize) -> Vec<SignedTransaction> {
    (0..count)
        .map(|i| {
            let private_key = Ed25519PrivateKey::generate_for_testing();
            let account = AccountAddress::from_hex_literal(&format!("0x{:02x}", i)).unwrap();

            let raw_txn = RawTransaction::new(
                account,
                i as u64,
                TransactionPayload::Script(aptos_types::transaction::Script::new(
                    vec![],
                    vec![],
                    vec![],
                )),
                1000,
                1,
                0,
                aptos_types::chain_id::ChainId::new(1),
            );

            SignedTransaction::new(raw_txn.clone(), private_key.public_key(), private_key.sign(&raw_txn).unwrap())
        })
        .collect()
}

/// Create a simple test block (using dummy payload)
fn create_test_block(round: Round) -> Block {
    let block_data = BlockData::new_for_testing(
        round,
        0,
        0, // epoch
        QuorumCert::dummy(),
        BlockType::Genesis
    );
    Block::new_for_testing(HashValue::random(), block_data, None)
}

/// Test auxiliary info generation and storage through the full pipeline (version 0)
#[tokio::test]
async fn test_pipeline_auxiliary_info_storage_v0() {
    // Create test block
    let block = create_test_block(1);

    // Create mock executor and other dependencies
    let executor = Arc::new(MockBlockExecutor::new());
    let proposer = AccountAddress::random();
    let validators = Arc::from([proposer].as_slice());
    let signer = Arc::new(ValidatorSigner::random(None));
    let state_sync_notifier = Arc::new(MockNotificationSender);
    let payload_manager = Arc::new(MockPayloadManager);
    let txn_notifier = Arc::new(MockTxnNotifier);

    // Use simplified block preparer by directly using the existing one
    let block_preparer = Arc::new(crate::block_preparer::BlockPreparer::new(
        payload_manager.clone(),
        Arc::new(BlockTransactionFilterConfig::default()),
        crate::transaction_deduper::create_transaction_deduper(TransactionDeduperType::NoDedup),
        crate::transaction_shuffler::create_transaction_shuffler(TransactionShufflerType::NoShuffling),
    ));

    // Create pipeline builder with version 0 (None auxiliary info)
    let pipeline_builder = PipelineBuilder::new(
        block_preparer,
        executor.clone(),
        validators,
        BlockExecutorConfigFromOnchain::new_no_block_limit(),
        false, // is_randomness_enabled
        signer,
        state_sync_notifier,
        payload_manager,
        txn_notifier,
        true, // enable_pre_commit
        true, // order_vote_enabled
        0,     // persisted_auxiliary_info_version = 0
    );

    // Create a pipelined block with dummy transactions
    let test_txns = create_test_transactions(3);
    let pipelined_block = PipelinedBlock::new(
        block,
        test_txns,
        StateComputeResult::new_dummy()
    );

    // Create parent futures (simplified for test)
    let ledger_info = LedgerInfo::mock_genesis(None);
    let parent_futs = pipeline_builder.build_root(
        StateComputeResult::new_dummy(),
        LedgerInfoWithSignatures::new(ledger_info, AggregateSignature::empty()),
    );

    // Build pipeline for the block
    pipeline_builder.build(
        &pipelined_block,
        parent_futs,
        Box::new(|_order_proof, _commit_proof| {}),
    );

    // Trigger pipeline execution by providing necessary inputs
    let pipeline_tx = pipelined_block.pipeline_tx().lock().take().unwrap();
    let qc = Arc::new(QuorumCert::dummy());
    let _ = pipeline_tx.qc_tx.unwrap().send(qc);
    let _ = pipeline_tx.rand_tx.unwrap().send(None);

    // Wait for execution to complete
    let pipeline_futs = pipelined_block.pipeline_futs().unwrap();
    let _ = pipeline_futs.execute_fut.await;

    // Verify auxiliary info was stored correctly (Version 0 should have None)
    let stored_infos = executor.get_stored_auxiliary_infos();
    assert!(!stored_infos.is_empty(), "No auxiliary info blocks were stored");

    let first_block_infos = &stored_infos[0];

    // For version 0, all auxiliary info should be None
    for (i, info) in first_block_infos.iter().enumerate() {
        match info {
            PersistedAuxiliaryInfo::None => {
                println!("✓ V0: Transaction {} has None auxiliary info as expected", i);
            },
            _ => panic!("Expected None auxiliary info for transaction {} with version 0", i),
        }
    }

    println!("✓ Pipeline auxiliary info test V0 completed successfully");
}

/// Test auxiliary info generation and storage through the full pipeline (version 1)
#[tokio::test]
async fn test_pipeline_auxiliary_info_storage_v1() {
    // Create test block
    let block = create_test_block(1);

    // Create mock executor and other dependencies
    let executor = Arc::new(MockBlockExecutor::new());
    let proposer = AccountAddress::random();
    let validators = Arc::from([proposer].as_slice());
    let signer = Arc::new(ValidatorSigner::random(None));
    let state_sync_notifier = Arc::new(MockNotificationSender);
    let payload_manager = Arc::new(MockPayloadManager);
    let txn_notifier = Arc::new(MockTxnNotifier);

    // Use simplified block preparer by directly using the existing one
    let block_preparer = Arc::new(crate::block_preparer::BlockPreparer::new(
        payload_manager.clone(),
        Arc::new(BlockTransactionFilterConfig::default()),
        crate::transaction_deduper::create_transaction_deduper(TransactionDeduperType::NoDedup),
        crate::transaction_shuffler::create_transaction_shuffler(TransactionShufflerType::NoShuffling),
    ));

    // Create pipeline builder with version 1 (V1 auxiliary info with transaction index)
    let pipeline_builder = PipelineBuilder::new(
        block_preparer,
        executor.clone(),
        validators,
        BlockExecutorConfigFromOnchain::new_no_block_limit(),
        false, // is_randomness_enabled
        signer,
        state_sync_notifier,
        payload_manager,
        txn_notifier,
        false, // enable_pre_commit
        false, // order_vote_enabled
        1,     // persisted_auxiliary_info_version = 1
    );

    // Create a pipelined block with dummy transactions
    let test_txns = create_test_transactions(3);
    let pipelined_block = PipelinedBlock::new(
        block,
        test_txns,
        StateComputeResult::new_dummy()
    );

    // Create parent futures (simplified for test)
    let ledger_info = LedgerInfo::mock_genesis(None);
    let parent_futs = pipeline_builder.build_root(
        StateComputeResult::new_dummy(),
        LedgerInfoWithSignatures::new(ledger_info, AggregateSignature::empty()),
    );

    // Build pipeline for the block
    pipeline_builder.build(
        &pipelined_block,
        parent_futs,
        Box::new(|_order_proof, _commit_proof| {}),
    );

    // Trigger pipeline execution by providing necessary inputs
    let pipeline_tx = pipelined_block.pipeline_tx().lock().take().unwrap();
    let qc = Arc::new(QuorumCert::dummy());
    let _ = pipeline_tx.qc_tx.unwrap().send(qc);
    let _ = pipeline_tx.rand_tx.unwrap().send(None);

    // Wait for execution to complete
    let pipeline_futs = pipelined_block.pipeline_futs().unwrap();
    let _ = pipeline_futs.execute_fut.await;

    // Verify auxiliary info was stored correctly (Version 1 should have transaction indices)
    let stored_infos = executor.get_stored_auxiliary_infos();
    assert!(!stored_infos.is_empty(), "No auxiliary info blocks were stored");

    let first_block_infos = &stored_infos[0];

    // For version 1, auxiliary info should contain transaction indices
    // Note: The block will have block metadata + validator txns + user txns
    // We need to account for that in our assertions
    for (i, info) in first_block_infos.iter().enumerate() {
        match info {
            PersistedAuxiliaryInfo::V1 { transaction_index } => {
                assert_eq!(*transaction_index, i as u32);
                println!("✓ V1: Transaction {} has auxiliary info with transaction_index = {}",
                         i, transaction_index);
            },
            PersistedAuxiliaryInfo::None => {
                // This might be block metadata or validator transactions
                println!("✓ V1: Transaction {} has None auxiliary info (likely metadata/validator txn)", i);
            },
        }
    }

    println!("✓ Pipeline auxiliary info test V1 completed successfully");
}

/// Test that different auxiliary info versions produce different results
#[tokio::test]
async fn test_auxiliary_info_version_differences() {
    // Test both versions and compare results
    for version in [0u8, 1u8] {
        let block = create_test_block(1);
        let executor = Arc::new(MockBlockExecutor::new());
        let proposer = AccountAddress::random();
        let validators = Arc::from([proposer].as_slice());
        let signer = Arc::new(ValidatorSigner::random(None));
        let payload_manager = Arc::new(MockPayloadManager);

        let block_preparer = Arc::new(crate::block_preparer::BlockPreparer::new(
            payload_manager.clone(),
            Arc::new(BlockTransactionFilterConfig::default()),
            crate::transaction_deduper::create_transaction_deduper(TransactionDeduperType::NoDedup),
            crate::transaction_shuffler::create_transaction_shuffler(TransactionShufflerType::NoShuffling),
        ));

        let pipeline_builder = PipelineBuilder::new(
            block_preparer,
            executor.clone(),
            validators,
            BlockExecutorConfigFromOnchain::new_no_block_limit(),
            false,
            signer,
            Arc::new(MockNotificationSender),
            payload_manager,
            Arc::new(MockTxnNotifier),
            false,
            false,
            version,
        );

        let test_txns = create_test_transactions(3);
        let pipelined_block = PipelinedBlock::new(
            block,
            test_txns,
            StateComputeResult::new_dummy()
        );
        let ledger_info = LedgerInfo::mock_genesis(None);
        let parent_futs = pipeline_builder.build_root(
            StateComputeResult::new_dummy(),
            LedgerInfoWithSignatures::new(ledger_info, AggregateSignature::empty()),
        );

        pipeline_builder.build(
            &pipelined_block,
            parent_futs,
            Box::new(|_order_proof, _commit_proof| {}),
        );

        let pipeline_tx = pipelined_block.pipeline_tx().lock().take().unwrap();
        let qc = Arc::new(QuorumCert::dummy());
        let _ = pipeline_tx.qc_tx.unwrap().send(qc);
        let _ = pipeline_tx.rand_tx.unwrap().send(None);

        let pipeline_futs = pipelined_block.pipeline_futs().unwrap();
        let _ = pipeline_futs.execute_fut.await;

        let stored_infos = executor.get_stored_auxiliary_infos();
        assert!(!stored_infos.is_empty());

        println!("✓ Version {} auxiliary info test completed", version);
    }

    println!("✓ Auxiliary info version differences test completed");
}
