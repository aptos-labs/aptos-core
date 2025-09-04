// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::arc_with_non_send_sync, clippy::unwrap_used)]

use crate::serializer::SafetyRulesInput;
#[cfg(any(test, feature = "fuzzing"))]
use velor_consensus_types::block::Block;
use velor_consensus_types::{
    block_data::{BlockData, BlockType},
    common::Payload,
    order_vote_proposal::OrderVoteProposal,
    quorum_cert::QuorumCert,
    timeout_2chain::TwoChainTimeout,
    vote_data::VoteData,
    vote_proposal::VoteProposal,
};
use velor_crypto::{
    bls12381,
    hash::{HashValue, TransactionAccumulatorHasher},
    test_utils::TEST_SEED,
    traits::{SigningKey, Uniform},
};
use velor_types::{
    account_address::AccountAddress,
    epoch_change::EpochChangeProof,
    epoch_state::EpochState,
    ledger_info::LedgerInfoWithSignatures,
    proof::AccumulatorExtensionProof,
    proptest_types::{AccountInfoUniverse, BlockInfoGen},
    transaction::SignedTransaction,
    validator_verifier::{ValidatorConsensusInfo, ValidatorVerifier},
};
use proptest::prelude::*;
use rand::{rngs::StdRng, SeedableRng};
use std::sync::Arc;

const MAX_BLOCK_SIZE: usize = 10000;
const MAX_NUM_ADDR_TO_VALIDATOR_INFO: usize = 10;
const MAX_NUM_LEAVES: usize = 20;
const MAX_NUM_LEDGER_INFO_WITH_SIGS: usize = 10;
const MAX_NUM_SUBTREE_ROOTS: usize = 20;
const MAX_PROPOSAL_TRANSACTIONS: usize = 5;
const NUM_UNIVERSE_ACCOUNTS: usize = 3;

// This generates an arbitrary AccumulatorExtensionProof<TransactionAccumulatorHasher>.
prop_compose! {
    pub fn arb_accumulator_extension_proof(
    )(
        frozen_subtree_roots in prop::collection::vec(any::<HashValue>(), 0..MAX_NUM_SUBTREE_ROOTS),
        leaf_count in any::<u64>(),
        leaves in prop::collection::vec(any::<HashValue>(), 0..MAX_NUM_LEAVES),
    ) -> AccumulatorExtensionProof<TransactionAccumulatorHasher> {
        AccumulatorExtensionProof::<TransactionAccumulatorHasher>::new(
            frozen_subtree_roots,
            leaf_count,
            leaves
        )
    }
}

// This generates an arbitrary Block.
prop_compose! {
    pub fn arb_block(
    )(
        id in any::<HashValue>(),
        block_data in arb_block_data(),
        include_signature in any::<bool>(),
    ) -> Block {
        let signature = if include_signature {
            let mut rng = StdRng::from_seed(TEST_SEED);
            let private_key = bls12381::PrivateKey::generate(&mut rng);
            let signature = private_key.sign(&block_data).unwrap();
            Some(signature)
        } else {
            None
        };
        Block::new_for_testing(id, block_data, signature)
    }
}

// This generates an arbitrary BlockData.
prop_compose! {
    pub fn arb_block_data(
    )(
        epoch in any::<u64>(),
        round in any::<u64>(),
        timestamp_usecs in any::<u64>(),
        quorum_cert in arb_quorum_cert(),
        block_type in arb_block_type(),
    ) -> BlockData {
        BlockData::new_for_testing(epoch, round, timestamp_usecs, quorum_cert, block_type)
    }
}

// This generates an arbitrary BlockType::Proposal enum instance.
prop_compose! {
    pub fn arb_block_type_proposal(
    )(
        author in any::<AccountAddress>(),
        txns in prop::collection::vec(any::<SignedTransaction>(), 0..MAX_PROPOSAL_TRANSACTIONS),
    ) -> BlockType {
        BlockType::Proposal{
            payload: Payload::DirectMempool(txns),
            author,
            failed_authors: Vec::new(),
        }
    }
}

// This generates an arbitrary BlockType::Proposal enum instance.
prop_compose! {
    pub fn arb_nil_block(
    )(
        author in any::<AccountAddress>(),
        round in any::<u64>(),
    ) -> BlockType {
        BlockType::NilBlock{
            failed_authors: vec![(round, author)],
        }
    }
}

// This generates an arbitrary VoteProposal.
prop_compose! {
    pub fn arb_vote_proposal(
    )(
        accumulator_extension_proof in arb_accumulator_extension_proof(),
        block in arb_block(),
        next_epoch_state in arb_epoch_state(),
    ) -> VoteProposal {
        VoteProposal::new(accumulator_extension_proof, block, next_epoch_state, false)
    }
}

// This generates an arbitrary OrderVoteProposal.
prop_compose! {
    pub fn arb_order_vote_proposal(
    )(
        block in arb_block(),
        next_epoch_state in arb_epoch_state(),
        parent_block_info_gen in any::<BlockInfoGen>(),
        mut parent_account_info_universe in any_with::<AccountInfoUniverse>(NUM_UNIVERSE_ACCOUNTS),
        parent_block_size in 1..MAX_BLOCK_SIZE,
        signed_ledger_info in any::<LedgerInfoWithSignatures>(),
    ) -> OrderVoteProposal {
        let proposed_block_info = block.gen_block_info(
            HashValue::zero(),
            0,
            next_epoch_state,
        );
        let parent_block_info = parent_block_info_gen.materialize(
            &mut parent_account_info_universe,
            parent_block_size
        );
        let vote_data = VoteData::new(proposed_block_info.clone(), parent_block_info);
        let quorum_cert = QuorumCert::new(vote_data, signed_ledger_info);
        OrderVoteProposal::new(block, proposed_block_info, Arc::new(quorum_cert))
    }
}

// This generates an arbitrary EpochChangeProof.
prop_compose! {
    pub fn arb_epoch_change_proof(
    )(
        more in any::<bool>(),
        ledger_info_with_sigs in prop::collection::vec(
            any::<LedgerInfoWithSignatures>(),
            0..MAX_NUM_LEDGER_INFO_WITH_SIGS
        ),
    ) -> EpochChangeProof {
        EpochChangeProof::new(
            ledger_info_with_sigs,
            more,
        )
    }
}

// This generates an arbitrary Timeout.
prop_compose! {
    pub fn arb_timeout(
    )(
        epoch in any::<u64>(),
        round in any::<u64>(),
        qc in arb_quorum_cert(),
    ) -> TwoChainTimeout {
        TwoChainTimeout::new(epoch, round, qc)
    }
}

// This generates an arbitrary and optional EpochState.
prop_compose! {
    pub fn arb_epoch_state(
    )(
        include_epoch_state in any::<bool>(),
        epoch in any::<u64>(),
        validator_infos in prop::collection::vec(
            any::<ValidatorConsensusInfo>(),
            0..MAX_NUM_ADDR_TO_VALIDATOR_INFO
        ),
    ) -> Option<EpochState> {
        let verifier = ValidatorVerifier::new(
            validator_infos,
        );
        if include_epoch_state {
            Some(EpochState::new(
                epoch,
                verifier
            ))
        } else {
            None
        }
    }
}

// This generates an arbitrary QuorumCert.
prop_compose! {
    pub fn arb_quorum_cert(
    )(
        proposed_block_info_gen in any::<BlockInfoGen>(),
        parent_block_info_gen in any::<BlockInfoGen>(),
        mut proposed_account_info_universe in
            any_with::<AccountInfoUniverse>(NUM_UNIVERSE_ACCOUNTS),
        mut parent_account_info_universe in any_with::<AccountInfoUniverse>(NUM_UNIVERSE_ACCOUNTS),
        proposed_block_size in 1..MAX_BLOCK_SIZE,
        parent_block_size in 1..MAX_BLOCK_SIZE,
        signed_ledger_info in any::<LedgerInfoWithSignatures>(),
    ) -> QuorumCert {
        let proposed_block_info = proposed_block_info_gen.materialize(
            &mut proposed_account_info_universe,
            proposed_block_size
        );
        let parent_block_info = parent_block_info_gen.materialize(
            &mut parent_account_info_universe,
            parent_block_size
        );
        let vote_data = VoteData::new(proposed_block_info, parent_block_info);
        QuorumCert::new(vote_data, signed_ledger_info)
    }
}

// This generates an arbitrary BlockType enum.
fn arb_block_type() -> impl Strategy<Value = BlockType> {
    prop_oneof![
        arb_block_type_proposal(),
        arb_nil_block(),
        Just(BlockType::Genesis),
    ]
}

// This generates an arbitrary SafetyRulesInput enum.
pub fn arb_safety_rules_input() -> impl Strategy<Value = SafetyRulesInput> {
    prop_oneof![
        Just(SafetyRulesInput::ConsensusState),
        arb_epoch_change_proof().prop_map(|input| SafetyRulesInput::Initialize(Box::new(input))),
        arb_vote_proposal().prop_map(|input| {
            SafetyRulesInput::ConstructAndSignVoteTwoChain(Box::new(input), Box::new(None))
        }),
        arb_order_vote_proposal()
            .prop_map(|input| { SafetyRulesInput::ConstructAndSignOrderVote(Box::new(input)) }),
        arb_block_data().prop_map(|input| { SafetyRulesInput::SignProposal(Box::new(input)) }),
        arb_timeout().prop_map(|input| {
            SafetyRulesInput::SignTimeoutWithQC(Box::new(input), Box::new(None))
        }),
    ]
}

#[cfg(any(test, feature = "fuzzing"))]
pub mod fuzzing {
    use crate::{error::Error, serializer::SafetyRulesInput, test_utils, TSafetyRules};
    use velor_consensus_types::{
        block_data::BlockData, order_vote::OrderVote, order_vote_proposal::OrderVoteProposal,
        timeout_2chain::TwoChainTimeout, vote::Vote, vote_proposal::VoteProposal,
    };
    use velor_crypto::bls12381;
    use velor_types::epoch_change::EpochChangeProof;

    pub fn fuzz_initialize(proof: EpochChangeProof) -> Result<(), Error> {
        let mut safety_rules = test_utils::test_safety_rules_uninitialized();
        safety_rules.initialize(&proof)
    }

    pub fn fuzz_construct_and_sign_vote_two_chain(
        vote_proposal: VoteProposal,
    ) -> Result<Vote, Error> {
        let mut safety_rules = test_utils::test_safety_rules();
        safety_rules.construct_and_sign_vote_two_chain(&vote_proposal, None)
    }

    pub fn fuzz_construct_and_sign_order_vote(
        order_vote_proposal: OrderVoteProposal,
    ) -> Result<OrderVote, Error> {
        let mut safety_rules = test_utils::test_safety_rules();
        safety_rules.construct_and_sign_order_vote(&order_vote_proposal)
    }

    pub fn fuzz_handle_message(safety_rules_input: SafetyRulesInput) -> Result<Vec<u8>, Error> {
        // Create a safety rules serializer test instance for fuzzing
        let mut serializer_service = test_utils::test_serializer();

        // encode the safety_rules_input and fuzz the handle_message() method
        if let Ok(safety_rules_input) = serde_json::to_vec(&safety_rules_input) {
            serializer_service.handle_message(safety_rules_input)
        } else {
            Err(Error::SerializationError(
                "Unable to serialize safety rules input for fuzzer!".into(),
            ))
        }
    }

    pub fn fuzz_sign_proposal(block_data: &BlockData) -> Result<bls12381::Signature, Error> {
        let mut safety_rules = test_utils::test_safety_rules();
        safety_rules.sign_proposal(block_data)
    }

    pub fn fuzz_sign_timeout_with_qc(
        timeout: TwoChainTimeout,
    ) -> Result<bls12381::Signature, Error> {
        let mut safety_rules = test_utils::test_safety_rules();
        safety_rules.sign_timeout_with_qc(&timeout, None)
    }
}

// Note: these tests ensure that the various fuzzers are maintained (i.e., not broken
// at some time in the future and only discovered when a fuzz test fails).
#[cfg(test)]
mod tests {
    use crate::{
        fuzzing::{
            fuzz_construct_and_sign_order_vote, fuzz_construct_and_sign_vote_two_chain,
            fuzz_handle_message, fuzz_initialize, fuzz_sign_proposal, fuzz_sign_timeout_with_qc,
        },
        fuzzing_utils::{
            arb_block_data, arb_epoch_change_proof, arb_order_vote_proposal,
            arb_safety_rules_input, arb_timeout, arb_vote_proposal,
        },
    };
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(10))]

        #[test]
        fn handle_message_proptest(input in arb_safety_rules_input()) {
            let _ = fuzz_handle_message(input);
        }

        #[test]
        fn initialize_proptest(input in arb_epoch_change_proof()) {
            let _ = fuzz_initialize(input);
        }

        #[test]
        fn construct_and_sign_vote_two_chain_proptest(input in arb_vote_proposal()) {
            let _ = fuzz_construct_and_sign_vote_two_chain(input);
        }

        #[test]
        fn contruct_and_sign_order_vote(input in arb_order_vote_proposal()) {
            let _ = fuzz_construct_and_sign_order_vote(input);
        }

        #[test]
        fn sign_proposal_proptest(input in arb_block_data()) {
            let _ = fuzz_sign_proposal(&input);
        }

        #[test]
        fn sign_timeout_proptest(input in arb_timeout()) {
            let _ = fuzz_sign_timeout_with_qc(input);
        }
    }
}
