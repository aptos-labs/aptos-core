// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    logging::{LogEvent, LogSchema},
    rand::secret_sharing::{
        network_messages::SecretShareMessage, secret_share_store::SecretShareStore,
        types::RequestSecretShare,
    },
};
use anyhow::ensure;
use aptos_consensus_types::common::Author;
use aptos_infallible::Mutex;
use aptos_logger::info;
use aptos_reliable_broadcast::BroadcastStatus;
use aptos_types::secret_sharing::{SecretShare, SecretShareConfig, SecretShareMetadata};
use std::sync::Arc;

pub struct SecretShareAggregateState {
    secret_share_metadata: SecretShareMetadata,
    secret_share_store: Arc<Mutex<SecretShareStore>>,
    secret_share_config: SecretShareConfig,
}

impl SecretShareAggregateState {
    pub fn new(
        secret_share_store: Arc<Mutex<SecretShareStore>>,
        secret_share_metadata: SecretShareMetadata,
        secret_share_config: SecretShareConfig,
    ) -> Self {
        Self {
            secret_share_store,
            secret_share_metadata,
            secret_share_config,
        }
    }
}

impl BroadcastStatus<SecretShareMessage, SecretShareMessage> for Arc<SecretShareAggregateState> {
    type Aggregated = ();
    type Message = RequestSecretShare;
    type Response = SecretShare;

    fn add(&self, peer: Author, share: Self::Response) -> anyhow::Result<Option<()>> {
        ensure!(share.author() == &peer, "Author does not match");
        ensure!(
            share.metadata() == &self.secret_share_metadata,
            "Metadata does not match: local {:?}, received {:?}",
            self.secret_share_metadata,
            share.metadata()
        );
        share.verify(&self.secret_share_config)?;
        info!(LogSchema::new(LogEvent::ReceiveReactiveSecretShare)
            .epoch(share.epoch())
            .round(share.metadata().round)
            .remote_peer(*share.author()));
        let mut store = self.secret_share_store.lock();
        let aggregated = store.add_share(share)?.then_some(());
        Ok(aggregated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rand::secret_sharing::{
        secret_share_store::SecretShareStore,
        test_utils::{create_metadata, create_secret_share, TestContext},
    };
    use aptos_types::secret_sharing::SecretSharedKey;
    use futures_channel::mpsc::{unbounded, UnboundedReceiver};

    fn make_state(
        ctx: &TestContext,
        metadata: &SecretShareMetadata,
    ) -> (
        Arc<SecretShareAggregateState>,
        UnboundedReceiver<SecretSharedKey>,
    ) {
        let (tx, rx) = unbounded();
        let mut store = SecretShareStore::new(
            ctx.epoch,
            ctx.authors[0],
            ctx.secret_share_config.clone(),
            tx,
        );
        store.update_highest_known_round(metadata.round);

        // Add self share so the store is in PendingDecision state
        let self_share = create_secret_share(ctx, 0, metadata);
        store.add_self_share(self_share).unwrap();

        let state = Arc::new(SecretShareAggregateState::new(
            Arc::new(Mutex::new(store)),
            metadata.clone(),
            ctx.secret_share_config.clone(),
        ));
        (state, rx)
    }

    #[test]
    fn test_broadcast_add_happy_path() {
        let ctx = TestContext::new(vec![1, 1, 1, 1]);
        let metadata = create_metadata(ctx.epoch, 5);
        let (state, _rx) = make_state(&ctx, &metadata);

        // Valid share accepted, returns Ok(None) when below threshold
        let share = create_secret_share(&ctx, 1, &metadata);
        let result = state.add(ctx.authors[1], share);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_broadcast_add_triggers_aggregation() {
        let ctx = TestContext::new(vec![1, 1, 1, 1]);
        let metadata = create_metadata(ctx.epoch, 5);
        let (state, mut rx) = make_state(&ctx, &metadata);

        // Add enough shares to trigger aggregation
        // With 4 validators weight 1 each and threshold 3, we need 3 shares total.
        // Self share (weight 1) is already in the store. We need 2 more peer shares.
        for i in 1..=2 {
            let share = create_secret_share(&ctx, i, &metadata);
            let result = state.add(ctx.authors[i], share).unwrap();
            if i < 2 {
                assert!(result.is_none());
            } else {
                // The aggregation is triggered asynchronously via spawn_blocking,
                // so add_share returns decided=true but the channel delivery is async
                assert!(result.is_some());
            }
        }

        // Verify decision arrives on channel
        use futures::StreamExt;
        let key: SecretSharedKey =
            tokio::time::timeout(std::time::Duration::from_secs(5), rx.next())
                .await
                .expect("Timed out waiting for decision")
                .expect("Channel closed unexpectedly");
        assert_eq!(key.metadata, metadata);
    }
}
