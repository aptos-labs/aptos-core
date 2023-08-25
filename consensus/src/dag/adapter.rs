// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    dag::{order_rule::Notifier, storage::DAGStorage, CertifiedNode},
    experimental::buffer_manager::OrderedBlocks,
};
use aptos_consensus_types::{
    block::Block,
    common::{Author, Payload, Round},
    executed_block::ExecutedBlock,
};
use aptos_crypto::HashValue;
use aptos_executor_types::StateComputeResult;
use aptos_logger::error;
use aptos_types::{
    aggregate_signature::AggregateSignature,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
};
use futures_channel::mpsc::UnboundedSender;
use std::sync::Arc;

pub struct BufferManagerAdapter {
    executor_channel: UnboundedSender<OrderedBlocks>,
    storage: Arc<dyn DAGStorage>,
}

impl BufferManagerAdapter {
    pub fn new(
        executor_channel: UnboundedSender<OrderedBlocks>,
        storage: Arc<dyn DAGStorage>,
    ) -> Self {
        Self {
            executor_channel,
            storage,
        }
    }
}

impl Notifier for BufferManagerAdapter {
    fn send(
        &mut self,
        ordered_nodes: Vec<Arc<CertifiedNode>>,
        failed_author: Vec<(Round, Author)>,
    ) -> anyhow::Result<()> {
        let anchor = ordered_nodes.last().unwrap();
        let anchor_id = anchor.id();
        let epoch = anchor.epoch();
        let round = anchor.round();
        let timestamp = anchor.metadata().timestamp();
        let author = *anchor.author();
        let mut payload = Payload::empty(!anchor.payload().is_direct());
        let mut node_digests = vec![];
        for node in &ordered_nodes {
            payload.extend(node.payload().clone());
            node_digests.push(node.digest());
        }
        // TODO: we may want to split payload into multiple blocks
        let block = ExecutedBlock::new(
            Block::new_for_dag(epoch, round, timestamp, payload, author, failed_author)?,
            StateComputeResult::new_dummy(),
        );
        let block_info = block.block_info();
        let storage = self.storage.clone();
        Ok(self.executor_channel.unbounded_send(OrderedBlocks {
            ordered_blocks: vec![block],
            ordered_proof: LedgerInfoWithSignatures::new(
                LedgerInfo::new(block_info, HashValue::zero()),
                AggregateSignature::empty(),
            ),
            callback: Box::new(
                move |_committed_blocks: &[Arc<ExecutedBlock>],
                      _commit_decision: LedgerInfoWithSignatures| {
                    // TODO: this doesn't really work since not every block will trigger a callback,
                    // we need to update the buffer manager to invoke all callbacks instead of only last one
                    if let Err(e) = storage
                        .delete_certified_nodes(node_digests)
                        .and_then(|_| storage.delete_ordered_anchor_ids(vec![anchor_id]))
                    {
                        error!(
                            "Failed to garbage collect committed nodes and anchor: {:?}",
                            e
                        );
                    }
                },
            ),
        })?)
    }
}
