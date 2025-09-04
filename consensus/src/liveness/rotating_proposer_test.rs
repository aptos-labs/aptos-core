// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::liveness::{
    proposer_election::ProposerElection, rotating_proposer_election::RotatingProposer,
};
use velor_types::account_address::AccountAddress;

#[test]
fn test_rotating_proposer() {
    let chosen_author = AccountAddress::random();
    let another_author = AccountAddress::random();
    let proposers = vec![chosen_author, another_author];
    let pe = RotatingProposer::new(proposers, 1);

    // Send a proposal from both chosen author and another author, the only winning proposals
    // follow the round-robin rotation.

    assert!(!pe.is_valid_proposer(chosen_author, 1));
    assert!(pe.is_valid_proposer(another_author, 1),);
    assert!(pe.is_valid_proposer(chosen_author, 2));
    assert!(!pe.is_valid_proposer(another_author, 2));
    assert_eq!(pe.get_valid_proposer(1), another_author);
    assert_eq!(pe.get_valid_proposer(2), chosen_author);
}

#[test]
fn test_rotating_proposer_with_three_contiguous_rounds() {
    let chosen_author = AccountAddress::random();
    let another_author = AccountAddress::random();
    let proposers = vec![chosen_author, another_author];
    let pe = RotatingProposer::new(proposers, 3);

    // Send a proposal from both chosen author and another author, the only winning proposals
    // follow the round-robin rotation with 3 contiguous rounds.

    assert!(!pe.is_valid_proposer(another_author, 1));
    assert!(pe.is_valid_proposer(chosen_author, 1));
    assert!(pe.is_valid_proposer(chosen_author, 2));
    assert!(!pe.is_valid_proposer(another_author, 2));
    assert_eq!(pe.get_valid_proposer(1), chosen_author);
    assert_eq!(pe.get_valid_proposer(2), chosen_author);
}

#[test]
fn test_fixed_proposer() {
    let chosen_author = AccountAddress::random();
    let another_author = AccountAddress::random();
    let pe = RotatingProposer::new(vec![chosen_author], 1);

    // Send a proposal from both chosen author and another author, the only winning proposal is
    // from the chosen author.

    assert!(pe.is_valid_proposer(chosen_author, 1));
    assert!(!pe.is_valid_proposer(another_author, 1));
    assert_eq!(pe.get_valid_proposer(1), chosen_author);
    assert!(pe.is_valid_proposer(chosen_author, 2));
}
