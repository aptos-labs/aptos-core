// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashMap, sync::Arc};

use crate::liveness::{
    leader_reputation::{
        ActiveInactiveHeuristic, LeaderReputation, MetadataBackend, NewBlockEventAggregation,
        ReputationHeuristic,
    },
    proposer_election::{next, ProposerElection},
};

use aptos_infallible::Mutex;
use aptos_types::{
    account_address::AccountAddress,
    account_config::NewBlockEvent,
    block_metadata::new_block_event_key,
    contract_event::{ContractEvent, EventWithVersion},
    event::EventKey,
    transaction::Version,
    validator_signer::ValidatorSigner,
};
use consensus_types::common::{Author, Round};
use itertools::Itertools;
use move_deps::move_core_types::{language_storage::TypeTag, move_resource::MoveStructType};
use storage_interface::{DbReader, Order};

use super::leader_reputation::{AptosDBBackend, ProposerAndVoterHeuristic};

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

struct MockDbReader {
    events: Mutex<Vec<EventWithVersion>>,
    random_address: Author,
    last_timestamp: Mutex<u64>,
    idx: Mutex<u64>,
    to_add_event_after_call: Mutex<Option<(u64, Round)>>,

    fetched: Mutex<usize>,
}

impl MockDbReader {
    pub fn new() -> MockDbReader {
        Self {
            events: Mutex::new(vec![]),
            random_address: Author::random(),
            last_timestamp: Mutex::new(100000),
            idx: Mutex::new(0),
            to_add_event_after_call: Mutex::new(None),
            fetched: Mutex::new(0),
        }
    }

    pub fn add_event(&self, epoch: u64, round: Round) {
        let mut idx = self.idx.lock();
        *idx += 1;
        self.events.lock().push(EventWithVersion::new(
            *idx,
            ContractEvent::new(
                new_block_event_key(),
                *idx,
                TypeTag::Struct(NewBlockEvent::struct_tag()),
                bcs::to_bytes(&NewBlockEvent::new(
                    epoch,
                    round,
                    vec![],
                    self.random_address,
                    vec![],
                    *self.last_timestamp.lock(),
                ))
                .unwrap(),
            ),
        ));
        *self.last_timestamp.lock() += 100;
    }

    pub fn add_another_transaction(&self) {
        *self.idx.lock() += 1;
    }

    pub fn add_event_after_call(&self, epoch: u64, round: Round) {
        *self.to_add_event_after_call.lock() = Some((epoch, round));
    }

    fn fetched(&self) -> usize {
        *self.fetched.lock()
    }
}

impl DbReader for MockDbReader {
    fn get_events(
        &self,
        _event_key: &EventKey,
        start: u64,
        order: Order,
        limit: u64,
    ) -> anyhow::Result<Vec<EventWithVersion>> {
        *self.fetched.lock() += 1;
        assert_eq!(start, u64::max_value());
        assert!(order == Order::Descending);
        let events = self.events.lock();
        // println!("Events {:?}", *events);
        Ok(events
            .iter()
            .skip(events.len().saturating_sub(limit as usize))
            .rev()
            .cloned()
            .collect())
    }

    /// Returns the latest version, error on on non-bootstrapped DB.
    fn get_latest_version(&self) -> anyhow::Result<Version> {
        let version = *self.idx.lock();
        let mut to_add = self.to_add_event_after_call.lock();
        if let Some((epoch, round)) = *to_add {
            self.add_event(epoch, round);
            *to_add = None;
        }
        Ok(version)
    }
}

#[test]
fn backend_wrapper_test() {
    let aptos_db = Arc::new(MockDbReader::new());
    let backend = AptosDBBackend::new(1, 3, 3, aptos_db.clone());

    aptos_db.add_event(0, 1);
    for i in 2..6 {
        aptos_db.add_event(1, i);
    }
    let mut fetch_count = 0;

    let mut assert_history = |round, expected_history: Vec<Round>, to_fetch| {
        let history: Vec<Round> = backend
            .get_block_metadata(round)
            .iter()
            .map(|e| e.round())
            .collect();
        assert_eq!(expected_history, history, "At round {}", round);
        if to_fetch {
            fetch_count += 1;
        }
        assert_eq!(fetch_count, aptos_db.fetched(), "At round {}", round);
    };

    assert_history(6, vec![5, 4, 3], true);
    // while history doesn't change, no need to refetch, no matter the round
    assert_history(5, vec![5, 4, 3], false);
    assert_history(4, vec![4, 3, 2], false);
    assert_history(3, vec![3, 2], false);
    assert_history(5, vec![5, 4, 3], false);
    assert_history(6, vec![5, 4, 3], false);

    // as soon as history change, we fetch again
    aptos_db.add_event(1, 6);
    assert_history(6, vec![6, 5, 4], true);
    aptos_db.add_event(1, 7);
    assert_history(6, vec![6, 5, 4], false);
    aptos_db.add_event(1, 8);
    assert_history(6, vec![6, 5, 4], false);

    assert_history(9, vec![8, 7, 6], true);
    aptos_db.add_event(1, 10);
    // we need to refetch, as we don't know if round that arrived is for 9 or not.
    assert_history(9, vec![8, 7, 6], true);
    assert_history(9, vec![8, 7, 6], false);
    aptos_db.add_event(1, 11);
    // since we already saw round 10, and are asking for round 9, no need to fetch again.
    assert_history(9, vec![8, 7, 6], false);
    aptos_db.add_event(1, 12);
    assert_history(9, vec![8, 7, 6], false);

    // last time we fetched, we saw 10, so we don't need to fetch for 10
    // but need to fetch for 11.
    assert_history(10, vec![10, 8, 7], false);
    assert_history(11, vec![11, 10, 8], true);
    assert_history(12, vec![12, 11, 10], false);

    // since history include target round, unrelated transaction don't require refresh
    aptos_db.add_another_transaction();
    assert_history(12, vec![12, 11, 10], false);

    // since history doesn't include target round, any unrelated transaction requires refresh
    assert_history(13, vec![12, 11, 10], true);
    aptos_db.add_another_transaction();
    assert_history(13, vec![12, 11, 10], true);
    assert_history(13, vec![12, 11, 10], false);
    aptos_db.add_another_transaction();
    assert_history(13, vec![12, 11, 10], true);
    assert_history(13, vec![12, 11, 10], false);

    // check for race condition
    aptos_db.add_another_transaction();
    aptos_db.add_event_after_call(1, 13);
    // in the first we add event after latest_db_version is fetched, as a race.
    // Second one should know that there is nothing new.
    assert_history(14, vec![13, 12, 11], true);
    assert_history(14, vec![13, 12, 11], false);
}
