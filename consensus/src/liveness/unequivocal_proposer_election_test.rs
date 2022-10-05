// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use aptos_types::validator_signer::ValidatorSigner;
use consensus_types::{
    block::{block_test_utils::certificate_for_genesis, Block},
    common::{Author, Payload, Round},
};

use crate::liveness::unequivocal_proposer_election::UnequivocalProposerElection;

use super::proposer_election::ProposerElection;

struct MockProposerElection {
    proposers: HashMap<Round, Author>,
}

impl MockProposerElection {
    pub fn new(proposers: HashMap<Round, Author>) -> Self {
        Self { proposers }
    }
}

impl ProposerElection for MockProposerElection {
    fn get_valid_proposer(&self, round: Round) -> Author {
        *self.proposers.get(&round).unwrap()
    }
}

#[test]
fn test_is_valid_proposal() {
    let chosen_validator_signer = ValidatorSigner::random([0u8; 32]);
    let chosen_author = chosen_validator_signer.author();
    let another_validator_signer = ValidatorSigner::random([1u8; 32]);
    // let another_author = another_validator_signer.author();

    // Test genesis and the next block
    let quorum_cert = certificate_for_genesis();

    let good_proposal = Block::new_proposal(
        Payload::empty(),
        1,
        1,
        quorum_cert.clone(),
        &chosen_validator_signer,
        Vec::new(),
    )
    .unwrap();
    let bad_author_proposal = Block::new_proposal(
        Payload::empty(),
        1,
        1,
        quorum_cert.clone(),
        &another_validator_signer,
        Vec::new(),
    )
    .unwrap();
    let bad_duplicate_proposal = Block::new_proposal(
        Payload::empty(),
        1,
        2,
        quorum_cert.clone(),
        &chosen_validator_signer,
        Vec::new(),
    )
    .unwrap();
    let next_good_proposal = Block::new_proposal(
        Payload::empty(),
        2,
        3,
        quorum_cert.clone(),
        &chosen_validator_signer,
        Vec::new(),
    )
    .unwrap();
    let next_bad_duplicate_proposal = Block::new_proposal(
        Payload::empty(),
        2,
        4,
        quorum_cert,
        &chosen_validator_signer,
        Vec::new(),
    )
    .unwrap();

    let pe =
        UnequivocalProposerElection::new(Box::new(MockProposerElection::new(HashMap::from([
            (1, chosen_author),
            (2, chosen_author),
        ]))));

    assert!(pe.is_valid_proposer(chosen_author, 1));
    assert!(pe.is_valid_proposal(&good_proposal));
    assert!(!pe.is_valid_proposal(&bad_author_proposal));

    // another proposal from the valid proposer should fail
    assert!(!pe.is_valid_proposal(&bad_duplicate_proposal));
    // good proposal still passes
    assert!(pe.is_valid_proposal(&good_proposal));

    // going to the next round:
    assert!(pe.is_valid_proposal(&next_good_proposal));
    assert!(!pe.is_valid_proposal(&next_bad_duplicate_proposal));

    // Proposal from previous round is not valid any more:
    assert!(!pe.is_valid_proposal(&good_proposal));
}
