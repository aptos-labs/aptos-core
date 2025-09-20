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
pub struct MoonBlockMetadata {
    pub earth_round: Round,
    pub earth_qc: Option<QuorumCert>,
}

impl MoonBlockMetadata {
    pub fn new(earth_round: Round, earth_qc: Option<QuorumCert>) -> Self {
        Self {
            earth_round,
            earth_qc,
        }
    }

    pub fn earth_qc(&self) -> Option<&QuorumCert> {
        self.earth_qc.as_ref()
    }

    pub fn verify(&self, earth_verifier: &ValidatorVerifier) -> anyhow::Result<()> {
        if let Some(qc) = &self.earth_qc {
            qc.verify(earth_verifier)?;
            ensure!(
                qc.certified_block().round() == self.earth_round - 1,
                "Invalid earth QC round"
            );
        }
        Ok(())
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub enum MoonBlock {
    V0 {
        metadata: MoonBlockMetadata,
        payload: OptQuorumStorePayload,
    },
}

impl MoonBlock {
    pub fn take_inner(self) -> (MoonBlockMetadata, OptQuorumStorePayload) {
        match self {
            MoonBlock::V0 { metadata, payload } => (metadata, payload),
        }
    }

    pub fn metadata(&self) -> &MoonBlockMetadata {
        match self {
            MoonBlock::V0 { metadata, .. } => metadata,
        }
    }

    pub fn payload(&self) -> &OptQuorumStorePayload {
        match self {
            MoonBlock::V0 { payload, .. } => payload,
        }
    }

    pub fn payload_mut(&mut self) -> &mut OptQuorumStorePayload {
        match self {
            MoonBlock::V0 { payload, .. } => payload,
        }
    }

    pub fn take_payload(self) -> OptQuorumStorePayload {
        match self {
            MoonBlock::V0 { payload, .. } => payload,
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct MoonBlockFullMetadata {
    pub moon_block_metadata: MoonBlockMetadata,
    pub moon_id: HashValue,
    pub moon_author: Author,
    pub moon_round: Round,
    pub moon_timestamp_usecs: u64,
    pub moon_qc: QuorumCert,
    pub failed_authors: Vec<(Round, Author)>,
}

impl MoonBlockFullMetadata {
    pub fn new(
        moon_block_metadata: MoonBlockMetadata,
        moon_id: HashValue,
        moon_author: Author,
        moon_round: Round,
        moon_timestamp_usecs: u64,
        moon_qc: QuorumCert,
        failed_authors: Vec<(Round, Author)>,
    ) -> Self {
        Self {
            moon_block_metadata,
            moon_id,
            moon_author,
            moon_round,
            moon_timestamp_usecs,
            moon_qc,
            failed_authors,
        }
    }

    pub fn verify(
        &self,
        moon_verifier: &ValidatorVerifier,
        earth_verifier: &ValidatorVerifier,
    ) -> anyhow::Result<()> {
        self.moon_qc.verify(moon_verifier)?;
        self.moon_block_metadata.verify(earth_verifier)?;
        Ok(())
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct OrderedMoonBlocks {
    pub moon_blocks: Vec<Block>,
    pub ordering_proof: WrappedLedgerInfo,
    // for verifying ordering proof in case of indirect ordering
    pub aux_moon_blocks: Vec<Block>,
}

impl OrderedMoonBlocks {
    pub fn new(
        moon_blocks: Vec<Block>,
        ordering_proof: WrappedLedgerInfo,
        aux_moon_blocks: Vec<Block>,
    ) -> Self {
        Self {
            moon_blocks,
            ordering_proof,
            aux_moon_blocks,
        }
    }

    pub fn verify(
        &self,
        moon_validator: &ValidatorVerifier,
        parent_id: HashValue,
    ) -> anyhow::Result<()> {
        // verify all moon blocks are well formed
        for moon_block in self.moon_blocks.iter() {
            moon_block.verify_well_formed()?;
        }
        for aux_moon_block in self.aux_moon_blocks.iter() {
            aux_moon_block.verify_well_formed()?;
        }
        // verify all moon blocks have correct QC or signatures
        for moon_block in self.moon_blocks.iter() {
            moon_block.validate_signature(moon_validator)?;
        }
        for aux_moon_block in self.aux_moon_blocks.iter() {
            aux_moon_block.validate_signature(moon_validator)?;
        }
        // verify the ordering proof
        self.ordering_proof.verify(moon_validator)?;
        // verify all moon blocks and aux moon blocks are linked by QC
        let all_blocks_ref = self.moon_blocks.iter().chain(self.aux_moon_blocks.iter());
        let mut expected_parent_id = parent_id;
        for block in all_blocks_ref {
            ensure!(
                block.parent_id() == expected_parent_id,
                "Block parent ID does not match the expected parent ID"
            );
            expected_parent_id = block.id();
        }
        // verify the last moon block is ordered by the ordering proof
        let last_moon_block = if self.aux_moon_blocks.is_empty() {
            self.moon_blocks.last().expect("Moon blocks are empty")
        } else {
            self.aux_moon_blocks.last().expect("Just checked non empty")
        };
        ensure!(
            last_moon_block.id() == self.ordering_proof.commit_info().id(),
            "Last moon block ID does not match the ordering proof commit info ID"
        );
        Ok(())
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct EarthBlockMetadata {
    pub moon_block_full_metadata: Vec<MoonBlockFullMetadata>,
}

impl EarthBlockMetadata {
    pub fn new(moon_block_full_metadata: Vec<MoonBlockFullMetadata>) -> Self {
        Self {
            moon_block_full_metadata,
        }
    }

    pub fn verify(
        &self,
        moon_verifier: &ValidatorVerifier,
        earth_verifier: &ValidatorVerifier,
    ) -> anyhow::Result<()> {
        self.moon_block_full_metadata
            .iter()
            .try_for_each(|moon_block_full_metadata| {
                moon_block_full_metadata.verify(moon_verifier, earth_verifier)
            })?;
        Ok(())
    }

    pub fn verify_well_formed(&self) -> anyhow::Result<()> {
        // check all metadata are linked by QC
        // check all moon rounds are increasing
        // check all moon timestamps are increasing
        let mut parent_id = self
            .moon_block_full_metadata
            .first()
            .expect("Moon block full metadata is empty")
            .moon_qc
            .certified_block()
            .id();
        let mut previous_moon_round = 0;
        let mut previous_moon_timestamp_usecs = 0;
        for moon_block_full_metadata in self.moon_block_full_metadata.iter() {
            ensure!(
                moon_block_full_metadata.moon_qc.certified_block().id() == parent_id,
                "Moon IDs are not linked by QC"
            );
            ensure!(
                moon_block_full_metadata.moon_round > previous_moon_round,
                "Moon rounds are not increasing"
            );
            ensure!(
                moon_block_full_metadata.moon_timestamp_usecs > previous_moon_timestamp_usecs,
                "Moon timestamps are not increasing"
            );
            parent_id = moon_block_full_metadata.moon_id;
            previous_moon_round = moon_block_full_metadata.moon_round;
            previous_moon_timestamp_usecs = moon_block_full_metadata.moon_timestamp_usecs;
        }
        Ok(())
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub enum EarthBlock {
    V0 {
        metadata: EarthBlockMetadata,
        payload: OptQuorumStorePayload,
    },
}

impl EarthBlock {
    pub fn from_moon_blocks(moon_blocks: Vec<Block>) -> Self {
        let mut payload = OptQuorumStorePayload::V1(OptQuorumStorePayloadV1::new_empty());
        let mut moon_block_full_metadata_vec = Vec::new();
        for block in moon_blocks {
            let moon_id = block.id();
            let moon_author = block.author().expect("Moon block author expected");
            let moon_round = block.round();
            let moon_timestamp_usecs = block.timestamp_usecs();
            let moon_qc = block.quorum_cert().clone();
            let failed_authors = block
                .block_data()
                .failed_authors()
                .cloned()
                .unwrap_or(Vec::new());
            let moon_block = block.take_moon_block().expect("Moon block expected");
            let (metadata, moon_payload) = moon_block.take_inner();
            let moon_block_full_metadata = MoonBlockFullMetadata::new(
                metadata,
                moon_id,
                moon_author,
                moon_round,
                moon_timestamp_usecs,
                moon_qc,
                failed_authors,
            );

            moon_block_full_metadata_vec.push(moon_block_full_metadata);
            payload = payload.extend(moon_payload);
        }
        let earth_block_metadata = EarthBlockMetadata::new(moon_block_full_metadata_vec);
        EarthBlock::V0 {
            metadata: earth_block_metadata,
            payload,
        }
    }

    pub fn metadata(&self) -> &EarthBlockMetadata {
        match self {
            EarthBlock::V0 { metadata, .. } => metadata,
        }
    }

    pub fn payload(&self) -> &OptQuorumStorePayload {
        match self {
            EarthBlock::V0 { payload, .. } => payload,
        }
    }

    pub fn payload_mut(&mut self) -> &mut OptQuorumStorePayload {
        match self {
            EarthBlock::V0 { payload, .. } => payload,
        }
    }

    pub fn take_payload(self) -> OptQuorumStorePayload {
        match self {
            EarthBlock::V0 { payload, .. } => payload,
        }
    }
}
