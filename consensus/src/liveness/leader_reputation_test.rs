// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::leader_reputation::{
    extract_epoch_to_proposers_impl, AptosDBBackend, ProposerAndVoterHeuristic,
};
use crate::liveness::{
    leader_reputation::{
        LeaderReputation, MetadataBackend, NewBlockEventAggregation, ReputationHeuristic,
    },
    proposer_election::{choose_index, ProposerElection},
};
use aptos_bitvec::BitVec;
use aptos_crypto::bls12381;
use aptos_infallible::Mutex;
use aptos_keygen::KeyGen;
use aptos_types::{
    account_address::AccountAddress,
    account_config::{new_block_event_key, NewBlockEvent},
    contract_event::{ContractEvent, EventWithVersion},
    epoch_state::EpochState,
    event::EventKey,
    transaction::Version,
    validator_verifier::{ValidatorConsensusInfo, ValidatorVerifier},
};
use claims::assert_err;
use consensus_types::common::{Author, Round};
use itertools::Itertools;
use move_deps::move_core_types::{language_storage::TypeTag, move_resource::MoveStructType};
use num_traits::Pow;
use std::{collections::HashMap, sync::Arc};
use storage_interface::{DbReader, Order};

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
    fn get_block_metadata(&self, _target_epoch: u64, _target_round: Round) -> Vec<NewBlockEvent> {
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
        self.round = 0;
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
            AccountAddress::random(),
            self.epoch,
            self.round,
            self.round,
            BitVec::from(voters).into(),
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

    if let Ok(voters) = NewBlockEventAggregation::bitvec_to_voters(&validators, &bitmap.into()) {
        assert_eq!(&validators[0], voters[0]);
        assert_eq!(&validators[1], voters[1]);
        assert_eq!(&validators[3], voters[2]);
    } else {
        unreachable!();
    }
}

#[test]
fn test_aggregation_bitmap_to_voters_mismatched_lengths() {
    let validators: Vec<_> = (0..8) // size of 8 with one u8 in bitvec
        .into_iter()
        .map(|_| Author::random())
        .collect();
    let bitmap_too_long = vec![true; 9]; // 2 bytes in bitvec
    assert!(
        NewBlockEventAggregation::bitvec_to_voters(&validators, &bitmap_too_long.into()).is_err()
    );
    let bitmap_too_short: Vec<bool> = vec![]; // 0 bytes in bitvec
    assert!(
        NewBlockEventAggregation::bitvec_to_voters(&validators, &bitmap_too_short.into()).is_err()
    );
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
    validators0: Vec<Author>,
    validators1: Vec<Author>,
    block_builder: TestBlockBuilder,
    history: Vec<NewBlockEvent>,
}

impl Example1 {
    fn new() -> Self {
        let mut sorted_validators: Vec<Author> =
            (0..5).into_iter().map(|_| Author::random()).collect();
        sorted_validators.sort();
        // same first 3 validators, different 4th validator (index 3).
        let mut validators0: Vec<Author> = sorted_validators[..3].to_vec();
        validators0.push(sorted_validators[3]);
        let mut validators1: Vec<Author> = validators0[..3].to_vec();
        validators1.push(sorted_validators[4]);

        Self {
            validators0,
            validators1,
            block_builder: TestBlockBuilder::new(),
            history: vec![],
        }
    }

    fn step1(&mut self) {
        self.history.push(self.block_builder.create_block(
            self.validators0[0],
            vec![false, true, true, false],
            vec![3],
        ));
        self.history.push(self.block_builder.create_block(
            self.validators0[0],
            vec![false, true, true, false],
            vec![],
        ));
        self.history.push(self.block_builder.create_block(
            self.validators0[1],
            vec![true, false, true, false],
            vec![2],
        ));
        self.history.push(self.block_builder.create_block(
            self.validators0[2],
            vec![true, true, false, false],
            vec![],
        ));
    }

    fn step2(&mut self) {
        self.history.push(self.block_builder.create_block(
            self.validators0[3],
            vec![true, true, false, false],
            vec![1],
        ));
        self.history.push(self.block_builder.create_block(
            self.validators0[3],
            vec![true, true, false, false],
            vec![1],
        ));
    }

    fn step3(&mut self) {
        self.block_builder.new_epoch();
        self.history.push(self.block_builder.create_block(
            self.validators1[3],
            vec![true, true, false, false],
            vec![0],
        ));
    }
}

#[test]
fn test_aggregation_counting() {
    let mut example1 = Example1::new();
    let validators0 = example1.validators0.clone();
    let epoch_to_validators = HashMap::from([(0u64, validators0.clone())]);
    let aggregation = NewBlockEventAggregation::new(2, 5);

    example1.step1();

    assert_eq!(
        aggregation.count_proposals(&epoch_to_validators, &example1.history),
        HashMap::from([
            (validators0[0], 2),
            (validators0[1], 1),
            (validators0[2], 1),
        ])
    );
    assert_eq!(
        aggregation.count_failed_proposals(&epoch_to_validators, &example1.history),
        HashMap::from([(validators0[2], 1), (validators0[3], 1),])
    );
    assert_eq!(
        aggregation.count_votes(&epoch_to_validators, &example1.history),
        HashMap::from([
            (validators0[0], 2),
            (validators0[1], 1),
            (validators0[2], 1),
        ])
    );

    example1.step2();

    assert_eq!(
        aggregation.count_proposals(&epoch_to_validators, &example1.history),
        HashMap::from([
            (validators0[0], 1),
            (validators0[1], 1),
            (validators0[2], 1),
            (validators0[3], 2),
        ])
    );
    assert_eq!(
        aggregation.count_failed_proposals(&epoch_to_validators, &example1.history),
        HashMap::from([(validators0[2], 1), (validators0[1], 2),])
    );
    assert_eq!(
        aggregation.count_votes(&epoch_to_validators, &example1.history),
        HashMap::from([(validators0[0], 2), (validators0[1], 2),])
    );

    example1.step3();

    let validators1 = example1.validators1.clone();
    let epoch_to_validators = HashMap::from([(1u64, validators1.clone())]);

    assert_eq!(
        aggregation.count_proposals(&epoch_to_validators, &example1.history),
        HashMap::from([(validators1[3], 1),])
    );
    assert_eq!(
        aggregation.count_failed_proposals(&epoch_to_validators, &example1.history),
        HashMap::from([(validators1[0], 1),])
    );
    assert_eq!(
        aggregation.count_votes(&epoch_to_validators, &example1.history),
        HashMap::from([(validators1[0], 1), (validators1[1], 1),])
    );

    let epoch_to_validators =
        HashMap::from([(0u64, validators0.clone()), (1u64, validators1.clone())]);

    assert_ne!(validators0[3], validators1[3]);

    assert_eq!(
        aggregation.count_proposals(&epoch_to_validators, &example1.history),
        HashMap::from([
            (validators1[1], 1),
            (validators1[2], 1),
            (validators0[3], 2),
            (validators1[3], 1),
        ])
    );
    assert_eq!(
        aggregation.count_failed_proposals(&epoch_to_validators, &example1.history),
        HashMap::from([
            (validators1[0], 1),
            (validators1[2], 1),
            (validators1[1], 2),
        ])
    );
    assert_eq!(
        aggregation.count_votes(&epoch_to_validators, &example1.history),
        HashMap::from([(validators1[0], 2), (validators1[1], 2),])
    );
}

/// ####

#[test]
fn test_proposer_and_voter_heuristic() {
    let mut example1 = Example1::new();
    let validators0 = example1.validators0.clone();
    let epoch_to_validators0 = HashMap::from([(0u64, validators0.clone())]);
    let heuristic = ProposerAndVoterHeuristic::new(example1.validators0[0], 100, 10, 1, 49, 2, 5);

    example1.step1();
    assert_eq!(
        heuristic.get_weights(0, &epoch_to_validators0, &example1.history),
        vec![100, 100, 1, 1]
    );

    example1.step2();
    assert_eq!(
        heuristic.get_weights(0, &epoch_to_validators0, &example1.history),
        vec![100, 1, 1, 100]
    );

    example1.step3();

    let validators1 = example1.validators1.clone();
    let epoch_to_validators1 = HashMap::from([(1u64, validators1.clone())]);
    assert_eq!(
        heuristic.get_weights(1, &epoch_to_validators1, &example1.history),
        vec![1, 100, 10, 100]
    );

    let epoch_to_validators01 = HashMap::from([(0u64, validators0), (1u64, validators1)]);
    assert_eq!(
        heuristic.get_weights(1, &epoch_to_validators01, &example1.history),
        vec![1, 1, 1, 100]
    );
}

/// #### LeaderReputation test ####

#[test]
fn test_api() {
    let active_weight: u64 = 9;
    let inactive_weight: u64 = 1;
    let proposers: Vec<AccountAddress> =
        (0..5).map(|_| AccountAddress::random()).sorted().collect();

    // 5 * base_stake just below u64::MAX
    let base_stake: u64 = 3_000_000_000_000_000_000;

    let voting_powers: Vec<u64> = (0..5).map(|i| base_stake * (i + 1)).collect();

    let mut block_builder = TestBlockBuilder::new();
    // first metadata is ignored because of window size 1
    let expected_weights = vec![
        active_weight as u128 * base_stake as u128,
        inactive_weight as u128 * (2 * base_stake) as u128,
        inactive_weight as u128 * (3 * base_stake) as u128,
        active_weight as u128 * (4 * base_stake) as u128,
        inactive_weight as u128 * (5 * base_stake) as u128,
    ];
    let total_weights: u128 = expected_weights.iter().sum();

    let mut selected = [0; 5].to_vec();
    for epoch in 1..1000 {
        block_builder.new_epoch();
        let history = vec![
            block_builder.create_block(proposers[0], vec![false, true, true, false, false], vec![]),
            block_builder.create_block(
                proposers[0],
                vec![false, false, false, true, false],
                vec![],
            ),
        ];
        let leader_reputation = LeaderReputation::new(
            epoch,
            HashMap::from([(epoch, proposers.clone())]),
            voting_powers.clone(),
            Box::new(MockHistory::new(1, history)),
            Box::new(ProposerAndVoterHeuristic::new(
                proposers[0],
                active_weight,
                inactive_weight,
                0,
                10,
                proposers.len(),
                proposers.len(),
            )),
            4,
        );
        let round = 42u64;
        let state = [epoch.to_le_bytes(), round.to_le_bytes()].concat().to_vec();

        let expected_index = choose_index(expected_weights.clone(), state);
        selected[expected_index] += 1;
        let unexpected_index = (expected_index + 1) % proposers.len();
        let output = leader_reputation.get_valid_proposer(round);
        assert_eq!(output, proposers[expected_index]);
        assert!(leader_reputation.is_valid_proposer(proposers[expected_index], round));
        assert!(!leader_reputation.is_valid_proposer(proposers[unexpected_index], round));
    }

    for i in 0..5 {
        let p = expected_weights[i] as f32 / total_weights as f32;
        let expected = (1000.0 * p) as i32;
        let std_dev = (1000.0 * p * (1.0 - p)).pow(0.5);
        // We've run the election enough times, to expect occurances to be close to the average
        // (each test is independent, as seed is different for every cycle)
        // We check that difference from average is below 3 standard deviations,
        // which will approximately be true in 99.7% of cases.
        // (as we can approximate each selection with normal distribution)
        //
        // Test is deterministic, as all seeds are, so if it passes once, shouldn't ever fail.
        // Meaning, wheen we change the selection formula, there is 0.3% chance this test will fail
        // unnecessarily.
        assert!(
            expected.abs_diff(selected[i]) as f32 <= 3.0 * std_dev,
            "{}: expected={} selected={}, std_dev: {}",
            i,
            expected,
            selected[i],
            std_dev
        );
    }
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

        let mut votes = BitVec::with_num_bits(1);
        votes.set(0);

        self.events.lock().push(EventWithVersion::new(
            *idx,
            ContractEvent::new(
                new_block_event_key(),
                *idx,
                TypeTag::Struct(NewBlockEvent::struct_tag()),
                bcs::to_bytes(&NewBlockEvent::new(
                    AccountAddress::random(),
                    epoch,
                    round,
                    round,
                    votes.into(),
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
        _ledger_version: Version,
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
    let backend = AptosDBBackend::new(3, 3, aptos_db.clone());

    aptos_db.add_event(0, 1);
    for i in 2..6 {
        aptos_db.add_event(1, i);
    }
    let mut fetch_count = 0;

    let mut assert_history = |round, expected_history: Vec<Round>, to_fetch| {
        let history: Vec<Round> = backend
            .get_block_metadata(1, round)
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
    assert_history(3, vec![3, 2, 1], false);
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

#[test]
fn backend_test_cross_epoch() {
    let aptos_db = Arc::new(MockDbReader::new());
    let backend = AptosDBBackend::new(3, 3, aptos_db.clone());

    aptos_db.add_event(0, 1);
    aptos_db.add_event(1, 1);
    aptos_db.add_event(1, 2);
    aptos_db.add_event(1, 3);
    aptos_db.add_event(2, 1);
    aptos_db.add_event(2, 2);

    let mut fetch_count = 0;

    let mut assert_history = |epoch, round, expected_history: Vec<(u64, Round)>, to_fetch| {
        let history: Vec<(u64, Round)> = backend
            .get_block_metadata(epoch, round)
            .iter()
            .map(|e| (e.epoch(), e.round()))
            .collect();
        assert_eq!(expected_history, history, "At round {}", round);
        if to_fetch {
            fetch_count += 1;
        }
        assert_eq!(fetch_count, aptos_db.fetched(), "At round {}", round);
    };

    assert_history(2, 2, vec![(2, 2), (2, 1), (1, 3)], true);
    assert_history(2, 1, vec![(2, 1), (1, 3), (1, 2)], false);

    aptos_db.add_event(3, 1);

    assert_history(3, 2, vec![(3, 1), (2, 2), (2, 1)], true);
}

#[test]
fn test_extract_epoch_to_proposers_impl() {
    fn create_epoch_state(
        epoch: u64,
        authors: &[Author],
        public_key: &bls12381::PublicKey,
    ) -> EpochState {
        EpochState {
            epoch,
            verifier: ValidatorVerifier::new(
                authors
                    .iter()
                    .map(|author| ValidatorConsensusInfo::new(*author, public_key.clone(), 1))
                    .collect::<Vec<_>>(),
            ),
        }
    }

    let private_key = KeyGen::from_os_rng().generate_bls12381_private_key();
    let public_key = bls12381::PublicKey::from(&private_key);
    let authors: Vec<AccountAddress> = (0..7).map(|_| AccountAddress::random()).sorted().collect();

    let epoch_states = (0..7)
        .map(|i| create_epoch_state(i as u64, &[authors[i]], &public_key))
        .collect::<Vec<_>>();

    // last EpochState needs to be for current epoch:
    assert_err!(extract_epoch_to_proposers_impl(
        &[(&epoch_states[1], 100u64)],
        2,
        &[authors[2]],
        1000
    ));
    assert_err!(extract_epoch_to_proposers_impl(
        &[(&epoch_states[2], 100u64), (&epoch_states[3], 100u64)],
        2,
        &[authors[2]],
        1000
    ));

    assert_eq!(
        HashMap::from([(2, vec![authors[2]])]),
        extract_epoch_to_proposers_impl(&[(&epoch_states[2], 100u64)], 2, &[authors[2]], 1000)
            .unwrap()
    );
    assert_eq!(
        HashMap::from([(2, vec![authors[2]])]),
        extract_epoch_to_proposers_impl(&[(&epoch_states[2], 10000u64)], 2, &[authors[2]], 1000)
            .unwrap()
    );

    assert_eq!(
        HashMap::from([(2, vec![authors[2]]), (3, vec![authors[3]])]),
        extract_epoch_to_proposers_impl(
            &[(&epoch_states[2], 100u64), (&epoch_states[3], 10000u64)],
            3,
            &[authors[3]],
            1000
        )
        .unwrap()
    );
    assert_eq!(
        HashMap::from([(2, vec![authors[2]]), (3, vec![authors[3]])]),
        extract_epoch_to_proposers_impl(
            &[
                (&epoch_states[1], 100u64),
                (&epoch_states[2], 100u64),
                (&epoch_states[3], 10000u64)
            ],
            3,
            &[authors[3]],
            1000
        )
        .unwrap()
    );
    assert_eq!(
        HashMap::from([
            (1, vec![authors[1]]),
            (2, vec![authors[2]]),
            (3, vec![authors[3]]),
            (4, vec![authors[4]]),
            (5, vec![authors[5]])
        ]),
        extract_epoch_to_proposers_impl(
            &[
                (&epoch_states[1], 1u64),
                (&epoch_states[2], 1u64),
                (&epoch_states[3], 1u64),
                (&epoch_states[4], 1u64),
                (&epoch_states[5], 1u64)
            ],
            5,
            &[authors[5]],
            1000
        )
        .unwrap()
    );

    assert_eq!(
        HashMap::from([
            (2, vec![authors[2]]),
            (3, vec![authors[3]]),
            (4, vec![authors[4]]),
            (5, vec![authors[5]])
        ]),
        extract_epoch_to_proposers_impl(
            &[
                (&epoch_states[1], 400u64),
                (&epoch_states[2], 400u64),
                (&epoch_states[3], 400u64),
                (&epoch_states[4], 400u64),
                (&epoch_states[5], 400u64)
            ],
            5,
            &[authors[5]],
            1000
        )
        .unwrap()
    );
}
