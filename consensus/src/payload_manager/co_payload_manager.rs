// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    consensus_observer::{
        observer::payload_store::BlockPayloadStatus,
        publisher::consensus_publisher::ConsensusPublisher,
    },
    payload_manager::TPayloadManager,
};
use aptos_bitvec::BitVec;
use aptos_consensus_types::{
    block::Block,
    common::{Author, Payload, Round},
};
use aptos_crypto::HashValue;
use aptos_executor_types::{ExecutorError::InternalError, *};
use aptos_infallible::Mutex;
use aptos_types::transaction::SignedTransaction;
use async_trait::async_trait;
use std::{
    collections::{btree_map::Entry, BTreeMap},
    sync::Arc,
};

/// Returns the transactions for the consensus observer payload manager
async fn get_transactions_for_observer(
    block: &Block,
    block_payloads: &Arc<Mutex<BTreeMap<(u64, Round), BlockPayloadStatus>>>,
    consensus_publisher: &Option<Arc<ConsensusPublisher>>,
) -> ExecutorResult<(Vec<SignedTransaction>, Option<u64>, Option<u64>)> {
    // The data should already be available (as consensus observer will only ever
    // forward a block to the executor once the data has been received and verified).
    let block_payload = match block_payloads.lock().entry((block.epoch(), block.round())) {
        Entry::Occupied(mut value) => match value.get_mut() {
            BlockPayloadStatus::AvailableAndVerified(block_payload) => block_payload.clone(),
            BlockPayloadStatus::AvailableAndUnverified(_) => {
                // This shouldn't happen (the payload should already be verified)
                let error = format!(
                    "Payload data for block epoch {}, round {} is unverified!",
                    block.epoch(),
                    block.round()
                );
                return Err(InternalError { error });
            },
        },
        Entry::Vacant(_) => {
            // This shouldn't happen (the payload should already be present)
            let error = format!(
                "Missing payload data for block epoch {}, round {}!",
                block.epoch(),
                block.round()
            );
            return Err(InternalError { error });
        },
    };

    // If the payload is valid, publish it to any downstream observers
    let transaction_payload = block_payload.transaction_payload();
    if let Some(consensus_publisher) = consensus_publisher {
        consensus_publisher.publish_block_payload(
            block.gen_block_info(HashValue::zero(), 0, None),
            transaction_payload.clone(),
        );
    }

    // Return the transactions and the transaction limit
    Ok((
        transaction_payload.transactions(),
        transaction_payload.transaction_limit(),
        transaction_payload.gas_limit(),
    ))
}

pub struct ConsensusObserverPayloadManager {
    txns_pool: Arc<Mutex<BTreeMap<(u64, Round), BlockPayloadStatus>>>,
    consensus_publisher: Option<Arc<ConsensusPublisher>>,
}

impl ConsensusObserverPayloadManager {
    pub fn new(
        txns_pool: Arc<Mutex<BTreeMap<(u64, Round), BlockPayloadStatus>>>,
        consensus_publisher: Option<Arc<ConsensusPublisher>>,
    ) -> Self {
        Self {
            txns_pool,
            consensus_publisher,
        }
    }
}

#[async_trait]
impl TPayloadManager for ConsensusObserverPayloadManager {
    fn notify_commit(&self, _block_timestamp: u64, _payloads: Vec<Payload>) {}

    fn prefetch_payload_data(&self, _payload: &Payload, _author: Author, _timestamp: u64) {}

    fn check_payload_availability(&self, _block: &Block) -> Result<(), BitVec> {
        unreachable!("this method isn't used in ConsensusObserver")
    }

    async fn get_transactions(
        &self,
        block: &Block,
        _block_signers: Option<BitVec>,
    ) -> ExecutorResult<(Vec<SignedTransaction>, Option<u64>, Option<u64>)> {
        get_transactions_for_observer(block, &self.txns_pool, &self.consensus_publisher).await
    }
}
