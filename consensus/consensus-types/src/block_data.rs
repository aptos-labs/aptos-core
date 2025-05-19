// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::{Author, Payload, Round},
    proposal_ext::ProposalExt,
    quorum_cert::QuorumCert,
    vote_data::VoteData,
};
use aptos_bitvec::BitVec;
use aptos_crypto::hash::HashValue;
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use aptos_types::{
    aggregate_signature::AggregateSignature,
    block_info::BlockInfo,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    validator_txn::ValidatorTransaction,
};
use mirai_annotations::*;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub enum BlockType {
    Proposal {
        /// T of the block (e.g. one or more transaction(s)
        payload: Payload,
        /// Author of the block that can be validated by the author's public key and the signature
        author: Author,
        /// Failed authors from the parent's block to this block.
        /// I.e. the list of consecutive proposers from the
        /// immediately preceeding rounds that didn't produce a successful block.
        failed_authors: Vec<(Round, Author)>,
    },
    /// NIL blocks don't have authors or signatures: they're generated upon timeouts to fill in the
    /// gaps in the rounds.
    NilBlock {
        /// Failed authors from the parent's block to this block (including this block)
        /// I.e. the list of consecutive proposers from the
        /// immediately preceeding rounds that didn't produce a successful block.
        failed_authors: Vec<(Round, Author)>,
    },
    /// A genesis block is the first committed block in any epoch that is identically constructed on
    /// all validators by any (potentially different) LedgerInfo that justifies the epoch change
    /// from the previous epoch.  The genesis block is used as the first root block of the
    /// BlockTree for all epochs.
    Genesis,

    /// Proposal with extensions (e.g. system transactions).
    ProposalExt(ProposalExt),

    /// A virtual block that's constructed by nodes from DAG, this is purely a local thing so
    /// we hide it from serde
    #[serde(skip_deserializing)]
    DAGBlock {
        author: Author,
        failed_authors: Vec<(Round, Author)>,
        validator_txns: Vec<ValidatorTransaction>,
        payload: Payload,
        node_digests: Vec<HashValue>,
        parent_block_id: HashValue,
        parents_bitvec: BitVec,
    },
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq, CryptoHasher, BCSCryptoHash)]
/// Block has the core data of a consensus block that should be persistent when necessary.
/// Each block must know the id of its parent and keep the QuorurmCertificate to that parent.
pub struct BlockData {
    /// Epoch number corresponds to the set of validators that are active for this block.
    epoch: u64,
    /// The round of a block is an internal monotonically increasing counter used by Consensus
    /// protocol.
    round: Round,
    /// The approximate physical time a block is proposed by a proposer.  This timestamp is used
    /// for
    /// * Time-dependent logic in smart contracts (the current time of execution)
    /// * Clients determining if they are relatively up-to-date with respect to the block chain.
    ///
    /// It makes the following guarantees:
    ///   1. Time Monotonicity: Time is monotonically increasing in the block chain.
    ///      (i.e. If H1 < H2, H1.Time < H2.Time).
    ///   2. If a block of transactions B is agreed on with timestamp T, then at least
    ///      f+1 honest validators think that T is in the past. An honest validator will
    ///      only vote on a block when its own clock >= timestamp T.
    ///   3. If a block of transactions B has a QC with timestamp T, an honest validator
    ///      will not serve such a block to other validators until its own clock >= timestamp T.
    ///   4. Current: an honest validator is not issuing blocks with a timestamp in the
    ///       future. Currently we consider a block is malicious if it was issued more
    ///       that 5 minutes in the future.
    timestamp_usecs: u64,
    /// Contains the quorum certified ancestor and whether the quorum certified ancestor was
    /// voted on successfully
    quorum_cert: QuorumCert,
    /// If a block is a real proposal, contains its author and signature.
    block_type: BlockType,
}

impl BlockData {
    pub fn author(&self) -> Option<Author> {
        match &self.block_type {
            BlockType::Proposal { author, .. } | BlockType::DAGBlock { author, .. } => {
                Some(*author)
            },
            BlockType::ProposalExt(p) => Some(*p.author()),
            _ => None,
        }
    }

    pub fn block_type(&self) -> &BlockType {
        &self.block_type
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn parent_id(&self) -> HashValue {
        if let BlockType::DAGBlock {
            parent_block_id, ..
        } = self.block_type()
        {
            *parent_block_id
        } else {
            self.quorum_cert.certified_block().id()
        }
    }

    pub fn payload(&self) -> Option<&Payload> {
        match &self.block_type {
            BlockType::Proposal { payload, .. } | BlockType::DAGBlock { payload, .. } => {
                Some(payload)
            },
            BlockType::ProposalExt(p) => p.payload(),
            _ => None,
        }
    }

    pub fn validator_txns(&self) -> Option<&Vec<ValidatorTransaction>> {
        match &self.block_type {
            BlockType::ProposalExt(proposal_ext) => proposal_ext.validator_txns(),
            BlockType::Proposal { .. } | BlockType::NilBlock { .. } | BlockType::Genesis => None,
            BlockType::DAGBlock { validator_txns, .. } => Some(validator_txns),
        }
    }

    pub fn dag_nodes(&self) -> Option<&Vec<HashValue>> {
        if let BlockType::DAGBlock {
            node_digests: nodes_digests,
            ..
        } = &self.block_type
        {
            Some(nodes_digests)
        } else {
            None
        }
    }

    pub fn round(&self) -> Round {
        self.round
    }

    pub fn timestamp_usecs(&self) -> u64 {
        self.timestamp_usecs
    }

    pub fn quorum_cert(&self) -> &QuorumCert {
        &self.quorum_cert
    }

    pub fn is_genesis_block(&self) -> bool {
        matches!(self.block_type, BlockType::Genesis)
    }

    pub fn is_nil_block(&self) -> bool {
        matches!(self.block_type, BlockType::NilBlock { .. })
    }

    /// the list of consecutive proposers from the immediately preceeding
    /// rounds that didn't produce a successful block
    pub fn failed_authors(&self) -> Option<&Vec<(Round, Author)>> {
        match &self.block_type {
            BlockType::Proposal { failed_authors, .. }
            | BlockType::NilBlock { failed_authors, .. }
            | BlockType::DAGBlock { failed_authors, .. } => Some(failed_authors),
            BlockType::ProposalExt(p) => Some(p.failed_authors()),
            BlockType::Genesis => None,
        }
    }

    pub fn new_genesis_from_ledger_info(ledger_info: &LedgerInfo) -> Self {
        assert!(ledger_info.ends_epoch());
        let ancestor = BlockInfo::new(
            ledger_info.epoch(),
            0,                 /* round */
            HashValue::zero(), /* parent block id */
            ledger_info.transaction_accumulator_hash(),
            ledger_info.version(),
            ledger_info.timestamp_usecs(),
            None,
        );

        // Genesis carries a placeholder quorum certificate to its parent id with LedgerInfo
        // carrying information about version from the last LedgerInfo of previous epoch.
        let genesis_quorum_cert = QuorumCert::new(
            VoteData::new(ancestor.clone(), ancestor.clone()),
            LedgerInfoWithSignatures::new(
                LedgerInfo::new(ancestor, HashValue::zero()),
                AggregateSignature::empty(),
            ),
        );

        BlockData::new_genesis(ledger_info.timestamp_usecs(), genesis_quorum_cert)
    }

    #[cfg(any(test, feature = "fuzzing"))]
    // This method should only used by tests and fuzzers to produce arbitrary BlockData types.
    pub fn new_for_testing(
        epoch: u64,
        round: Round,
        timestamp_usecs: u64,
        quorum_cert: QuorumCert,
        block_type: BlockType,
    ) -> Self {
        Self {
            epoch,
            round,
            timestamp_usecs,
            quorum_cert,
            block_type,
        }
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn dummy_with_validator_txns(txns: Vec<ValidatorTransaction>) -> Self {
        Self::new_proposal_ext(
            txns,
            Payload::empty(false, true),
            Author::ONE,
            vec![],
            1,
            1,
            QuorumCert::dummy(),
        )
    }

    #[allow(unexpected_cfgs)]
    pub fn new_genesis(timestamp_usecs: u64, quorum_cert: QuorumCert) -> Self {
        assume!(quorum_cert.certified_block().epoch() < u64::MAX); // unlikely to be false in this universe
        Self {
            epoch: quorum_cert.certified_block().epoch() + 1,
            round: 0,
            timestamp_usecs,
            quorum_cert,
            block_type: BlockType::Genesis,
        }
    }

    #[allow(unexpected_cfgs)]
    pub fn new_nil(
        round: Round,
        quorum_cert: QuorumCert,
        failed_authors: Vec<(Round, Author)>,
    ) -> Self {
        // We want all the NIL blocks to agree on the timestamps even though they're generated
        // independently by different validators, hence we're using the timestamp of a parent + 1.
        assume!(quorum_cert.certified_block().timestamp_usecs() < u64::MAX); // unlikely to be false in this universe
        let timestamp_usecs = quorum_cert.certified_block().timestamp_usecs();

        Self {
            epoch: quorum_cert.certified_block().epoch(),
            round,
            timestamp_usecs,
            quorum_cert,
            block_type: BlockType::NilBlock { failed_authors },
        }
    }

    pub fn new_for_dag(
        epoch: u64,
        round: Round,
        timestamp_usecs: u64,
        validator_txns: Vec<ValidatorTransaction>,
        payload: Payload,
        author: Author,
        failed_authors: Vec<(Round, Author)>,
        parent_block_id: HashValue,
        parents_bitvec: BitVec,
        node_digests: Vec<HashValue>,
    ) -> Self {
        Self {
            epoch,
            round,
            timestamp_usecs,
            quorum_cert: QuorumCert::new(
                VoteData::new(BlockInfo::empty(), BlockInfo::empty()),
                LedgerInfoWithSignatures::new(
                    LedgerInfo::new(BlockInfo::empty(), HashValue::zero()),
                    AggregateSignature::new(BitVec::default(), None),
                ),
            ),
            block_type: BlockType::DAGBlock {
                author,
                validator_txns,
                payload,
                failed_authors,
                node_digests,
                parent_block_id,
                parents_bitvec,
            },
        }
    }

    pub fn new_proposal(
        payload: Payload,
        author: Author,
        failed_authors: Vec<(Round, Author)>,
        round: Round,
        timestamp_usecs: u64,
        quorum_cert: QuorumCert,
    ) -> Self {
        Self {
            epoch: quorum_cert.certified_block().epoch(),
            round,
            timestamp_usecs,
            quorum_cert,
            block_type: BlockType::Proposal {
                payload,
                author,
                failed_authors,
            },
        }
    }

    pub fn new_proposal_ext(
        validator_txns: Vec<ValidatorTransaction>,
        payload: Payload,
        author: Author,
        failed_authors: Vec<(Round, Author)>,
        round: Round,
        timestamp_usecs: u64,
        quorum_cert: QuorumCert,
    ) -> Self {
        Self {
            epoch: quorum_cert.certified_block().epoch(),
            round,
            timestamp_usecs,
            quorum_cert,
            block_type: BlockType::ProposalExt(ProposalExt::V0 {
                validator_txns,
                payload,
                author,
                failed_authors,
            }),
        }
    }

    /// It's a reconfiguration suffix block if the parent block's executed state indicates next epoch.
    pub fn is_reconfiguration_suffix(&self) -> bool {
        self.quorum_cert.certified_block().has_reconfiguration()
    }
}

#[test]
fn test_reconfiguration_suffix() {
    use aptos_types::{
        account_address::AccountAddress, epoch_state::EpochState, on_chain_config::ValidatorSet,
    };

    let reconfig_block_info = BlockInfo::new(
        1,
        1,
        HashValue::random(),
        HashValue::random(),
        100,
        1,
        Some(EpochState::empty()),
    );
    let quorum_cert = QuorumCert::new(
        VoteData::new(reconfig_block_info, BlockInfo::random(0)),
        LedgerInfoWithSignatures::new(
            LedgerInfo::new(
                BlockInfo::genesis(HashValue::random(), ValidatorSet::empty()),
                HashValue::zero(),
            ),
            AggregateSignature::empty(),
        ),
    );
    let reconfig_suffix_block = BlockData::new_proposal(
        Payload::empty(false, true),
        AccountAddress::random(),
        Vec::new(),
        2,
        2,
        quorum_cert,
    );
    assert!(reconfig_suffix_block.is_reconfiguration_suffix());
}

#[test]
fn test_dag_block_no_deserialize() {
    #[derive(Serialize)]
    #[serde(rename = "BlockType")]
    #[allow(dead_code)]
    pub enum FakeBlockType {
        Proposal,
        NilBlock,
        Genesis,
        DAG {
            author: Author,
            failed_authors: Vec<(Round, Author)>,
            payload: Payload,
            node_digests: Vec<HashValue>,
        },
    }
    // test deserialize failure
    let fake = FakeBlockType::DAG {
        author: Author::ZERO,
        failed_authors: vec![],
        payload: Payload::DirectMempool(vec![]),
        node_digests: vec![],
    };
    let bytes = bcs::to_bytes(&fake).unwrap();
    bcs::from_bytes::<BlockType>(&bytes).unwrap_err();
}
