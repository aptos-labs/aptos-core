// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block_data::{BlockData, BlockType},
    common::{Author, Payload, Round},
    opt_block_data::OptBlockData,
    quorum_cert::QuorumCert,
};
use anyhow::{bail, ensure, format_err, Result};
use aptos_bitvec::BitVec;
use aptos_crypto::{bls12381, hash::CryptoHash, HashValue};
use aptos_infallible::duration_since_epoch;
use aptos_types::{
    account_address::AccountAddress,
    block_info::BlockInfo,
    block_metadata::BlockMetadata,
    block_metadata_ext::BlockMetadataExt,
    epoch_state::EpochState,
    ledger_info::LedgerInfo,
    randomness::Randomness,
    transaction::{SignedTransaction, Transaction, Version},
    validator_signer::ValidatorSigner,
    validator_txn::ValidatorTransaction,
    validator_verifier::ValidatorVerifier,
};
use mirai_annotations::debug_checked_verify_eq;
use serde::{Deserialize, Deserializer, Serialize};
use std::{
    convert::TryFrom,
    fmt::{self, Display, Formatter},
    iter::once,
};

#[path = "block_test_utils.rs"]
#[cfg(any(test, feature = "fuzzing"))]
pub mod block_test_utils;

#[cfg(test)]
#[path = "block_test.rs"]
pub mod block_test;

#[derive(Serialize, Clone, PartialEq, Eq)]
/// Block has the core data of a consensus block that should be persistent when necessary.
/// Each block must know the id of its parent and keep the QuorurmCertificate to that parent.
pub struct Block {
    /// This block's id as a hash value, it is generated at call time
    #[serde(skip)]
    id: HashValue,
    /// The container for the actual block
    block_data: BlockData,
    /// Signature that the hash of this block has been authored by the owner of the private key,
    /// this is only set within Proposal blocks
    signature: Option<bls12381::Signature>,
}

impl fmt::Debug for Block {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl Display for Block {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        let author = self
            .author()
            .map(|addr| format!("{}", addr))
            .unwrap_or_else(|| "(NIL)".to_string());
        write!(
            f,
            "[id: {}, author: {}, epoch: {}, round: {:02}, parent_id: {}, timestamp: {}]",
            self.id,
            author,
            self.epoch(),
            self.round(),
            self.parent_id(),
            self.timestamp_usecs(),
        )
    }
}

impl Block {
    pub fn author(&self) -> Option<Author> {
        self.block_data.author()
    }

    pub fn epoch(&self) -> u64 {
        self.block_data.epoch()
    }

    pub fn id(&self) -> HashValue {
        self.id
    }

    // Is this block a parent of the parameter block?
    #[cfg(test)]
    pub fn is_parent_of(&self, block: &Self) -> bool {
        block.parent_id() == self.id
    }

    pub fn parent_id(&self) -> HashValue {
        self.block_data.parent_id()
    }

    pub fn payload(&self) -> Option<&Payload> {
        self.block_data.payload()
    }

    pub fn payload_size(&self) -> usize {
        match self.block_data.payload() {
            None => 0,
            Some(payload) => match payload {
                Payload::InQuorumStore(pos) => pos.proofs.len(),
                Payload::DirectMempool(_txns) => 0,
                Payload::InQuorumStoreWithLimit(pos) => pos.proof_with_data.proofs.len(),
                Payload::QuorumStoreInlineHybrid(inline_batches, proof_with_data, _)
                | Payload::QuorumStoreInlineHybridV2(inline_batches, proof_with_data, _) => {
                    inline_batches.len() + proof_with_data.proofs.len()
                },
                Payload::OptQuorumStore(opt_quorum_store_payload) => {
                    opt_quorum_store_payload.num_txns()
                },
            },
        }
    }

    pub fn quorum_cert(&self) -> &QuorumCert {
        self.block_data.quorum_cert()
    }

    pub fn round(&self) -> Round {
        self.block_data.round()
    }

    pub fn signature(&self) -> Option<&bls12381::Signature> {
        self.signature.as_ref()
    }

    pub fn timestamp_usecs(&self) -> u64 {
        self.block_data.timestamp_usecs()
    }

    pub fn gen_block_info(
        &self,
        executed_state_id: HashValue,
        version: Version,
        next_epoch_state: Option<EpochState>,
    ) -> BlockInfo {
        BlockInfo::new(
            self.epoch(),
            self.round(),
            self.id(),
            executed_state_id,
            version,
            self.timestamp_usecs(),
            next_epoch_state,
        )
    }

    pub fn block_data(&self) -> &BlockData {
        &self.block_data
    }

    pub fn is_genesis_block(&self) -> bool {
        self.block_data.is_genesis_block()
    }

    pub fn is_nil_block(&self) -> bool {
        self.block_data.is_nil_block()
    }

    pub fn is_opt_block(&self) -> bool {
        self.block_data.is_opt_block()
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn make_genesis_block() -> Self {
        Self::make_genesis_block_from_ledger_info(&LedgerInfo::mock_genesis(None))
    }

    /// Construct new genesis block for next epoch deterministically from the end-epoch LedgerInfo
    /// We carry over most fields except round and block id
    pub fn make_genesis_block_from_ledger_info(ledger_info: &LedgerInfo) -> Self {
        let block_data = BlockData::new_genesis_from_ledger_info(ledger_info);
        Block {
            id: block_data.hash(),
            block_data,
            signature: None,
        }
    }

    #[cfg(any(test, feature = "fuzzing"))]
    // This method should only used by tests and fuzzers to produce arbitrary Block types.
    pub fn new_for_testing(
        id: HashValue,
        block_data: BlockData,
        signature: Option<bls12381::Signature>,
    ) -> Self {
        Block {
            id,
            block_data,
            signature,
        }
    }

    /// The NIL blocks are special: they're not carrying any real payload and are generated
    /// independently by different validators just to fill in the round with some QC.
    pub fn new_nil(
        round: Round,
        quorum_cert: QuorumCert,
        failed_authors: Vec<(Round, Author)>,
    ) -> Self {
        let block_data = BlockData::new_nil(round, quorum_cert, failed_authors);

        Block {
            id: block_data.hash(),
            block_data,
            signature: None,
        }
    }

    pub fn new_for_dag(
        epoch: u64,
        round: Round,
        timestamp: u64,
        validator_txns: Vec<ValidatorTransaction>,
        payload: Payload,
        author: Author,
        failed_authors: Vec<(Round, Author)>,
        parent_block_id: HashValue,
        parents_bitvec: BitVec,
        node_digests: Vec<HashValue>,
    ) -> Self {
        let block_data = BlockData::new_for_dag(
            epoch,
            round,
            timestamp,
            validator_txns,
            payload,
            author,
            failed_authors,
            parent_block_id,
            parents_bitvec,
            node_digests,
        );
        Self {
            id: block_data.hash(),
            block_data,
            signature: None,
        }
    }

    pub fn new_proposal(
        payload: Payload,
        round: Round,
        timestamp_usecs: u64,
        quorum_cert: QuorumCert,
        validator_signer: &ValidatorSigner,
        failed_authors: Vec<(Round, Author)>,
    ) -> anyhow::Result<Self> {
        let block_data = BlockData::new_proposal(
            payload,
            validator_signer.author(),
            failed_authors,
            round,
            timestamp_usecs,
            quorum_cert,
        );

        Self::new_proposal_from_block_data(block_data, validator_signer)
    }

    pub fn new_proposal_ext(
        validator_txns: Vec<ValidatorTransaction>,
        payload: Payload,
        round: Round,
        timestamp_usecs: u64,
        quorum_cert: QuorumCert,
        validator_signer: &ValidatorSigner,
        failed_authors: Vec<(Round, Author)>,
    ) -> anyhow::Result<Self> {
        let block_data = BlockData::new_proposal_ext(
            validator_txns,
            payload,
            validator_signer.author(),
            failed_authors,
            round,
            timestamp_usecs,
            quorum_cert,
        );

        Self::new_proposal_from_block_data(block_data, validator_signer)
    }

    pub fn new_proposal_from_block_data(
        block_data: BlockData,
        validator_signer: &ValidatorSigner,
    ) -> anyhow::Result<Self> {
        let signature = validator_signer.sign(&block_data)?;
        Ok(Self::new_proposal_from_block_data_and_signature(
            block_data, signature,
        ))
    }

    pub fn new_proposal_from_block_data_and_signature(
        block_data: BlockData,
        signature: bls12381::Signature,
    ) -> Self {
        Block {
            id: block_data.hash(),
            block_data,
            signature: Some(signature),
        }
    }

    pub fn new_from_opt(
        block_data: OptBlockData,
        quorum_cert: QuorumCert,
        failed_authors: Vec<(Round, Author)>,
    ) -> Result<Self> {
        let block_data = BlockData::new_from_opt(block_data, quorum_cert, failed_authors)?;
        Ok(Block {
            id: block_data.hash(),
            block_data,
            signature: None,
        })
    }

    pub fn validator_txns(&self) -> Option<&Vec<ValidatorTransaction>> {
        self.block_data.validator_txns()
    }

    /// Verifies that the proposal and the QC are correctly signed.
    /// If this is the genesis block, we skip these checks.
    pub fn validate_signature(&self, validator: &ValidatorVerifier) -> anyhow::Result<()> {
        match self.block_data.block_type() {
            BlockType::Genesis => bail!("We should not accept genesis from others"),
            BlockType::NilBlock { .. } => self.quorum_cert().verify(validator),
            BlockType::Proposal { author, .. } => {
                let signature = self
                    .signature
                    .as_ref()
                    .ok_or_else(|| format_err!("Missing signature in Proposal"))?;
                validator.verify(*author, &self.block_data, signature)?;
                self.quorum_cert().verify(validator)
            },
            BlockType::ProposalExt(proposal_ext) => {
                let signature = self
                    .signature
                    .as_ref()
                    .ok_or_else(|| format_err!("Missing signature in Proposal"))?;
                validator.verify(*proposal_ext.author(), &self.block_data, signature)?;
                self.quorum_cert().verify(validator)
            },
            BlockType::OptProposal { .. } => {
                // Optimistic proposal is not signed by proposer
                self.block_data()
                    .grandparent_qc()
                    .map_or(Ok(()), |qc| qc.verify(validator))?;
                self.quorum_cert().verify(validator)
            },
            BlockType::DAGBlock { .. } => bail!("We should not accept DAG block from others"),
        }
    }

    /// Makes sure that the proposal makes sense, independently of the current state.
    /// If this is the genesis block, we skip these checks.
    #[allow(unexpected_cfgs)]
    pub fn verify_well_formed(&self) -> anyhow::Result<()> {
        ensure!(
            !self.is_genesis_block(),
            "We must not accept genesis from others"
        );
        let parent = self.quorum_cert().certified_block();
        ensure!(
            parent.round() < self.round(),
            "Block must have a greater round than parent's block"
        );
        ensure!(
            parent.epoch() == self.epoch(),
            "block's parent should be in the same epoch"
        );
        if parent.has_reconfiguration() {
            ensure!(
                self.payload().map_or(true, |p| p.is_empty()),
                "Reconfiguration suffix should not carry payload"
            );
        }

        if let Some(payload) = self.payload() {
            payload.verify_epoch(self.epoch())?;
        }

        if let Some(failed_authors) = self.block_data().failed_authors() {
            // when validating for being well formed,
            // allow for missing failed authors,
            // for whatever reason (from different max configuration, etc),
            // but don't allow anything that shouldn't be there.
            //
            // we validate the full correctness of this field in round_manager.process_proposal()
            let succ_round = self.round() + u64::from(self.is_nil_block());
            let skipped_rounds = succ_round.checked_sub(parent.round() + 1);
            ensure!(
                skipped_rounds.is_some(),
                "Block round is smaller than block's parent round"
            );
            ensure!(
                failed_authors.len() <= skipped_rounds.unwrap() as usize,
                "Block has more failed authors than missed rounds"
            );
            let mut bound = parent.round();
            for (round, _) in failed_authors {
                ensure!(
                    bound < *round && *round < succ_round,
                    "Incorrect round in failed authors"
                );
                bound = *round;
            }
        }

        if self.is_nil_block() || parent.has_reconfiguration() {
            ensure!(
                self.timestamp_usecs() == parent.timestamp_usecs(),
                "Nil/reconfig suffix block must have same timestamp as parent"
            );
        } else {
            ensure!(
                self.timestamp_usecs() > parent.timestamp_usecs(),
                "Blocks must have strictly increasing timestamps"
            );

            let current_ts = duration_since_epoch();

            // we can say that too far is 5 minutes in the future
            const TIMEBOUND: u64 = 300_000_000;
            ensure!(
                self.timestamp_usecs() <= (current_ts.as_micros() as u64).saturating_add(TIMEBOUND),
                "Blocks must not be too far in the future"
            );
        }
        ensure!(
            !self.quorum_cert().ends_epoch(),
            "Block cannot be proposed in an epoch that has ended"
        );
        debug_checked_verify_eq!(
            self.id(),
            self.block_data.hash(),
            "Block id mismatch the hash"
        );
        Ok(())
    }

    pub fn combine_to_input_transactions(
        validator_txns: Vec<ValidatorTransaction>,
        txns: Vec<SignedTransaction>,
        metadata: BlockMetadataExt,
    ) -> Vec<Transaction> {
        once(Transaction::from(metadata))
            .chain(
                validator_txns
                    .into_iter()
                    .map(Transaction::ValidatorTransaction),
            )
            .chain(txns.into_iter().map(Transaction::UserTransaction))
            .collect()
    }

    fn previous_bitvec(&self) -> BitVec {
        if let BlockType::DAGBlock { parents_bitvec, .. } = self.block_data.block_type() {
            parents_bitvec.clone()
        } else if let BlockType::OptProposal { grandparent_qc, .. } = self.block_data.block_type() {
            grandparent_qc.ledger_info().get_voters_bitvec().clone()
        } else {
            self.quorum_cert().ledger_info().get_voters_bitvec().clone()
        }
    }

    pub fn new_block_metadata(&self, validators: &[AccountAddress]) -> BlockMetadata {
        BlockMetadata::new(
            self.id(),
            self.epoch(),
            self.round(),
            self.author().unwrap_or(AccountAddress::ZERO),
            self.previous_bitvec().into(),
            // For nil block, we use 0x0 which is convention for nil address in move.
            self.block_data()
                .failed_authors()
                .map_or(vec![], |failed_authors| {
                    Self::failed_authors_to_indices(validators, failed_authors)
                }),
            self.timestamp_usecs(),
        )
    }

    pub fn new_metadata_with_randomness(
        &self,
        validators: &[AccountAddress],
        randomness: Option<Randomness>,
    ) -> BlockMetadataExt {
        BlockMetadataExt::new_v1(
            self.id(),
            self.epoch(),
            self.round(),
            self.author().unwrap_or(AccountAddress::ZERO),
            self.previous_bitvec().into(),
            // For nil block, we use 0x0 which is convention for nil address in move.
            self.block_data()
                .failed_authors()
                .map_or(vec![], |failed_authors| {
                    Self::failed_authors_to_indices(validators, failed_authors)
                }),
            self.timestamp_usecs(),
            randomness,
        )
    }

    fn failed_authors_to_indices(
        validators: &[AccountAddress],
        failed_authors: &[(Round, Author)],
    ) -> Vec<u32> {
        failed_authors
            .iter()
            .map(|(_round, failed_author)| {
                validators
                    .iter()
                    .position(|&v| v == *failed_author)
                    .unwrap_or_else(|| {
                        panic!(
                            "Failed author {} not in validator list {:?}",
                            *failed_author, validators
                        )
                    })
            })
            .map(|index| u32::try_from(index).expect("Index is out of bounds for u32"))
            .collect()
    }
}

impl<'de> Deserialize<'de> for Block {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename = "Block")]
        struct BlockWithoutId {
            block_data: BlockData,
            signature: Option<bls12381::Signature>,
        }

        let BlockWithoutId {
            block_data,
            signature,
        } = BlockWithoutId::deserialize(deserializer)?;

        Ok(Block {
            id: block_data.hash(),
            block_data,
            signature,
        })
    }
}
