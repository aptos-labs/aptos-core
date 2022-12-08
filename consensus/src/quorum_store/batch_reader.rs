// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_consensus_types::proof_of_store::{LogicalTime, ProofOfStore};
use aptos_types::transaction::SignedTransaction;
use executor_types::Error;
use tokio::sync::oneshot;

pub struct BatchReader {}

impl BatchReader {
    pub async fn get_batch(
        &self,
        _proof: ProofOfStore,
    ) -> oneshot::Receiver<Result<Vec<SignedTransaction>, Error>> {
        let (_tx, rx) = oneshot::channel();
        // TODO: verify expiration

        // TODO: coming soon

        rx
    }

    pub async fn update_certified_round(&self, _certified_time: LogicalTime) {
        // TODO: coming soon
    }
}
