// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block::Block,
    common::{Author, Payload, Round},
    payload::{OptQuorumStorePayload, OptQuorumStorePayloadV1},
    quorum_cert::QuorumCert,
    wrapped_ledger_info::WrappedLedgerInfo,
};
use anyhow::ensure;
use aptos_crypto::HashValue;
use aptos_types::{validator_txn::ValidatorTransaction, validator_verifier::ValidatorVerifier};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub enum OptBlockBody {
    V0 {
        validator_txns: Vec<ValidatorTransaction>,
        // T of the block (e.g. one or more transaction(s)
        payload: Payload,
        // Author of the block that can be validated by the author's public key and the signature
        author: Author,
        // QC of the grandparent block
        grandparent_qc: QuorumCert,
    },
}

impl OptBlockBody {
    pub fn author(&self) -> &Author {
        match self {
            OptBlockBody::V0 { author, .. } => author,
        }
    }

    pub fn validator_txns(&self) -> Option<&Vec<ValidatorTransaction>> {
        match self {
            OptBlockBody::V0 { validator_txns, .. } => Some(validator_txns),
        }
    }

    pub fn payload(&self) -> &Payload {
        match self {
            OptBlockBody::V0 { payload, .. } => payload,
        }
    }

    pub fn take_payload(self) -> Payload {
        match self {
            OptBlockBody::V0 { payload, .. } => payload,
        }
    }

    pub fn grandparent_qc(&self) -> &QuorumCert {
        match self {
            OptBlockBody::V0 { grandparent_qc, .. } => grandparent_qc,
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub enum ProposalExt {
    V0 {
        validator_txns: Vec<ValidatorTransaction>,
        /// T of the block (e.g. one or more transaction(s)
        payload: Payload,
        /// Author of the block that can be validated by the author's public key and the signature
        author: Author,
        /// Failed authors from the parent's block to this block.
        /// I.e. the list of consecutive proposers from the
        /// immediately preceeding rounds that didn't produce a successful block.
        failed_authors: Vec<(Round, Author)>,
    },
}

impl ProposalExt {
    pub fn author(&self) -> &Author {
        match self {
            ProposalExt::V0 { author, .. } => author,
        }
    }

    pub fn failed_authors(&self) -> &Vec<(Round, Author)> {
        match self {
            ProposalExt::V0 { failed_authors, .. } => failed_authors,
        }
    }

    pub fn validator_txns(&self) -> Option<&Vec<ValidatorTransaction>> {
        match self {
            ProposalExt::V0 { validator_txns, .. } => Some(validator_txns),
        }
    }

    pub fn payload(&self) -> Option<&Payload> {
        match self {
            ProposalExt::V0 { payload, .. } => Some(payload),
        }
    }

    pub fn take_payload(self) -> Option<Payload> {
        match self {
            ProposalExt::V0 { payload, .. } => Some(payload),
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct ProxyBlockMetadata {
    pub primary_round: Round,
    pub primary_qc: Option<QuorumCert>,
}

impl ProxyBlockMetadata {
    pub fn new(primary_round: Round, primary_qc: Option<QuorumCert>) -> Self {
        Self {
            primary_round,
            primary_qc,
        }
    }

    pub fn primary_qc(&self) -> Option<&QuorumCert> {
        self.primary_qc.as_ref()
    }

    pub fn verify(&self, primary_verifier: &ValidatorVerifier) -> anyhow::Result<()> {
        if let Some(qc) = &self.primary_qc {
            qc.verify(primary_verifier)?;
            ensure!(
                qc.certified_block().round() == self.primary_round - 1,
                "Invalid primary QC round"
            );
        }
        Ok(())
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub enum ProxyBlock {
    V0 {
        metadata: ProxyBlockMetadata,
        payload: OptQuorumStorePayload,
    },
}

impl ProxyBlock {
    pub fn take_inner(self) -> (ProxyBlockMetadata, OptQuorumStorePayload) {
        match self {
            ProxyBlock::V0 { metadata, payload } => (metadata, payload),
        }
    }

    pub fn metadata(&self) -> &ProxyBlockMetadata {
        match self {
            ProxyBlock::V0 { metadata, .. } => metadata,
        }
    }

    pub fn payload(&self) -> &OptQuorumStorePayload {
        match self {
            ProxyBlock::V0 { payload, .. } => payload,
        }
    }

    pub fn payload_mut(&mut self) -> &mut OptQuorumStorePayload {
        match self {
            ProxyBlock::V0 { payload, .. } => payload,
        }
    }

    pub fn take_payload(self) -> OptQuorumStorePayload {
        match self {
            ProxyBlock::V0 { payload, .. } => payload,
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct ProxyBlockFullMetadata {
    pub proxy_block_metadata: ProxyBlockMetadata,
    pub proxy_id: HashValue,
    pub proxy_author: Author,
    pub proxy_round: Round,
    pub proxy_timestamp_usecs: u64,
    pub proxy_qc: QuorumCert,
    pub failed_authors: Vec<(Round, Author)>,
}

impl ProxyBlockFullMetadata {
    pub fn new(
        proxy_block_metadata: ProxyBlockMetadata,
        proxy_id: HashValue,
        proxy_author: Author,
        proxy_round: Round,
        proxy_timestamp_usecs: u64,
        proxy_qc: QuorumCert,
        failed_authors: Vec<(Round, Author)>,
    ) -> Self {
        Self {
            proxy_block_metadata,
            proxy_id,
            proxy_author,
            proxy_round,
            proxy_timestamp_usecs,
            proxy_qc,
            failed_authors,
        }
    }

    pub fn verify(
        &self,
        proxy_verifier: &ValidatorVerifier,
        primary_verifier: &ValidatorVerifier,
    ) -> anyhow::Result<()> {
        self.proxy_qc.verify(proxy_verifier)?;
        self.proxy_block_metadata.verify(primary_verifier)?;
        Ok(())
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct OrderedProxyBlocks {
    pub proxy_blocks: Vec<Block>,
    pub ordering_proof: WrappedLedgerInfo,
    // for verifying ordering proof in case of indirect ordering
    pub aux_proxy_blocks: Vec<Block>,
}

impl OrderedProxyBlocks {
    pub fn new(
        proxy_blocks: Vec<Block>,
        ordering_proof: WrappedLedgerInfo,
        aux_proxy_blocks: Vec<Block>,
    ) -> Self {
        Self {
            proxy_blocks,
            ordering_proof,
            aux_proxy_blocks,
        }
    }

    pub fn verify(
        &self,
        proxy_validator: &ValidatorVerifier,
        parent_id: HashValue,
    ) -> anyhow::Result<()> {
        // verify all proxy blocks are well formed
        for proxy_block in self.proxy_blocks.iter() {
            proxy_block.verify_well_formed()?;
        }
        for aux_proxy_block in self.aux_proxy_blocks.iter() {
            aux_proxy_block.verify_well_formed()?;
        }
        // verify all proxy blocks have correct QC or signatures
        for proxy_block in self.proxy_blocks.iter() {
            proxy_block.validate_signature(proxy_validator)?;
        }
        for aux_proxy_block in self.aux_proxy_blocks.iter() {
            aux_proxy_block.validate_signature(proxy_validator)?;
        }
        // verify the ordering proof
        self.ordering_proof.verify(proxy_validator)?;
        // verify all proxy blocks and aux proxy blocks are linked by QC
        let all_blocks_ref = self.proxy_blocks.iter().chain(self.aux_proxy_blocks.iter());
        let mut expected_parent_id = parent_id;
        for block in all_blocks_ref {
            ensure!(
                block.parent_id() == expected_parent_id,
                "Block parent ID does not match the expected parent ID"
            );
            expected_parent_id = block.id();
        }
        // verify the last proxy block is ordered by the ordering proof
        let last_proxy_block = if self.aux_proxy_blocks.is_empty() {
            self.proxy_blocks.last().expect("Proxy blocks are empty")
        } else {
            self.aux_proxy_blocks.last().expect("Just checked non empty")
        };
        ensure!(
            last_proxy_block.id() == self.ordering_proof.commit_info().id(),
            "Last proxy block ID does not match the ordering proof commit info ID"
        );
        Ok(())
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct PrimaryBlockMetadata {
    pub proxy_block_full_metadata: Vec<ProxyBlockFullMetadata>,
}

impl PrimaryBlockMetadata {
    pub fn new(proxy_block_full_metadata: Vec<ProxyBlockFullMetadata>) -> Self {
        Self {
            proxy_block_full_metadata,
        }
    }

    pub fn verify(
        &self,
        proxy_verifier: &ValidatorVerifier,
        primary_verifier: &ValidatorVerifier,
    ) -> anyhow::Result<()> {
        self.proxy_block_full_metadata
            .iter()
            .try_for_each(|proxy_block_full_metadata| {
                proxy_block_full_metadata.verify(proxy_verifier, primary_verifier)
            })?;
        Ok(())
    }

    pub fn verify_well_formed(&self) -> anyhow::Result<()> {
        // check all metadata are linked by QC
        // check all proxy rounds are increasing
        // check all proxy timestamps are increasing
        let mut parent_id = self
            .proxy_block_full_metadata
            .first()
            .expect("Proxy block full metadata is empty")
            .proxy_qc
            .certified_block()
            .id();
        let mut previous_proxy_round = 0;
        let mut previous_proxy_timestamp_usecs = 0;
        for proxy_block_full_metadata in self.proxy_block_full_metadata.iter() {
            ensure!(
                proxy_block_full_metadata.proxy_qc.certified_block().id() == parent_id,
                "Proxy IDs are not linked by QC"
            );
            ensure!(
                proxy_block_full_metadata.proxy_round > previous_proxy_round,
                "Proxy rounds are not increasing"
            );
            ensure!(
                proxy_block_full_metadata.proxy_timestamp_usecs > previous_proxy_timestamp_usecs,
                "Proxy timestamps are not increasing"
            );
            parent_id = proxy_block_full_metadata.proxy_id;
            previous_proxy_round = proxy_block_full_metadata.proxy_round;
            previous_proxy_timestamp_usecs = proxy_block_full_metadata.proxy_timestamp_usecs;
        }
        Ok(())
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub enum PrimaryBlock {
    V0 {
        metadata: PrimaryBlockMetadata,
        payload: OptQuorumStorePayload,
    },
}

impl PrimaryBlock {
    pub fn from_proxy_blocks(proxy_blocks: Vec<Block>) -> Self {
        let mut payload = OptQuorumStorePayload::V1(OptQuorumStorePayloadV1::new_empty());
        let mut proxy_block_full_metadata_vec = Vec::new();
        for block in proxy_blocks {
            let proxy_id = block.id();
            let proxy_author = block.author().expect("Proxy block author expected");
            let proxy_round = block.round();
            let proxy_timestamp_usecs = block.timestamp_usecs();
            let proxy_qc = block.quorum_cert().clone();
            let failed_authors = block
                .block_data()
                .failed_authors()
                .cloned()
                .unwrap_or(Vec::new());
            let proxy_block = block.take_proxy_block().expect("Proxy block expected");
            let (metadata, proxy_payload) = proxy_block.take_inner();
            let proxy_block_full_metadata = ProxyBlockFullMetadata::new(
                metadata,
                proxy_id,
                proxy_author,
                proxy_round,
                proxy_timestamp_usecs,
                proxy_qc,
                failed_authors,
            );

            proxy_block_full_metadata_vec.push(proxy_block_full_metadata);
            payload = payload.extend(proxy_payload);
        }
        let primary_block_metadata = PrimaryBlockMetadata::new(proxy_block_full_metadata_vec);
        PrimaryBlock::V0 {
            metadata: primary_block_metadata,
            payload,
        }
    }

    pub fn metadata(&self) -> &PrimaryBlockMetadata {
        match self {
            PrimaryBlock::V0 { metadata, .. } => metadata,
        }
    }

    pub fn payload(&self) -> &OptQuorumStorePayload {
        match self {
            PrimaryBlock::V0 { payload, .. } => payload,
        }
    }

    pub fn payload_mut(&mut self) -> &mut OptQuorumStorePayload {
        match self {
            PrimaryBlock::V0 { payload, .. } => payload,
        }
    }

    pub fn take_payload(self) -> OptQuorumStorePayload {
        match self {
            PrimaryBlock::V0 { payload, .. } => payload,
        }
    }
}
