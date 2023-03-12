// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_consensus_types::node::CertifiedNode;
use tokio::sync::mpsc::Receiver;

pub struct Bullshark {}

impl Bullshark {
    pub fn new() -> Self
    {
        Self {}
    }

    pub async fn start(mut self, mut rx: Receiver<CertifiedNode>) {
        loop {
            tokio::select! {
            Some(_) = rx.recv() => {

            }
                }
        }
    }
}