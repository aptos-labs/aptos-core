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
