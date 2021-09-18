// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeMap;

use anyhow::anyhow;
use itertools::zip_eq;

use consensus_types::{
    common::Author, executed_block::ExecutedBlock, experimental::commit_vote::CommitVote,
};
use diem_crypto::ed25519::Ed25519Signature;
use diem_types::{
    account_address::AccountAddress,
    block_info::BlockInfo,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    validator_verifier::ValidatorVerifier,
};

use crate::state_replication::StateComputerCommitCallBackType;
use diem_crypto::HashValue;

// we differentiate buffer items at different stages
// for better code readability
pub struct OrderedBufferItem {
    pub pending_votes: BTreeMap<AccountAddress, Ed25519Signature>,
    pub callback: StateComputerCommitCallBackType,
    pub ordered_blocks: Vec<ExecutedBlock>,
    pub ordered_proof: LedgerInfoWithSignatures,
}

pub struct ExecutedBufferItem {
    pub executed_blocks: Vec<ExecutedBlock>,
    pub pending_votes: BTreeMap<AccountAddress, Ed25519Signature>,
    pub callback: StateComputerCommitCallBackType,
    pub commit_info: BlockInfo,
    pub ordered_proof: LedgerInfoWithSignatures,
}

impl ExecutedBufferItem {
    pub fn generate_commit_ledger_info(&self) -> LedgerInfo {
        LedgerInfo::new(
            self.commit_info.clone(),
            self.ordered_proof.ledger_info().consensus_data_hash(),
        )
    }
}

pub struct SignedBufferItem {
    pub executed_blocks: Vec<ExecutedBlock>,
    pub commit_proof: LedgerInfoWithSignatures,
    pub callback: StateComputerCommitCallBackType,
    pub commit_vote: CommitVote,
}

pub struct AggregatedBufferItem {
    pub executed_blocks: Vec<ExecutedBlock>,
    pub aggregated_proof: LedgerInfoWithSignatures,
    pub callback: StateComputerCommitCallBackType,
}

pub enum BufferItem {
    Ordered(Box<OrderedBufferItem>),
    Executed(Box<ExecutedBufferItem>),
    Signed(Box<SignedBufferItem>),
    Aggregated(Box<AggregatedBufferItem>),
}

impl BufferItem {
    pub fn new_ordered(
        ordered_blocks: Vec<ExecutedBlock>,
        ordered_proof: LedgerInfoWithSignatures,
        callback: StateComputerCommitCallBackType,
    ) -> Self {
        Self::Ordered(Box::new(OrderedBufferItem {
            pending_votes: BTreeMap::<AccountAddress, Ed25519Signature>::new(),
            callback,
            ordered_blocks,
            ordered_proof,
        }))
    }

    // pipeline functions
    pub fn advance_to_executed(self, executed_blocks: Vec<ExecutedBlock>) -> Self {
        match self {
            Self::Ordered(ordered_item) => {
                let OrderedBufferItem {
                    ordered_blocks,
                    pending_votes,
                    callback,
                    ordered_proof,
                } = *ordered_item;
                for (b1, b2) in zip_eq(ordered_blocks.iter(), executed_blocks.iter()) {
                    assert_eq!(b1.id(), b2.id());
                }
                let commit_info = executed_blocks.last().unwrap().block_info();
                Self::Executed(Box::new(ExecutedBufferItem {
                    executed_blocks,
                    pending_votes,
                    callback,
                    commit_info,
                    ordered_proof,
                }))
            }
            _ => {
                panic!("Only ordered blocks can advance to executed blocks.")
            }
        }
    }

    fn aggregate_ledger_info(
        commit_ledger_info: &LedgerInfo,
        signatures: BTreeMap<AccountAddress, Ed25519Signature>,
        validator: &ValidatorVerifier,
    ) -> LedgerInfoWithSignatures {
        let valid_sigs = signatures
            .into_iter()
            .filter(|(author, sig)| validator.verify(*author, commit_ledger_info, sig).is_ok())
            .collect();

        LedgerInfoWithSignatures::new(commit_ledger_info.clone(), valid_sigs)
    }

    pub fn advance_to_signed(
        self,
        author: Author,
        signature: Ed25519Signature,
        validator: &ValidatorVerifier,
    ) -> (Self, CommitVote) {
        match self {
            Self::Executed(executed_item) => {
                let commit_ledger_info = executed_item.generate_commit_ledger_info();
                let ExecutedBufferItem {
                    executed_blocks,
                    callback,
                    mut pending_votes,
                    ..
                } = *executed_item;

                pending_votes.insert(author, signature.clone());
                let commit_proof =
                    Self::aggregate_ledger_info(&commit_ledger_info, pending_votes, validator);

                let commit_vote =
                    CommitVote::new_with_signature(author, commit_ledger_info, signature);

                (
                    Self::Signed(Box::new(SignedBufferItem {
                        executed_blocks,
                        callback,
                        commit_proof,
                        commit_vote: commit_vote.clone(),
                    })),
                    commit_vote,
                )
            }
            _ => {
                panic!("Only executed buffer items can advance to signed blocks.")
            }
        }
    }

    /// this function assumes block id matches and the validity of ledger_info and that it has the voting power
    /// it returns an updated item
    pub fn try_advance_to_aggregated_with_ledger_info(
        self,
        commit_ledger_info: LedgerInfoWithSignatures,
    ) -> Self {
        match self {
            Self::Signed(signed_item) => {
                let SignedBufferItem {
                    executed_blocks,
                    callback,
                    commit_proof,
                    ..
                } = *signed_item;
                assert_eq!(commit_proof.commit_info(), commit_ledger_info.commit_info(),);
                Self::Aggregated(Box::new(AggregatedBufferItem {
                    executed_blocks,
                    callback,
                    aggregated_proof: commit_ledger_info,
                }))
            }
            Self::Executed(executed_item) => {
                let ExecutedBufferItem {
                    executed_blocks,
                    callback,
                    commit_info,
                    ..
                } = *executed_item;
                assert_eq!(commit_info, *commit_ledger_info.commit_info());
                Self::Aggregated(Box::new(AggregatedBufferItem {
                    executed_blocks,
                    callback,
                    aggregated_proof: commit_ledger_info,
                }))
            }
            Self::Ordered(ordered_item) => {
                let ordered = *ordered_item;
                assert!(ordered
                    .ordered_proof
                    .commit_info()
                    .match_ordered_only(commit_ledger_info.commit_info()));
                Self::Ordered(Box::new(OrderedBufferItem {
                    pending_votes: commit_ledger_info.signatures().clone(),
                    ..ordered
                }))
            }
            Self::Aggregated(_) => {
                unreachable!("Found aggregated buffer item but any aggregated buffer item should get dequeued right away.");
            }
        }
    }

    pub fn try_advance_to_aggregated(self, validator: &ValidatorVerifier) -> Self {
        match self {
            Self::Signed(signed_item) => {
                if signed_item
                    .commit_proof
                    .check_voting_power(validator)
                    .is_ok()
                {
                    Self::Aggregated(Box::new(AggregatedBufferItem {
                        executed_blocks: signed_item.executed_blocks,
                        aggregated_proof: signed_item.commit_proof,
                        callback: signed_item.callback,
                    }))
                } else {
                    Self::Signed(signed_item)
                }
            }
            Self::Executed(executed_item) => {
                let maybe_aggregated_proof = Self::aggregate_ledger_info(
                    &executed_item.generate_commit_ledger_info(),
                    executed_item.pending_votes.clone(),
                    validator,
                );
                if validator
                    .check_voting_power(maybe_aggregated_proof.signatures().keys())
                    .is_ok()
                {
                    Self::Aggregated(Box::new(AggregatedBufferItem {
                        executed_blocks: executed_item.executed_blocks,
                        aggregated_proof: maybe_aggregated_proof,
                        callback: executed_item.callback,
                    }))
                } else {
                    Self::Executed(executed_item)
                }
            }
            _ => self,
        }
    }

    // generic functions
    pub fn get_blocks(&self) -> &Vec<ExecutedBlock> {
        match self {
            Self::Ordered(ordered) => &ordered.ordered_blocks,
            Self::Executed(executed) => &executed.executed_blocks,
            Self::Signed(signed) => &signed.executed_blocks,
            Self::Aggregated(aggregated) => &aggregated.executed_blocks,
        }
    }

    pub fn block_id(&self) -> HashValue {
        self.get_blocks().last().unwrap().id()
    }

    pub fn get_commit_info(&self) -> &BlockInfo {
        match self {
            Self::Ordered(_) => {
                panic!("Ordered buffer item does not contain commit info");
            }
            Self::Executed(executed) => &executed.commit_info,
            Self::Signed(signed) => signed.commit_proof.ledger_info().commit_info(),
            Self::Aggregated(aggregated) => aggregated.aggregated_proof.ledger_info().commit_info(),
        }
    }

    pub fn add_signature_if_matched(&mut self, vote: CommitVote) -> anyhow::Result<()> {
        let target_commit_info = vote.commit_info();
        let author = vote.author();
        let signature = vote.signature().clone();
        match self {
            Self::Ordered(ordered) => {
                if ordered
                    .ordered_proof
                    .commit_info()
                    .match_ordered_only(target_commit_info)
                {
                    // we optimistically assume the vote will be valid in the future.
                    // when advancing to signed item, we will check if the sigs are valid.
                    // each author at most stores a single sig for each item,
                    // so an adversary will not be able to flood our memory.
                    ordered.pending_votes.insert(author, signature);
                    return Ok(());
                }
            }
            Self::Executed(executed) => {
                if &executed.commit_info == target_commit_info {
                    executed.pending_votes.insert(author, signature);
                    return Ok(());
                }
            }
            Self::Signed(signed) => {
                if signed.commit_proof.commit_info() == target_commit_info {
                    signed.commit_proof.add_signature(author, signature);
                    return Ok(());
                }
            }
            Self::Aggregated(aggregated) => {
                // we do not need to do anything for aggregated
                // but return true is helpful to stop the outer loop early
                if aggregated.aggregated_proof.commit_info() == target_commit_info {
                    return Ok(());
                }
            }
        }
        Err(anyhow!("Inconsistent commit info."))
    }

    pub fn has_been_executed(&self) -> bool {
        !self.is_ordered()
    }

    pub fn is_ordered(&self) -> bool {
        matches!(self, Self::Ordered(_))
    }

    pub fn is_executed(&self) -> bool {
        matches!(self, Self::Executed(_))
    }

    pub fn is_signed(&self) -> bool {
        matches!(self, Self::Signed(_))
    }

    pub fn is_aggregated(&self) -> bool {
        matches!(self, Self::Aggregated(_))
    }
}
