// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;

use crate::{experimental::hashable::Hashable, state_replication::StateComputerCommitCallBackType};
use anyhow::anyhow;
use aptos_consensus_types::{
    common::Author, executed_block::ExecutedBlock, experimental::{commit_vote::CommitVote, rand_share::RandShare, rand_decision::RandDecision},
};
use aptos_crypto::{bls12381, HashValue};
use aptos_logger::prelude::*;
use aptos_types::{
    aggregate_signature::PartialSignatures,
    block_info::BlockInfo,
    ledger_info::{LedgerInfo, LedgerInfoWithPartialSignatures, LedgerInfoWithSignatures},
    validator_verifier::ValidatorVerifier,
};
use itertools::zip_eq;

fn generate_commit_ledger_info(
    commit_info: &BlockInfo,
    ordered_proof: &LedgerInfoWithSignatures,
) -> LedgerInfo {
    LedgerInfo::new(
        commit_info.clone(),
        ordered_proof.ledger_info().consensus_data_hash(),
    )
}

fn verify_signatures(
    unverified_signatures: PartialSignatures,
    validator: &ValidatorVerifier,
    commit_ledger_info: &LedgerInfo,
) -> PartialSignatures {
    // Returns a valid partial signature from a set of unverified signatures.
    // TODO: Validating individual signatures in expensive. Replace this with optimistic signature
    // verification for BLS. Here, we can implement a tree-based batch verification technique that
    // filters out invalid signature shares much faster when there are only a few of them
    // (e.g., [LM07]: Finding Invalid Signatures in Pairing-Based Batches,
    // by Law, Laurie and Matt, Brian J., in Cryptography and Coding, 2007).
    PartialSignatures::new(
        unverified_signatures
            .signatures()
            .iter()
            .filter(|(author, sig)| validator.verify(**author, commit_ledger_info, sig).is_ok())
            .map(|(author, sig)| (*author, sig.clone()))
            .collect(),
    )
}

fn generate_executed_item_from_ordered(
    commit_info: BlockInfo,
    executed_blocks: Vec<ExecutedBlock>,
    verified_signatures: PartialSignatures,
    callback: StateComputerCommitCallBackType,
    ordered_proof: LedgerInfoWithSignatures,
) -> BufferItem {
    debug!("{} advance to executed from ordered", commit_info);
    let partial_commit_proof = LedgerInfoWithPartialSignatures::new(
        generate_commit_ledger_info(&commit_info, &ordered_proof),
        verified_signatures,
    );
    BufferItem::Executed(Box::new(ExecutedItem {
        executed_blocks,
        partial_commit_proof,
        callback,
        commit_info,
        ordered_proof,
    }))
}

fn aggregate_commit_proof(
    commit_ledger_info: &LedgerInfo,
    verified_signatures: &PartialSignatures,
    validator: &ValidatorVerifier,
) -> LedgerInfoWithSignatures {
    let aggregated_sig = validator
        .aggregate_signatures(verified_signatures)
        .expect("Failed to generate aggregated signature");
    LedgerInfoWithSignatures::new(commit_ledger_info.clone(), aggregated_sig)
}

// we differentiate buffer items at different stages
// for better code readability
pub struct OrderedItem {
    pub unverified_signatures: PartialSignatures,
    // This can happen in the fast forward sync path, where we can receive the commit proof
    // from peers.
    pub commit_proof: Option<LedgerInfoWithSignatures>,
    pub callback: StateComputerCommitCallBackType,
    pub ordered_blocks: Vec<ExecutedBlock>,
    pub ordered_proof: LedgerInfoWithSignatures,
    pub maybe_rand_vec: Vec<Option<Vec<u8>>>, // place holder for aggregated randomness
    pub num_rand_ready: usize,  // number of blocks that have randomness ready
}

pub struct ExecutionReadyItem {
    pub unverified_signatures: PartialSignatures,
    // This can happen in the fast forward sync path, where we can receive the commit proof
    // from peers.
    pub commit_proof: Option<LedgerInfoWithSignatures>,
    pub callback: StateComputerCommitCallBackType,
    pub ordered_blocks: Vec<ExecutedBlock>,
    pub ordered_proof: LedgerInfoWithSignatures,
    pub rand_vec: Vec<Vec<u8>>, // place holder for randomness
}

pub struct ExecutedItem {
    pub executed_blocks: Vec<ExecutedBlock>,
    pub partial_commit_proof: LedgerInfoWithPartialSignatures,
    pub callback: StateComputerCommitCallBackType,
    pub commit_info: BlockInfo,
    pub ordered_proof: LedgerInfoWithSignatures,
}

pub struct SignedItem {
    pub executed_blocks: Vec<ExecutedBlock>,
    pub partial_commit_proof: LedgerInfoWithPartialSignatures,
    pub callback: StateComputerCommitCallBackType,
    pub commit_vote: CommitVote,
}

pub struct AggregatedItem {
    pub executed_blocks: Vec<ExecutedBlock>,
    pub commit_proof: LedgerInfoWithSignatures,
    pub callback: StateComputerCommitCallBackType,
}

pub enum BufferItem {
    Ordered(Box<OrderedItem>),
    ExecutionReady(Box<ExecutionReadyItem>),
    Executed(Box<ExecutedItem>),
    Signed(Box<SignedItem>),
    Aggregated(Box<AggregatedItem>),
}

impl Hashable for BufferItem {
    fn hash(&self) -> HashValue {
        self.block_id()
    }
}

impl BufferItem {
    pub fn new_ordered(
        ordered_blocks: Vec<ExecutedBlock>,
        ordered_proof: LedgerInfoWithSignatures,
        callback: StateComputerCommitCallBackType,
    ) -> Self {
        let num = ordered_blocks.len();
        Self::Ordered(Box::new(OrderedItem {
            unverified_signatures: PartialSignatures::empty(),
            commit_proof: None,
            callback,
            ordered_blocks,
            ordered_proof,
            maybe_rand_vec: vec![None; num],
            num_rand_ready: 0,
        }))
    }

    // pipeline functions
    pub fn advance_to_executed_or_aggregated(
        self,
        executed_blocks: Vec<ExecutedBlock>,
        validator: &ValidatorVerifier,
        epoch_end_timestamp: Option<u64>,
    ) -> Self {
        match self {
            Self::ExecutionReady(execution_ready_item) => {
                let ExecutionReadyItem {
                    ordered_blocks,
                    commit_proof,
                    unverified_signatures,
                    callback,
                    ordered_proof,
                    rand_vec: _,
                } = *execution_ready_item;
                for (b1, b2) in zip_eq(ordered_blocks.iter(), executed_blocks.iter()) {
                    assert_eq!(b1.id(), b2.id());
                }
                let mut commit_info = executed_blocks.last().unwrap().block_info();
                match epoch_end_timestamp {
                    Some(timestamp) if commit_info.timestamp_usecs() != timestamp => {
                        assert!(executed_blocks.last().unwrap().is_reconfiguration_suffix());
                        commit_info.change_timestamp(timestamp);
                    },
                    _ => (),
                }
                if let Some(commit_proof) = commit_proof {
                    // We have already received the commit proof in fast forward sync path,
                    // we can just use that proof and proceed to aggregated
                    assert_eq!(commit_proof.commit_info().clone(), commit_info);
                    debug!(
                        "{} advance to aggregated from ordered",
                        commit_proof.commit_info()
                    );
                    Self::Aggregated(Box::new(AggregatedItem {
                        executed_blocks,
                        commit_proof,
                        callback,
                    }))
                } else {
                    let commit_ledger_info =
                        generate_commit_ledger_info(&commit_info, &ordered_proof);

                    let verified_signatures =
                        verify_signatures(unverified_signatures, validator, &commit_ledger_info);
                    if (validator.check_voting_power(verified_signatures.signatures().keys()))
                        .is_ok()
                    {
                        let commit_proof = aggregate_commit_proof(
                            &commit_ledger_info,
                            &verified_signatures,
                            validator,
                        );
                        debug!(
                            "{} advance to aggregated from ordered",
                            commit_proof.commit_info()
                        );
                        Self::Aggregated(Box::new(AggregatedItem {
                            executed_blocks,
                            commit_proof,
                            callback,
                        }))
                    } else {
                        generate_executed_item_from_ordered(
                            commit_info,
                            executed_blocks,
                            verified_signatures,
                            callback,
                            ordered_proof,
                        )
                    }
                }
            },
            _ => {
                panic!("Only ordered blocks can advance to executed blocks.")
            },
        }
    }

    pub fn advance_to_signed(self, author: Author, signature: bls12381::Signature) -> Self {
        match self {
            Self::Executed(executed_item) => {
                let ExecutedItem {
                    executed_blocks,
                    callback,
                    partial_commit_proof,
                    ..
                } = *executed_item;

                // we don't add the signature here, it'll be added when receiving the commit vote from self
                let commit_vote = CommitVote::new_with_signature(
                    author,
                    partial_commit_proof.ledger_info().clone(),
                    signature,
                );
                debug!("{} advance to signed", partial_commit_proof.commit_info());

                Self::Signed(Box::new(SignedItem {
                    executed_blocks,
                    callback,
                    partial_commit_proof,
                    commit_vote,
                }))
            },
            _ => {
                panic!("Only executed buffer items can advance to signed blocks.")
            },
        }
    }

    /// this function assumes block id matches and the validity of ledger_info and that it has the voting power
    /// it returns an updated item
    pub fn try_advance_to_aggregated_with_ledger_info(
        self,
        commit_proof: LedgerInfoWithSignatures,
    ) -> Self {
        match self {
            Self::Signed(signed_item) => {
                let SignedItem {
                    executed_blocks,
                    callback,
                    partial_commit_proof: local_commit_proof,
                    ..
                } = *signed_item;
                assert_eq!(local_commit_proof.commit_info(), commit_proof.commit_info(),);
                debug!(
                    "{} advance to aggregated with commit decision",
                    commit_proof.commit_info()
                );
                Self::Aggregated(Box::new(AggregatedItem {
                    executed_blocks,
                    callback,
                    commit_proof,
                }))
            },
            Self::Executed(executed_item) => {
                let ExecutedItem {
                    executed_blocks,
                    callback,
                    commit_info,
                    ..
                } = *executed_item;
                assert_eq!(commit_info, *commit_proof.commit_info());
                debug!(
                    "{} advance to aggregated with commit decision",
                    commit_proof.commit_info()
                );
                Self::Aggregated(Box::new(AggregatedItem {
                    executed_blocks,
                    callback,
                    commit_proof,
                }))
            },
            Self::Ordered(ordered_item) => {
                let ordered = *ordered_item;
                assert!(ordered
                    .ordered_proof
                    .commit_info()
                    .match_ordered_only(commit_proof.commit_info()));
                // can't aggregate it without execution, only store the signatures
                debug!(
                    "{} received commit decision in ordered stage",
                    commit_proof.commit_info()
                );
                Self::Ordered(Box::new(OrderedItem {
                    commit_proof: Some(commit_proof),
                    ..ordered
                }))
            },
            Self::ExecutionReady(execution_ready_item) => {
                let ordered = *execution_ready_item;
                assert!(ordered
                    .ordered_proof
                    .commit_info()
                    .match_ordered_only(commit_proof.commit_info()));
                // can't aggregate it without execution, only store the signatures
                debug!(
                    "{} received commit decision in execution ready stage",
                    commit_proof.commit_info()
                );
                Self::ExecutionReady(Box::new(ExecutionReadyItem {
                    commit_proof: Some(commit_proof),
                    ..ordered
                }))
            },
            Self::Aggregated(_) => {
                unreachable!("Found aggregated buffer item but any aggregated buffer item should get dequeued right away.");
            },
        }
    }

    pub fn try_advance_to_aggregated(self, validator: &ValidatorVerifier) -> Self {
        match self {
            Self::Signed(signed_item) => {
                if validator
                    .check_voting_power(signed_item.partial_commit_proof.signatures().keys())
                    .is_ok()
                {
                    Self::Aggregated(Box::new(AggregatedItem {
                        executed_blocks: signed_item.executed_blocks,
                        commit_proof: aggregate_commit_proof(
                            signed_item.partial_commit_proof.ledger_info(),
                            signed_item.partial_commit_proof.partial_sigs(),
                            validator,
                        ),
                        callback: signed_item.callback,
                    }))
                } else {
                    Self::Signed(signed_item)
                }
            },
            Self::Executed(executed_item) => {
                if validator
                    .check_voting_power(executed_item.partial_commit_proof.signatures().keys())
                    .is_ok()
                {
                    Self::Aggregated(Box::new(AggregatedItem {
                        executed_blocks: executed_item.executed_blocks,
                        commit_proof: aggregate_commit_proof(
                            executed_item.partial_commit_proof.ledger_info(),
                            executed_item.partial_commit_proof.partial_sigs(),
                            validator,
                        ),
                        callback: executed_item.callback,
                    }))
                } else {
                    Self::Executed(executed_item)
                }
            },
            _ => self,
        }
    }

    // generic functions
    pub fn get_blocks(&self) -> &Vec<ExecutedBlock> {
        match self {
            Self::Ordered(ordered) => &ordered.ordered_blocks,
            Self::ExecutionReady(execution_ready) => &execution_ready.ordered_blocks,
            Self::Executed(executed) => &executed.executed_blocks,
            Self::Signed(signed) => &signed.executed_blocks,
            Self::Aggregated(aggregated) => &aggregated.executed_blocks,
        }
    }

    pub fn block_id(&self) -> HashValue {
        self.get_blocks().last().unwrap().id()
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
                    // when advancing to executed item, we will check if the sigs are valid.
                    // each author at most stores a single sig for each item,
                    // so an adversary will not be able to flood our memory.
                    ordered
                        .unverified_signatures
                        .add_signature(author, signature);
                    return Ok(());
                }
            },
            Self::ExecutionReady(ordered) => {
                if ordered
                    .ordered_proof
                    .commit_info()
                    .match_ordered_only(target_commit_info)
                {
                    // we optimistically assume the vote will be valid in the future.
                    // when advancing to executed item, we will check if the sigs are valid.
                    // each author at most stores a single sig for each item,
                    // so an adversary will not be able to flood our memory.
                    ordered
                        .unverified_signatures
                        .add_signature(author, signature);
                    return Ok(());
                }
            },
            Self::Executed(executed) => {
                if executed.commit_info == *target_commit_info {
                    executed
                        .partial_commit_proof
                        .add_signature(author, signature);
                    return Ok(());
                }
            },
            Self::Signed(signed) => {
                if signed.partial_commit_proof.commit_info() == target_commit_info {
                    signed.partial_commit_proof.add_signature(author, signature);
                    return Ok(());
                }
            },
            Self::Aggregated(aggregated) => {
                // we do not need to do anything for aggregated
                // but return true is helpful to stop the outer loop early
                if aggregated.commit_proof.commit_info() == target_commit_info {
                    return Ok(());
                }
            },
        }
        Err(anyhow!("Inconsistent commit info."))
    }

    pub fn is_ordered(&self) -> bool {
        matches!(self, Self::Ordered(_))
    }

    pub fn is_execution_ready(&self) -> bool {
        matches!(self, Self::ExecutionReady(_))
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

    pub fn unwrap_ordered_ref(&self) -> &OrderedItem {
        match self {
            BufferItem::Ordered(item) => item.as_ref(),
            _ => panic!("Not ordered item"),
        }
    }

    pub fn unwrap_execution_ready_ref(&self) -> &ExecutionReadyItem {
        match self {
            BufferItem::ExecutionReady(item) => item.as_ref(),
            _ => panic!("Not execution ready item"),
        }
    }

    pub fn unwrap_signed_ref(&self) -> &SignedItem {
        match self {
            BufferItem::Signed(item) => item.as_ref(),
            _ => panic!("Not signed item"),
        }
    }

    pub fn unwrap_executed_ref(&self) -> &ExecutedItem {
        match self {
            BufferItem::Executed(item) => item.as_ref(),
            _ => panic!("Not executed item"),
        }
    }

    pub fn unwrap_aggregated(self) -> AggregatedItem {
        match self {
            BufferItem::Aggregated(item) => *item,
            _ => panic!("Not aggregated item"),
        }
    }

    // the following functions are prototyping VRF-based randomness
    pub fn into_execution_ready(
        self,
        rand_decision: RandDecision,
    ) -> Self {
        match self {
            Self::Ordered(ordered_item) => {
                println!(
                    "{} advanced with randomness decision",
                    rand_decision.block_info()
                );
                let num = ordered_item.ordered_blocks.len();
                Self::ExecutionReady(Box::new(ExecutionReadyItem { unverified_signatures: ordered_item.unverified_signatures, commit_proof: ordered_item.commit_proof, callback: ordered_item.callback, ordered_blocks: ordered_item.ordered_blocks, ordered_proof: ordered_item.ordered_proof, rand_vec: vec![vec![u8::MAX; 96]; num] }))    // the aggregated share is 96 bytes
            }
            _ => {
                self
            }
        }
    }

    pub fn update_block_rand(&mut self, block_id: HashValue, rand: Vec<u8>) {
        if let Some(idx) = self.find_block_idx_from_ordered(block_id) {
            assert!(idx < self.get_blocks().len());
            if let Self::Ordered(ref mut ordered_item) = *self {
                if ordered_item.maybe_rand_vec[idx].is_none() {
                    (*ordered_item).maybe_rand_vec[idx] = Some(rand);
                    (*ordered_item).num_rand_ready += 1;
                }
            }
        }
    }

    // return None if not found
    pub fn find_block_idx_from_ordered(&self, block_id: HashValue) -> Option<usize> {
        match self {
            BufferItem::Ordered(item) => {
                item.ordered_blocks.iter().position(|block| (*block).id() == block_id)
            }
            _ => panic!("Not ordered item"),
        }
    }

    pub fn is_rand_ready(&self) -> bool {
        matches!(self, Self::Ordered(item) if item.num_rand_ready == self.get_blocks().len())
    }

    pub fn try_advance_to_execution_ready(self) -> Self {
        if let Self::Ordered(ordered_item) = self {
            println!("advance to execution ready");
            return Self::ExecutionReady(Box::new(ExecutionReadyItem {
                unverified_signatures: ordered_item.unverified_signatures,
                commit_proof: ordered_item.commit_proof,
                callback: ordered_item.callback,
                ordered_blocks: ordered_item.ordered_blocks,
                ordered_proof: ordered_item.ordered_proof,
                rand_vec: ordered_item.maybe_rand_vec.iter().map(|x| x.clone().unwrap()).collect(),
            }));
        }
        self
    }

    pub fn get_hash(&self) -> HashValue {
        self.hash()
    }

}
