// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use crate::liveness::{
    leader_reputation::{
        ActiveInactiveHeuristic, LeaderReputation, MetadataBackend, NewBlockEventAggregation,
        ReputationHeuristic,
    },
    proposer_election::{next, ProposerElection},
};

use aptos_types::{
    account_address::AccountAddress, block_metadata::NewBlockEvent,
    validator_signer::ValidatorSigner,
};
use consensus_types::common::{Author, Round};
use itertools::Itertools;

use super::leader_reputation::ProposerAndVoterHeuristic;

struct MockHistory {
    window_size: usize,
    data: Vec<NewBlockEvent>,
}

impl MockHistory {
    fn new(window_size: usize, data: Vec<NewBlockEvent>) -> Self {
        Self { window_size, data }
    }
}

impl MetadataBackend for MockHistory {
    fn get_block_metadata(&self, _target_round: Round) -> Vec<NewBlockEvent> {
        let start = if self.data.len() > self.window_size {
            self.data.len() - self.window_size
        } else {
            0
        };
        self.data[start..].to_vec()
    }
}

struct TestBlockBuilder {
    epoch: u64,
    round: Round,
}

impl TestBlockBuilder {
    fn new() -> Self {
        Self { epoch: 0, round: 0 }
    }

    fn new_epoch(&mut self) -> &mut Self {
        self.epoch += 1;
        self
    }

    fn create_block(
        &mut self,
        proposer: Author,
        voters: Vec<bool>,
        failed_proposers: Vec<u64>,
    ) -> NewBlockEvent {
        self.round += 1 + failed_proposers.len() as u64;
        NewBlockEvent::new(
            self.epoch,
            self.round,
            voters,
            proposer,
            failed_proposers,
            self.round * 3600,
        )
    }
}

/// #### NewBlockEventAggregation tests ####

#[test]
fn test_aggregation_bitmap_to_voters() {
    let validators: Vec<_> = (0..4).into_iter().map(|_| Author::random()).collect();
    let bitmap = vec![true, true, false, true];

    if let Ok(voters) = NewBlockEventAggregation::bitmap_to_voters(&validators, &bitmap) {
        assert_eq!(&validators[0], voters[0]);
        assert_eq!(&validators[1], voters[1]);
        assert_eq!(&validators[3], voters[2]);
    } else {
        unreachable!();
    }
}

#[test]
fn test_aggregation_bitmap_to_voters_mismatched_lengths() {
    let validators: Vec<_> = (0..4) // size of 4
        .into_iter()
        .map(|_| Author::random())
        .collect();
    let bitmap_too_long = vec![true, true, false, true, true]; // size of 5
    assert!(NewBlockEventAggregation::bitmap_to_voters(&validators, &bitmap_too_long).is_err());
    let bitmap_too_short = vec![true, true, false];
    assert!(NewBlockEventAggregation::bitmap_to_voters(&validators, &bitmap_too_short).is_err());
}

#[test]
fn test_aggregation_indices_to_authors() {
    let validators: Vec<_> = (0..4).into_iter().map(|_| Author::random()).collect();
    let indices = vec![2u64, 2, 0, 3];

    if let Ok(authors) = NewBlockEventAggregation::indices_to_validators(&validators, &indices) {
        assert_eq!(&validators[2], authors[0]);
        assert_eq!(&validators[2], authors[1]);
        assert_eq!(&validators[0], authors[2]);
        assert_eq!(&validators[3], authors[3]);
    } else {
        unreachable!();
    }
}

#[test]
fn test_aggregation_indices_to_authors_out_of_index() {
    let validators: Vec<_> = (0..4).into_iter().map(|_| Author::random()).collect();
    let indices = vec![0, 0, 4, 0];
    assert!(NewBlockEventAggregation::indices_to_validators(&validators, &indices).is_err());
}

struct Example1 {
    validators: Vec<Author>,
    block_builder: TestBlockBuilder,
    history: Vec<NewBlockEvent>,
}

impl Example1 {
    fn new() -> Self {
        Self {
            validators: (0..4).into_iter().map(|_| Author::random()).collect(),
            block_builder: TestBlockBuilder::new(),
            history: vec![],
        }
    }

    fn step1(&mut self) {
        self.history.push(self.block_builder.create_block(
            self.validators[0],
            vec![false, true, true, false],
            vec![3],
        ));
        self.history.push(self.block_builder.create_block(
            self.validators[0],
            vec![false, true, true, false],
            vec![],
        ));
        self.history.push(self.block_builder.create_block(
            self.validators[1],
            vec![true, false, true, false],
            vec![2],
        ));
        self.history.push(self.block_builder.create_block(
            self.validators[2],
            vec![true, true, false, false],
            vec![],
        ));
    }

    fn step2(&mut self) {
        self.history.push(self.block_builder.create_block(
            self.validators[3],
            vec![true, true, false, false],
            vec![1],
        ));
        self.history.push(self.block_builder.create_block(
            self.validators[3],
            vec![true, true, false, false],
            vec![1],
        ));
    }

    fn step3(&mut self) {
        self.block_builder.new_epoch();
        self.history.push(self.block_builder.create_block(
            self.validators[3],
            vec![true, true, false, false],
            vec![0],
        ));
    }
}

fn example1_init() {}

#[test]
fn test_aggregation_counting() {
    let mut example1 = Example1::new();
    let validators = example1.validators.clone();
    let aggregation = NewBlockEventAggregation::new(2, 5);

    example1.step1();

    assert_eq!(
        aggregation.count_proposals(0, &example1.history),
        HashMap::from([(validators[0], 2), (validators[1], 1), (validators[2], 1),])
    );
    assert_eq!(
        aggregation.count_failed_proposals(0, &validators, &example1.history),
        HashMap::from([(validators[2], 1), (validators[3], 1),])
    );
    assert_eq!(
        aggregation.count_votes(0, &validators, &example1.history),
        HashMap::from([(validators[0], 2), (validators[1], 1), (validators[2], 1),])
    );

    example1.step2();

    assert_eq!(
        aggregation.count_proposals(0, &example1.history),
        HashMap::from([
            (validators[0], 1),
            (validators[1], 1),
            (validators[2], 1),
            (validators[3], 2),
        ])
    );
    assert_eq!(
        aggregation.count_failed_proposals(0, &validators, &example1.history),
        HashMap::from([(validators[2], 1), (validators[1], 2),])
    );
    assert_eq!(
        aggregation.count_votes(0, &validators, &example1.history),
        HashMap::from([(validators[0], 2), (validators[1], 2),])
    );

    example1.step3();

    assert_eq!(
        aggregation.count_proposals(1, &example1.history),
        HashMap::from([(validators[3], 1),])
    );
    assert_eq!(
        aggregation.count_failed_proposals(1, &validators, &example1.history),
        HashMap::from([(validators[0], 1),])
    );
    assert_eq!(
        aggregation.count_votes(1, &validators, &example1.history),
        HashMap::from([(validators[0], 1), (validators[1], 1),])
    );
}

/// ####

#[test]
fn test_proposer_and_voter_heuristic() {
    let mut example1 = Example1::new();
    let validators = example1.validators.clone();
    let heuristic = ProposerAndVoterHeuristic::new(validators[0], 100, 10, 1, 49, 2, 5);

    example1.step1();
    assert_eq!(
        heuristic.get_weights(0, &validators, &example1.history),
        vec![100, 100, 1, 1]
    );

    example1.step2();
    assert_eq!(
        heuristic.get_weights(0, &validators, &example1.history),
        vec![100, 1, 1, 100]
    );

    example1.step3();
    assert_eq!(
        heuristic.get_weights(1, &validators, &example1.history),
        vec![1, 100, 10, 100]
    );
}

/// #### ActiveInactiveHeuristic tests ####

#[test]
fn test_simple_heuristic() {
    let active_weight = 9;
    let inactive_weight = 1;
    let mut proposers = vec![];
    let mut signers = vec![];
    for i in 0..8 {
        let signer = ValidatorSigner::random([i; 32]);
        proposers.push(signer.author());
        signers.push(signer);
    }
    let mut block_builder = TestBlockBuilder::new();
    let heuristic = ActiveInactiveHeuristic::new(
        proposers[0],
        active_weight,
        inactive_weight,
        proposers.len(),
    );
    // 1. Window size not enough
    let weights = heuristic.get_weights(0, &proposers, &[]);
    assert_eq!(weights.len(), proposers.len());
    for w in weights {
        assert_eq!(w, inactive_weight);
    }
    // 2. Sliding window with [proposer 0, voters 1, 2], [proposer 0, voters 3]
    let weights = heuristic.get_weights(
        0,
        &proposers,
        &[
            block_builder.create_block(
                proposers[0],
                vec![false, true, true, false, false, false, false, false],
                vec![],
            ),
            block_builder.create_block(
                proposers[0],
                vec![false, false, false, true, false, false, false, false],
                vec![],
            ),
        ],
    );
    assert_eq!(weights.len(), proposers.len());
    for (i, w) in weights.iter().enumerate() {
        let expected = if i < 4 {
            active_weight
        } else {
            inactive_weight
        };
        assert_eq!(*w, expected);
    }
}

#[test]
fn test_with_failed_heuristic() {
    let active_weight = 9;
    let inactive_weight = 1;
    let mut proposers = vec![];
    let mut signers = vec![];
    for i in 0..8 {
        let signer = ValidatorSigner::random([i; 32]);
        proposers.push(signer.author());
        signers.push(signer);
    }
    let mut block_builder = TestBlockBuilder::new();
    let heuristic = ActiveInactiveHeuristic::new(
        proposers[0],
        active_weight,
        inactive_weight,
        proposers.len(),
    );
    // 1. Window size not enough
    let weights = heuristic.get_weights(0, &proposers, &[]);
    assert_eq!(weights.len(), proposers.len());
    for w in weights {
        assert_eq!(w, inactive_weight);
    }
    // 2. Sliding window with [proposer 0, voters 1, 2], [proposer 0, voters 3]
    let weights = heuristic.get_weights(
        0,
        &proposers,
        &[
            block_builder.create_block(
                proposers[0],
                vec![false, true, true, false, false, false, false, false],
                vec![],
            ),
            block_builder.create_block(
                proposers[0],
                vec![false, false, false, true, false, false, false, false],
                vec![],
            ),
        ],
    );
    assert_eq!(weights.len(), proposers.len());
    for (i, w) in weights.iter().enumerate() {
        let expected = if i < 4 {
            active_weight
        } else {
            inactive_weight
        };
        assert_eq!(*w, expected);
    }
}

#[test]
fn test_epoch_change() {
    let active_weight = 9;
    let inactive_weight = 1;
    let mut proposers = vec![];
    let mut signers = vec![];
    for i in 0..8 {
        let signer = ValidatorSigner::random([i; 32]);
        proposers.push(signer.author());
        signers.push(signer);
    }
    let mut block_builder = TestBlockBuilder::new();
    let heuristic = ActiveInactiveHeuristic::new(
        proposers[0],
        active_weight,
        inactive_weight,
        proposers.len(),
    );
    // History with [proposer 0, voters 1, 2], [proposer 0, voters 3] in current epoch
    let weights = heuristic.get_weights(
        2,
        &proposers,
        &[
            block_builder.create_block(
                proposers[0],
                vec![false, true, true, true, true, true, true, true],
                vec![],
            ),
            block_builder.new_epoch().create_block(
                proposers[0],
                vec![false, true, true, true, true, true, true, true],
                vec![],
            ),
            block_builder.new_epoch().create_block(
                proposers[0],
                vec![false, true, true, false, false, false, false, false],
                vec![],
            ),
            block_builder.create_block(
                proposers[0],
                vec![false, false, false, true, false, false, false, false],
                vec![],
            ),
        ],
    );
    assert_eq!(weights.len(), proposers.len());
    for (i, w) in weights.iter().enumerate() {
        let expected = if i < 4 {
            active_weight
        } else {
            inactive_weight
        };
        assert_eq!(*w, expected);
    }
}

/// #### LeaderReputation test ####

#[test]
fn test_api() {
    let active_weight = 9;
    let inactive_weight = 1;
    let proposers: Vec<AccountAddress> =
        (0..5).map(|_| AccountAddress::random()).sorted().collect();
    let mut block_builder = TestBlockBuilder::new();
    let history = vec![
        block_builder.create_block(proposers[0], vec![false, true, true, false, false], vec![]),
        block_builder.create_block(proposers[0], vec![false, false, false, true, false], vec![]),
    ];
    let leader_reputation = LeaderReputation::new(
        0,
        proposers.clone(),
        Box::new(MockHistory::new(1, history)),
        Box::new(ActiveInactiveHeuristic::new(
            proposers[0],
            active_weight,
            inactive_weight,
            proposers.len(),
        )),
        4,
    );
    let round = 42u64;
    // first metadata is ignored because of window size 1
    let expected_weights = vec![
        active_weight,
        inactive_weight,
        inactive_weight,
        active_weight,
        inactive_weight,
    ];
    let sum = expected_weights.iter().fold(0, |mut s, w| {
        s += *w;
        s
    });
    let mut state = round.to_le_bytes().to_vec();
    let chosen_weight = next(&mut state) % sum;
    let mut expected_index = 0usize;
    let mut accu = 0u64;
    for (i, w) in expected_weights.iter().enumerate() {
        accu += *w;
        if accu >= chosen_weight {
            expected_index = i;
        }
    }
    let unexpected_index = (expected_index + 1) % proposers.len();
    let output = leader_reputation.get_valid_proposer(round);
    assert_eq!(output, proposers[expected_index]);
    assert!(leader_reputation.is_valid_proposer(proposers[expected_index], 42));
    assert!(!leader_reputation.is_valid_proposer(proposers[unexpected_index], 42));
}
