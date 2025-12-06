// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

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
    dec_metadata: SecretShareMetadata,
    dec_store: Arc<Mutex<SecretShareStore>>,
    dec_config: SecretShareConfig,
}

impl SecretShareAggregateState {
    pub fn new(
        dec_store: Arc<Mutex<SecretShareStore>>,
        dec_metadata: SecretShareMetadata,
        dec_config: SecretShareConfig,
    ) -> Self {
        Self {
            dec_store,
            dec_metadata,
            dec_config,
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
            share.metadata() == &self.dec_metadata,
            "Metadata does not match: local {:?}, received {:?}",
            self.dec_metadata,
            share.metadata()
        );
        share.verify(&self.dec_config)?;
        info!(LogSchema::new(LogEvent::ReceiveReactiveSecretShare)
            .epoch(share.epoch())
            .round(share.metadata().round)
            .remote_peer(*share.author()));
        let mut store = self.dec_store.lock();
        let aggregated = if store.add_share(share)? {
            Some(())
        } else {
            None
        };
        Ok(aggregated)
    }
}
