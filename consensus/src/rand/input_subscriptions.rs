// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_consensus_types::{common::Round, pipelined_block::PipelinedBlock};
use aptos_reliable_broadcast::DropGuard;
use std::{
    collections::BTreeMap,
    sync::{Arc, Weak},
};

struct Subscription {
    block: Weak<PipelinedBlock>,
    requests: Vec<DropGuard>,
}

/// Share-service subscribers and request lifetimes, with no ordered-prefix queue.
/// Block retention and execution ordering belong to the block/commit pipeline.
#[derive(Default)]
pub(crate) struct InputSubscriptions {
    subscriptions: BTreeMap<Round, Subscription>,
}

impl InputSubscriptions {
    pub fn register(&mut self, block: &Arc<PipelinedBlock>) {
        assert!(
            self.subscriptions
                .insert(block.round(), Subscription {
                    block: Arc::downgrade(block),
                    requests: vec![],
                })
                .is_none(),
            "Share input registered twice"
        );
    }

    pub fn hold_request(&mut self, round: Round, guard: DropGuard) {
        if let Some(subscription) = self.subscriptions.get_mut(&round) {
            subscription.requests.push(guard);
        }
    }

    pub fn resolve(&mut self, round: Round) -> Option<Arc<PipelinedBlock>> {
        self.subscriptions.remove(&round)?.block.upgrade()
    }

    pub fn clear(&mut self) {
        self.subscriptions.clear();
    }

    pub fn len(&self) -> usize {
        self.subscriptions.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_consensus_types::{
        block::{block_test_utils::certificate_for_genesis, Block},
        pipelined_block::OrderedBlockWindow,
    };

    #[test]
    fn later_inputs_resolve_independently_without_retaining_blocks() {
        let a = Arc::new(PipelinedBlock::new_ordered(
            Block::new_nil(1, certificate_for_genesis(), vec![]),
            OrderedBlockWindow::empty(),
        ));
        let b = Arc::new(PipelinedBlock::new_ordered(
            Block::new_nil(2, certificate_for_genesis(), vec![]),
            OrderedBlockWindow::empty(),
        ));
        let mut subscriptions = InputSubscriptions::default();
        subscriptions.register(&a);
        subscriptions.register(&b);
        assert_eq!(Arc::strong_count(&a), 1);
        assert_eq!(subscriptions.resolve(2).unwrap().id(), b.id());
        assert_eq!(subscriptions.len(), 1);
        drop(a);
        assert!(subscriptions.resolve(1).is_none());
    }
}
