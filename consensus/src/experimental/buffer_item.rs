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
            Self::Ordered(ordered_item_box) => {
                let ordered_item = *ordered_item_box;
                for (b1, b2) in zip_eq(ordered_item.ordered_blocks.iter(), executed_blocks.iter()) {
                    assert_eq!(b1.id(), b2.id());
                }
                let commit_info = executed_blocks.last().unwrap().block_info();
                Self::Executed(Box::new(ExecutedBufferItem {
                    executed_blocks,
                    pending_votes: ordered_item.pending_votes,
                    callback: ordered_item.callback,
                    commit_info,
                    ordered_proof: ordered_item.ordered_proof,
                }))
            }
            _ => {
                panic!("Only ordered blocks can advance to executed blocks.")
            }
        }
    }

    pub fn advance_to_signed(
        self,
        author: Author,
        signature: Ed25519Signature,
        verifier: &ValidatorVerifier,
    ) -> (Self, CommitVote) {
        match self {
            Self::Executed(executed_item_box) => {
                let executed_item = *executed_item_box;

                let mut valid_sigs = BTreeMap::<AccountAddress, Ed25519Signature>::new();
                valid_sigs.insert(author, signature.clone());

                let commit_ledger_info = LedgerInfo::new(
                    executed_item.commit_info,
                    executed_item
                        .ordered_proof
                        .ledger_info()
                        .consensus_data_hash(),
                );

                for (author, sig) in executed_item.pending_votes.iter() {
                    if verifier.verify(*author, &commit_ledger_info, sig).is_ok() {
                        valid_sigs.insert(*author, sig.clone());
                    }
                }

                let commit_ledger_info_with_sigs =
                    LedgerInfoWithSignatures::new(commit_ledger_info.clone(), valid_sigs);

                let commit_vote =
                    CommitVote::new_with_signature(author, commit_ledger_info, signature);

                (
                    Self::Signed(Box::new(SignedBufferItem {
                        executed_blocks: executed_item.executed_blocks,
                        callback: executed_item.callback,
                        commit_proof: commit_ledger_info_with_sigs,
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

    pub fn try_advance_to_aggregated(self, validator: &ValidatorVerifier) -> (Self, bool) {
        /*
        TODO:
            executed buffer item might also advance to aggregated if it received
            enough signatures from peers
         */
        match self {
            Self::Signed(signed_item_box) => {
                let signed_item = *signed_item_box;
                if signed_item
                    .commit_proof
                    .check_voting_power(validator)
                    .is_ok()
                {
                    (
                        Self::Aggregated(Box::new(AggregatedBufferItem {
                            executed_blocks: signed_item.executed_blocks,
                            aggregated_proof: signed_item.commit_proof,
                            callback: signed_item.callback,
                        })),
                        true,
                    )
                } else {
                    (Self::Signed(Box::new(signed_item)), false)
                }
            }
            _ => {
                // we do not panic here since this is a try function
                (self, false)
            }
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

    pub fn add_signature_if_matched(
        &mut self,
        target_commit_info: &BlockInfo,
        author: Author,
        sig: Ed25519Signature,
    ) -> anyhow::Result<()> {
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
                    ordered.pending_votes.insert(author, sig);
                    return Ok(());
                }
            }
            Self::Executed(executed) => {
                if &executed.commit_info == target_commit_info {
                    executed.pending_votes.insert(author, sig);
                    return Ok(());
                }
            }
            Self::Signed(signed) => {
                if signed.commit_proof.commit_info() == target_commit_info {
                    signed.commit_proof.add_signature(author, sig);
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
        !matches!(self, Self::Ordered(_))
    }
}
