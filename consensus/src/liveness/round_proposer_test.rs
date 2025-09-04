// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::liveness::{
    proposer_election::ProposerElection, round_proposer_election::RoundProposer,
};
use velor_consensus_types::common::{Author, Round};
use velor_types::account_address::AccountAddress;
use std::collections::HashMap;

#[test]
fn test_round_proposer() {
    let chosen_author_round1 = AccountAddress::random();
    let chosen_author_round2 = AccountAddress::random();
    let another_author = AccountAddress::random();

    // A map that specifies the proposer per round
    let mut round_proposers: HashMap<Round, Author> = HashMap::new();
    round_proposers.insert(1, chosen_author_round1);
    round_proposers.insert(2, chosen_author_round2);

    let pe = RoundProposer::new(round_proposers, chosen_author_round1);

    // Send a proposal from both chosen author and another author, the only winning proposals
    // follow the round-proposers mapping

    // In round 3, send a proposal from chosen_author_round1 (which is also the default proposer).
    // The proposal should win because the map doesn't specify proposer for round 3 hence
    // falling back on the default proposer

    assert!(pe.is_valid_proposer(chosen_author_round1, 1),);
    assert!(!pe.is_valid_proposer(another_author, 1));
    assert!(pe.is_valid_proposer(chosen_author_round2, 2));
    assert!(!pe.is_valid_proposer(another_author, 2));
    assert!(pe.is_valid_proposer(chosen_author_round1, 3));
    assert!(!pe.is_valid_proposer(another_author, 3));
    assert_eq!(pe.get_valid_proposer(1), chosen_author_round1);
    assert_eq!(pe.get_valid_proposer(2), chosen_author_round2);
    assert_eq!(pe.get_valid_proposer(3), chosen_author_round1);
}
