// Copyright © Aptos Foundation

// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_consensus_types::node::{
    CertifiedNodeAck, NodeCertificate, SignedNodeDigest, SignedNodeDigestError,
    SignedNodeDigestInfo,
};
use aptos_crypto::{bls12381, HashValue};
use aptos_types::{
    aggregate_signature::PartialSignatures, PeerId, validator_verifier::ValidatorVerifier,
};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::collections::hash_map::{Entry, Iter};
use serde::{Deserialize, Serialize};
use aptos_schemadb::schema::ValueCodec;
use crate::dag::dag::{MissingDagNodeStatus, MissingNodeIdToStatusMapSchema};
use crate::dag::dag_storage::{ContainsKey, ItemId};
// pub(crate) trait MissingPeers {
//     fn get_peers_signatures() -> HashSet<PeerId>;
// }

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct IncrementalNodeCertificateState {
    signed_node_digest_info: SignedNodeDigestInfo,
    aggregated_signature: BTreeMap<PeerId, bls12381::Signature>,
}

#[allow(dead_code)]
impl IncrementalNodeCertificateState {
    pub fn new(digest: HashValue) -> Self {
        Self {
            signed_node_digest_info: SignedNodeDigestInfo::new(digest),
            aggregated_signature: BTreeMap::new(),
        }
    }

    pub(crate) fn missing_peers_signatures(
        &self,
        validator_verifier: &ValidatorVerifier,
    ) -> Vec<PeerId> {
        let all_peers: HashSet<&PeerId> = validator_verifier
            .address_to_validator_index()
            .keys()
            .collect();
        let singers: HashSet<&PeerId> = self.aggregated_signature.keys().collect();
        all_peers.difference(&singers).cloned().cloned().collect()
    }

    //Signature we already verified
    pub(crate) fn add_signature(
        &mut self,
        signed_node_digest: SignedNodeDigest,
    ) -> Result<(), SignedNodeDigestError> {
        if signed_node_digest.info() != &self.signed_node_digest_info {
            return Err(SignedNodeDigestError::WrongDigest);
        }

        if self
            .aggregated_signature
            .contains_key(&signed_node_digest.peer_id())
        {
            return Err(SignedNodeDigestError::DuplicatedSignature);
        }

        self.aggregated_signature
            .insert(signed_node_digest.peer_id(), signed_node_digest.signature());
        Ok(())
    }

    pub(crate) fn ready(&self, validator_verifier: &ValidatorVerifier) -> bool {
        validator_verifier
            .check_voting_power(self.aggregated_signature.keys())
            .is_ok()
    }

    pub(crate) fn take(&self, validator_verifier: &ValidatorVerifier) -> NodeCertificate {
        let proof = match validator_verifier
            .aggregate_signatures(&PartialSignatures::new(self.aggregated_signature.clone()))
        {
            Ok(sig) => NodeCertificate::new(self.signed_node_digest_info.clone(), sig),
            Err(e) => unreachable!("Cannot aggregate signatures on digest err = {:?}", e),
        };
        proof
    }
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct AckSet {
    digest: HashValue,
    set: HashSet<PeerId>,
}

impl AckSet {
    pub fn new(digest: HashValue) -> Self {
        Self {
            digest,
            set: HashSet::new(),
        }
    }

    pub fn add(&mut self, ack: CertifiedNodeAck) {
        if ack.digest() == self.digest {
            self.set.insert(ack.peer_id());
        }
    }

    pub fn missing_peers(&self, verifier: &ValidatorVerifier) -> Vec<PeerId> {
        let all_peers: HashSet<PeerId> = verifier
            .address_to_validator_index()
            .keys()
            .cloned()
            .collect();
        all_peers.difference(&self.set).cloned().collect()
    }
}


#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub(crate) struct MissingNodeIdToStatusMap {
    id: ItemId,
    inner: HashMap<HashValue, MissingDagNodeStatus>,
}

impl ContainsKey for MissingNodeIdToStatusMap {
    type Key = ItemId;

    fn key(&self) -> ItemId {
        self.id
    }
}

impl MissingNodeIdToStatusMap {
    pub(crate) fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4().into_bytes(),
            inner: HashMap::new(),
        }
    }

    pub(crate) fn get(&self, k: &HashValue) -> Option<&MissingDagNodeStatus> {
        self.inner.get(k)
    }

    pub(crate) fn entry(&mut self, k: HashValue) -> Entry<'_, HashValue, MissingDagNodeStatus> {
        self.inner.entry(k)
    }

    pub(crate) fn iter(&self) -> Iter<'_, HashValue, MissingDagNodeStatus> {
        self.inner.iter()
    }

    pub(crate) fn insert(&mut self, k: HashValue, v: MissingDagNodeStatus) -> Option<MissingDagNodeStatus> {
        self.inner.insert(k, v)
    }
}

impl ValueCodec<MissingNodeIdToStatusMapSchema> for MissingNodeIdToStatusMap {
    fn encode_value(&self) -> anyhow::Result<Vec<u8>> {
        let buf = bcs::to_bytes(self)?;
        Ok(buf)
    }

    fn decode_value(data: &[u8]) -> anyhow::Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}
