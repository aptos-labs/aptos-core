// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::dag::{
    dag_network::{DAGNetworkSender, RpcHandler},
    types::{DAGMessageTrait, Node, NodeDigestSignature},
};
use aptos_consensus_types::common::{Author, Round};
use aptos_types::validator_signer::ValidatorSigner;
use futures::{stream::FuturesUnordered, StreamExt};
use std::{collections::BTreeMap, future::Future, sync::Arc, time::Duration};

pub trait BroadcastStatus {
    type Ack: DAGMessageTrait;
    type Aggregated;
    type Message: DAGMessageTrait;

    fn add(&mut self, peer: Author, ack: Self::Ack) -> anyhow::Result<Option<Self::Aggregated>>;
}

pub struct ReliableBroadcast {
    validators: Vec<Author>,
    network_sender: Arc<dyn DAGNetworkSender>,
}

impl ReliableBroadcast {
    pub fn new(validators: Vec<Author>, network_sender: Arc<dyn DAGNetworkSender>) -> Self {
        Self {
            validators,
            network_sender,
        }
    }

    pub fn broadcast<S: BroadcastStatus>(
        &self,
        message: S::Message,
        mut aggregating: S,
    ) -> impl Future<Output = S::Aggregated> {
        let receivers: Vec<_> = self.validators.clone();
        let network_sender = self.network_sender.clone();
        async move {
            let mut fut = FuturesUnordered::new();
            let send_message = |receiver, message| {
                let network_sender = network_sender.clone();
                async move {
                    (
                        receiver,
                        network_sender
                            .send_rpc(receiver, message, Duration::from_millis(500))
                            .await,
                    )
                }
            };
            let network_message = message.into();
            for receiver in receivers {
                fut.push(send_message(receiver, network_message.clone()));
            }
            while let Some((receiver, result)) = fut.next().await {
                match result {
                    Ok(msg) => {
                        if let Ok(ack) = S::Ack::try_from(msg) {
                            if let Ok(Some(aggregated)) = aggregating.add(receiver, ack) {
                                return aggregated;
                            }
                        }
                    },
                    Err(_) => fut.push(send_message(receiver, network_message.clone())),
                }
            }
            unreachable!("Should aggregate with all responses");
        }
    }
}

pub struct NodeBroadcastHandler {
    signatures_by_round_peer: BTreeMap<Round, BTreeMap<Author, NodeDigestSignature>>,
    signer: ValidatorSigner,
}

impl NodeBroadcastHandler {
    pub fn new(signer: ValidatorSigner) -> Self {
        Self {
            signatures_by_round_peer: BTreeMap::new(),
            signer,
        }
    }

    pub fn gc_before_round(&mut self, min_round: Round) {
        self.signatures_by_round_peer.retain(|r, _| r >= &min_round);
    }
}

impl RpcHandler for NodeBroadcastHandler {
    type Ack = NodeDigestSignature;
    type Message = Node;

    fn process(&mut self, message: Self::Message) -> anyhow::Result<Self::Ack> {
        let signatures_by_peer = self
            .signatures_by_round_peer
            .entry(message.metadata().round())
            .or_insert(BTreeMap::new());
        match signatures_by_peer.get(message.metadata().author()) {
            None => {
                let signature = message.sign(&self.signer)?;
                let digest_signature = NodeDigestSignature::new(
                    message.metadata().epoch(),
                    message.digest(),
                    signature,
                );
                signatures_by_peer.insert(*message.metadata().author(), digest_signature.clone());
                Ok(digest_signature)
            },
            Some(ack) => Ok(ack.clone()),
        }
    }
}
