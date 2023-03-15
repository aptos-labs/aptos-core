// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::experimental::ordering_state_computer::OrderingStateComputer;
use aptos_consensus_types::node::CertifiedNode;
use std::sync::Arc;
use tokio::sync::mpsc::Receiver;

#[allow(dead_code)]
pub struct Bullshark {
    state_computer: Arc<OrderingStateComputer>,
}

#[allow(dead_code)]
impl Bullshark {
    pub fn new(state_computer: Arc<OrderingStateComputer>) -> Self {
        Self { state_computer }
    }

    pub async fn start(self, mut rx: Receiver<CertifiedNode>) {
        loop {
            tokio::select! {
            Some(_) = rx.recv() => {

            }
                }
        }
    }
}
