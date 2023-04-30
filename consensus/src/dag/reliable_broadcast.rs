// Copyright © Aptos Foundation

// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    dag::types::{AckSet, IncrementalNodeCertificateState},
    network::{DagSender, NetworkSender},
    round_manager::VerifiedEvent,
};
use aptos_channels::aptos_channel;
use aptos_consensus_types::{
    common::Round,
    node::{CertifiedNode, CertifiedNodeAck, Node, SignedNodeDigest},
};
use aptos_logger::info;
use aptos_types::{
    validator_signer::ValidatorSigner, validator_verifier::ValidatorVerifier, PeerId,
};
use futures::{FutureExt, StreamExt};
use futures_channel::oneshot;
use std::{collections::BTreeMap, sync::Arc, time::Duration};
use std::borrow::Borrow;
use std::fmt::{Debug, Formatter};
use std::path::Path;
use tokio::{sync::mpsc::Receiver, time};
use aptos_schemadb::{ColumnFamilyName, DB, define_schema, Options};
use crate::dag::reliable_broadcast::Status::NothingToSend;
use aptos_crypto::HashValue;
use aptos_schemadb::schema::{KeyCodec, Schema, ValueCodec};
use serde::{Deserialize, Serialize};
use anyhow::Result;

#[allow(dead_code)]
#[derive(Debug)]
pub(crate) enum ReliableBroadcastCommand {
    BroadcastRequest(Node),
}

pub trait ReliableBroadcastStorage {
    fn new(peer_id: PeerId, epoch: u64) -> Self; // peer_id + epoch should be enough to identify?
    fn load_all(&mut self) -> Option<ReliableBroadcastInMem>;
    fn save_all(&mut self, in_mem: &ReliableBroadcastInMem);
}

pub struct NaiveReliableBroadcastStorage {
    db: DB,
}

define_schema!(ReliableBroadcastStateSchema, HashValue, ReliableBroadcastInMem, "cf1");

impl KeyCodec<ReliableBroadcastStateSchema> for HashValue {
    fn encode_key(&self) -> Result<Vec<u8>> {
        Ok(self.to_vec())
    }

    fn decode_key(data: &[u8]) -> Result<Self> {
        Ok(HashValue::from_slice(data)?)
    }
}

impl ValueCodec<ReliableBroadcastStateSchema> for ReliableBroadcastInMem {
    fn encode_value(&self) -> Result<Vec<u8>> {
        Ok(bcs::to_bytes(self)?)
    }

    fn decode_value(data: &[u8]) -> Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}

impl ReliableBroadcastStorage for NaiveReliableBroadcastStorage {
    fn new(peer_id: PeerId, epoch: u64) -> Self {
        //TODO: a "session" as a DB/CF/key prefix?
        //TODO: find a better path.
        let db_path_str = format!("/tmp/reliable_broadcast_dbs/{peer_id}/{epoch}");
        let db_path = Path::new(db_path_str.as_str());
        let column_families = vec![ReliableBroadcastStateSchema::COLUMN_FAMILY_NAME];
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);
        let db_name = "name1"; //TODO: better name
        let db = DB::open(db_path, db_name, column_families, &opts).expect("ReliableBroadcast DB open failed");
        Self {
            db
        }
    }

    fn load_all(&mut self) -> Option<ReliableBroadcastInMem> {
        let k = HashValue::default();
        self.db.get::<ReliableBroadcastStateSchema>(&k).expect("Failed in loading reliable broadcast state!");
        //TODO: debugging
        None
    }

    fn save_all(&mut self, in_mem: &ReliableBroadcastInMem) {
        let k = HashValue::default();
        self.db.put::<ReliableBroadcastStateSchema>(&k, in_mem).expect("Failed in saving reliable broadcast state!");
    }
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub enum Status {
    NothingToSend,
    SendingNode(Node, IncrementalNodeCertificateState),
    SendingCertificate(CertifiedNode, AckSet),
}

// TODO: should we use the same message for node and certificate node -> create two verified events.

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct ReliableBroadcastInMem {
    pub my_id: PeerId,
    pub epoch: u64,
    pub status: Status,
    pub peer_round_signatures: BTreeMap<(Round, PeerId), SignedNodeDigest>,
    // vs BTreeMap<Round, BTreeMap<PeerId, ConsensusMsg>> vs Hashset?
}

impl Default for ReliableBroadcastInMem {
    fn default() -> Self {
        Self {
            my_id: PeerId::ONE,
            epoch: 0,
            status: Status::NothingToSend,
            peer_round_signatures: BTreeMap::new(),
        }
    }
}

impl ReliableBroadcastInMem {
    pub fn new(
        my_id: PeerId,
        epoch: u64,
    ) -> Self {
        let status = NothingToSend;
        Self {
            my_id,
            epoch,
            status,
            // TODO: Do we need to clean memory inside an epoc? We need to DB between epochs.
            peer_round_signatures: BTreeMap::new(),
        }
    }
}

pub struct ReliableBroadcast<T: ReliableBroadcastStorage> {
    storage: T,
    in_mem: ReliableBroadcastInMem,
    network_sender: NetworkSender,
    validator_verifier: ValidatorVerifier,
    validator_signer: Arc<ValidatorSigner>,
}

impl<T: ReliableBroadcastStorage> ReliableBroadcast<T> {
    pub fn new(
        my_id: PeerId,
        epoch: u64,
        network_sender: NetworkSender,
        validator_verifier: ValidatorVerifier,
        validator_signer: Arc<ValidatorSigner>,
    ) -> Self {
        let mut storage = T::new(my_id, epoch);
        Self {
            storage,
            in_mem: ReliableBroadcastInMem::default(),
            network_sender,
            validator_verifier,
            validator_signer,
        }
    }

    async fn handle_broadcast_request(&mut self, node: Node) {
        // It is live to stop broadcasting the previous node at this point.
        self.in_mem.status = Status::SendingNode(
            node.clone(),
            IncrementalNodeCertificateState::new(node.digest()),
        );
        self.persist_state(); //TODO: only write the part being changed.

        self.network_sender.send_node(node, None).await
    }

    // TODO: verify earlier that digest matches the node and epoch is right.
    // TODO: verify node has n-f parents(?).
    async fn handle_node_message(&mut self, node: Node) {
        match self.in_mem
            .peer_round_signatures
            .get(&(node.round(), node.source()))
        {
            Some(signed_node_digest) => {
                self.network_sender
                    .send_signed_node_digest(signed_node_digest.clone(), vec![node.source()])
                    .await
            },
            None => {
                let signed_node_digest =
                    SignedNodeDigest::new(self.in_mem.epoch, node.digest(), self.validator_signer.clone())
                        .unwrap();
                self.in_mem.peer_round_signatures
                    .insert((node.round(), node.source()), signed_node_digest.clone());
                self.persist_state();  //TODO: only write the part being changed.
                self.network_sender
                    .send_signed_node_digest(signed_node_digest, vec![node.source()])
                    .await;
            },
        }
    }
    fn persist_state(&mut self) {
        self.storage.save_all(&self.in_mem);
    }
    async fn handle_signed_digest(
        &mut self,
        signed_node_digest: SignedNodeDigest,
    ) -> Option<CertifiedNode> {
        match &mut self.in_mem.status {
            Status::SendingNode(node, incremental_node_certificate_state) => {

                if let Err(e) = incremental_node_certificate_state.add_signature(signed_node_digest.clone()) {
                    info!("DAG: could not add signature, err = {:?}", e);
                    None
                } else {
                    let maybe_node = if incremental_node_certificate_state.ready(&self.validator_verifier) {
                        let node_certificate =
                            incremental_node_certificate_state.take(&self.validator_verifier);
                        let certified_node = CertifiedNode::new(node.clone(), node_certificate);
                        let ack_set = AckSet::new(certified_node.node().digest());

                        self.in_mem.status = Status::SendingCertificate(certified_node.clone(), ack_set);
                        Some(certified_node)
                    } else {
                        None
                    };
                    self.persist_state();  //TODO: only write the part being changed.
                    maybe_node
                }
            },
            _ => {
                None
            }

        }
    }

    // TODO: consider marge node and certified node and use a trait to resend message.
    async fn handle_tick(&mut self) {
        match &self.in_mem.status {
            // Status::NothingToSend => info!("DAG: reliable broadcast has nothing to resend on tick peer_id {},", self.my_id),
            NothingToSend => info!("DAG: reliable broadcast has nothing to resend on tick"),
            Status::SendingNode(node, incremental_node_certificate_state) => {
                self.network_sender
                    .send_node(
                        node.clone(),
                        Some(
                            incremental_node_certificate_state
                                .missing_peers_signatures(&self.validator_verifier),
                        ),
                    )
                    .await;
            },
            Status::SendingCertificate(certified_node, ack_set) => {
                self.network_sender
                    .send_certified_node(
                        certified_node.clone(),
                        Some(ack_set.missing_peers(&self.validator_verifier)),
                        true,
                    )
                    .await;
            },
        };
    }

    async fn handle_certified_node_ack_msg(&mut self, ack: CertifiedNodeAck) {
        match &mut self.in_mem.status {
            Status::SendingCertificate(certified_node, ack_set) => {
                // TODO: check ack is up to date!
                if ack.digest() == certified_node.digest() {
                    ack_set.add(ack);
                    if ack_set.missing_peers(&self.validator_verifier).is_empty() {
                        self.in_mem.status = Status::NothingToSend;
                        self.persist_state(); //TODO: only write the part being changed.
                    }
                }
            },
            _ => {},
        }
    }

    pub(crate) async fn start(
        mut self,
        mut network_msg_rx: aptos_channel::Receiver<PeerId, VerifiedEvent>,
        mut command_rx: Receiver<ReliableBroadcastCommand>,
        close_rx: oneshot::Receiver<oneshot::Sender<()>>,
    ) {

        let in_mem = if let Some(in_mem) = self.storage.load_all() {
            in_mem
        } else {
            let in_mem = ReliableBroadcastInMem::new(self.in_mem.my_id, self.in_mem.epoch);
            self.persist_state(); //TODO: only write the part being changed.
            in_mem
        };

        // TODO: think about tick readability and races.
        let mut interval = time::interval(Duration::from_millis(500)); // TODO: time out should be slightly more than one network round trip.
        let mut close_rx = close_rx.into_stream();

        loop {
            tokio::select! {
                biased;

                _ = interval.tick() => {
                    self.handle_tick().await;
                },

                Some(command) = command_rx.recv() => {
                    match command {
                        ReliableBroadcastCommand::BroadcastRequest(node) => {
                            self.handle_broadcast_request(node).await;
                            interval.reset();
                        }
                    }
                },

                Some(msg) = network_msg_rx.next() => {
                    match msg {
                        VerifiedEvent::NodeMsg(node) => {
                            self.handle_node_message(*node).await
                        },

                        VerifiedEvent::SignedNodeDigestMsg(signed_node_digest) => {
                            if let Some(certified_node) = self.handle_signed_digest(*signed_node_digest).await {
                                self.network_sender.send_certified_node(certified_node, None, true).await;
                                interval.reset();
                            }

                        },


                        VerifiedEvent::CertifiedNodeAckMsg(ack) => {
                            self.handle_certified_node_ack_msg(*ack).await;
                        },

                        _ => unreachable!("reliable broadcast got wrong messsgae"),
                    }

                },

                close_req = close_rx.select_next_some() => {
                    if let Ok(ack_sender) = close_req {
                        ack_sender.send(()).expect("[ReliableBroadcast] Fail to ack shutdown");
                    }
                    break;
                }

            }
        }
    }
}
