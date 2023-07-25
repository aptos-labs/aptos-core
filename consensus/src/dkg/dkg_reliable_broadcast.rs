// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, ensure};
use aptos_consensus_types::common::{Author, Round};
use aptos_infallible::{RwLock, Mutex};
use aptos_logger::error;
use aptos_types::{epoch_state::EpochState, validator_signer::ValidatorSigner};
use futures::{stream::FuturesUnordered, StreamExt};
use std::{collections::BTreeMap, future::Future, mem, sync::Arc, time::Duration};
use thiserror::Error as ThisError;

use super::{types::{TDKGMessage, DKGNodeAck, DKGAggNode, DKGAggNodeAck}, dkg_network::{DKGNetworkSender, DKGRpcHandler}, dkg_store::DKGStore, DKGNode, dkg_manager::{DKGManager, self}};
use crate::network::TConsensusMsg;

pub trait DKGBroadcastStatus {
    type Ack: TDKGMessage;
    type Aggregated;
    type Message: TDKGMessage;

    fn add(&mut self, peer: Author, ack: Self::Ack) -> anyhow::Result<Option<Self::Aggregated>>;
}

pub struct ReliableBroadcast {
    validators: Vec<Author>,
    network_sender: Arc<dyn DKGNetworkSender>,
}

impl ReliableBroadcast {
    pub fn new(validators: Vec<Author>, network_sender: Arc<dyn DKGNetworkSender>) -> Self {
        Self {
            validators,
            network_sender,
        }
    }

    pub fn broadcast<S: DKGBroadcastStatus>(
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
            let network_message = message.into().into_network_message();
            for receiver in receivers {
                fut.push(send_message(receiver, network_message.clone()));
            }
            while let Some((receiver, result)) = fut.next().await {
                match result {
                    Ok(msg) => {
                        if let Ok(dag_msg) = msg.try_into() {
                            if let Ok(ack) = S::Ack::try_from(dag_msg) {
                                if let Ok(Some(aggregated)) = aggregating.add(receiver, ack) {
                                    return aggregated;
                                }
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

#[derive(ThisError, Debug)]
pub enum DKGNodeHandleError {
    #[error("dummy error")]
    DummyError,
}

pub struct DKGNodeHandler {
    dkg_store: Arc<DKGStore>,
    epoch_state: Arc<EpochState>,
    dkg_manager: Arc<Mutex<DKGManager>>,
}

impl DKGNodeHandler {
    pub fn new(
        dkg_store: Arc<DKGStore>,
        epoch_state: Arc<EpochState>,
        dkg_manager: Arc<Mutex<DKGManager>>,
    ) -> Self {
        Self {
            dkg_store,
            epoch_state,
            dkg_manager,
        }
    }
}

impl DKGRpcHandler for DKGNodeHandler {
    type DKGRequest = DKGNode;
    type DKGResponse = DKGNodeAck;

    fn process(&mut self, node: Self::DKGRequest) -> anyhow::Result<Self::DKGResponse> {
        let epoch = node.epoch();
        // dkg todo: persist the dkg nodes
        self.dkg_store.add_node(node, &self.epoch_state.verifier, self.dkg_manager.clone())?;
        Ok(DKGNodeAck::new(epoch))
    }
}

#[derive(Debug, ThisError)]
pub enum DKGAggNodeHandleError {
    #[error("dummy error")]
    DummyError,
}

pub struct DKGAggNodeHandler {
    dkg_store: Arc<DKGStore>,
    epoch_state: Arc<EpochState>,
    dkg_manager: Arc<Mutex<DKGManager>>,
}

impl DKGAggNodeHandler {
    pub fn new(dkg_store: Arc<DKGStore>, epoch_state: Arc<EpochState>, dkg_manager: Arc<Mutex<DKGManager>>) -> Self {
        Self {
            dkg_store,
            epoch_state,
            dkg_manager,
        }
    }
}

impl DKGRpcHandler for DKGAggNodeHandler {
    type DKGRequest = DKGAggNode;
    type DKGResponse = DKGAggNodeAck;

    fn process(&mut self, agg_node: Self::DKGRequest) -> anyhow::Result<Self::DKGResponse> {
        let epoch = agg_node.epoch();
        self.dkg_store.add_agg_nodes(agg_node, &self.epoch_state.verifier, self.dkg_manager.clone())?;
        Ok(DKGAggNodeAck::new(epoch))
    }
}
