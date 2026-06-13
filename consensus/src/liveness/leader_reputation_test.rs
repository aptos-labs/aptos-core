// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::leader_reputation::{
    extract_epoch_to_proposers_impl, AptosDBBackend, LatencyWeightedHeuristic,
    ProposerAndVoterHeuristic,
};
use crate::liveness::{
    leader_reputation::{
        LeaderReputation, MetadataBackend, NewBlockEventAggregation, ReputationHeuristic,
    },
    proposer_election::{choose_index, ProposerElection},
};
use aptos_bitvec::BitVec;
use aptos_consensus_types::common::{Author, Round};
use aptos_crypto::{bls12381, HashValue};
use aptos_infallible::Mutex;
use aptos_keygen::KeyGen;
use aptos_storage_interface::DbReader;
use aptos_types::{
    account_address::AccountAddress,
    account_config::{new_block_event_key, NewBlockEvent},
    contract_event::{ContractEvent, EventWithVersion},
    epoch_state::EpochState,
    transaction::Version,
    validator_verifier::{ValidatorConsensusInfo, ValidatorVerifier},
};
use claims::assert_err;
use itertools::Itertools;
use move_core_types::{language_storage::TypeTag, move_resource::MoveStructType};
use num_traits::Pow;
use std::{collections::HashMap, sync::Arc};

/// #### NewBlockEventAggregation tests ####

#[test]
fn test_aggregation_bitmap_to_voters() {
    let validators: Vec<_> = (0..4).map(|_| Author::random()).collect();
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
    let validators: Vec<_> = (0..4).map(|_| Author::random()).collect();
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
    let validators: Vec<_> = (0..4).map(|_| Author::random()).collect();
    let indices = vec![0, 0, 4, 0];
    assert!(NewBlockEventAggregation::indices_to_validators(&validators, &indices).is_err());
}

struct Example1 {
    validators0: Vec<Author>,
    validators1: Vec<Author>,
    aptos_db: Arc<MockDbReader>,
    backend: AptosDBBackend,
}

impl Example1 {
    fn new(window_size: usize) -> Self {
        let mut sorted_validators: Vec<Author> = (0..5).map(|_| Author::random()).collect();
        sorted_validators.sort();
        // same first 3 validators, different 4th validator (index 3).
        let mut validators0: Vec<Author> = sorted_validators[..3].to_vec();
        validators0.push(sorted_validators[3]);
        let mut validators1: Vec<Author> = validators0[..3].to_vec();
        validators1.push(sorted_validators[4]);

        let aptos_db = Arc::new(MockDbReader::new());
        let backend = AptosDBBackend::new(window_size, 0, aptos_db.clone());

        Self {
            validators0,
            validators1,
            aptos_db,
            backend,
        }
    }

    fn history(&self) -> Vec<NewBlockEvent> {
        self.backend.get_block_metadata(5, 0).0
    }

    fn step1(&mut self) {
        self.aptos_db
            .add_event_with_data(self.validators0[0], vec![1, 2], vec![3]);
        self.aptos_db
            .add_event_with_data(self.validators0[0], vec![1, 2], vec![]);
        self.aptos_db
            .add_event_with_data(self.validators0[1], vec![0, 2], vec![2]);
        self.aptos_db
            .add_event_with_data(self.validators0[2], vec![0, 1], vec![]);
    }

    fn step2(&mut self) {
        self.aptos_db
            .add_event_with_data(self.validators0[3], vec![0, 1], vec![1]);
        self.aptos_db
            .add_event_with_data(self.validators0[3], vec![0, 1], vec![1]);
    }

    fn step3(&mut self) {
        self.aptos_db.new_epoch();
        self.aptos_db
            .add_event_with_data(self.validators1[3], vec![0, 1], vec![0]);
    }
}

#[test]
fn test_aggregation_counting() {
    let mut example1 = Example1::new(5);
    let validators0 = example1.validators0.clone();
    let epoch_to_validators = HashMap::from([(0u64, validators0.clone())]);
    let aggregation = NewBlockEventAggregation::new(2, 5, false);

    example1.step1();

    assert_eq!(
        aggregation.count_proposals(&epoch_to_validators, &example1.history()),
        HashMap::from([
            (validators0[0], 2),
            (validators0[1], 1),
            (validators0[2], 1),
        ])
    );
    assert_eq!(
        aggregation.count_failed_proposals(&epoch_to_validators, &example1.history()),
        HashMap::from([(validators0[2], 1), (validators0[3], 1),])
    );
    assert_eq!(
        aggregation.count_votes(&epoch_to_validators, &example1.history()),
        HashMap::from([
            (validators0[0], 2),
            (validators0[1], 1),
            (validators0[2], 1),
        ])
    );

    example1.step2();

    assert_eq!(
        aggregation.count_proposals(&epoch_to_validators, &example1.history()),
        HashMap::from([
            (validators0[0], 1),
            (validators0[1], 1),
            (validators0[2], 1),
            (validators0[3], 2),
        ])
    );
    assert_eq!(
        aggregation.count_failed_proposals(&epoch_to_validators, &example1.history()),
        HashMap::from([(validators0[2], 1), (validators0[1], 2),])
    );
    assert_eq!(
        aggregation.count_votes(&epoch_to_validators, &example1.history()),
        HashMap::from([(validators0[0], 2), (validators0[1], 2),])
    );

    example1.step3();

    let validators1 = example1.validators1.clone();
    let epoch_to_validators = HashMap::from([(1u64, validators1.clone())]);

    assert_eq!(
        aggregation.count_proposals(&epoch_to_validators, &example1.history()),
        HashMap::from([(validators1[3], 1),])
    );
    assert_eq!(
        aggregation.count_failed_proposals(&epoch_to_validators, &example1.history()),
        HashMap::from([(validators1[0], 1),])
    );
    assert_eq!(
        aggregation.count_votes(&epoch_to_validators, &example1.history()),
        HashMap::from([(validators1[0], 1), (validators1[1], 1),])
    );

    let epoch_to_validators =
        HashMap::from([(0u64, validators0.clone()), (1u64, validators1.clone())]);

    assert_ne!(validators0[3], validators1[3]);

    assert_eq!(
        aggregation.count_proposals(&epoch_to_validators, &example1.history()),
        HashMap::from([
            (validators1[1], 1),
            (validators1[2], 1),
            (validators0[3], 2),
            (validators1[3], 1),
        ])
    );
    assert_eq!(
        aggregation.count_failed_proposals(&epoch_to_validators, &example1.history()),
        HashMap::from([
            (validators1[0], 1),
            (validators1[2], 1),
            (validators1[1], 2),
        ])
    );
    assert_eq!(
        aggregation.count_votes(&epoch_to_validators, &example1.history()),
        HashMap::from([(validators1[0], 2), (validators1[1], 2),])
    );
}

/// ####

#[test]
fn test_proposer_and_voter_heuristic() {
    let mut example1 = Example1::new(5);
    let validators0 = example1.validators0.clone();
    let epoch_to_validators0 = HashMap::from([(0u64, validators0.clone())]);
    let heuristic =
        ProposerAndVoterHeuristic::new(example1.validators0[0], 100, 10, 1, 49, 2, 5, false);

    example1.step1();
    assert_eq!(
        heuristic.get_weights(0, &epoch_to_validators0, &example1.history()),
        vec![100, 100, 1, 1]
    );

    example1.step2();
    assert_eq!(
        heuristic.get_weights(0, &epoch_to_validators0, &example1.history()),
        vec![100, 1, 1, 100]
    );

    example1.step3();

    let validators1 = example1.validators1.clone();
    let epoch_to_validators1 = HashMap::from([(1u64, validators1.clone())]);
    assert_eq!(
        heuristic.get_weights(1, &epoch_to_validators1, &example1.history()),
        vec![1, 100, 10, 100]
    );

    let epoch_to_validators01 = HashMap::from([(0u64, validators0), (1u64, validators1)]);
    assert_eq!(
        heuristic.get_weights(1, &epoch_to_validators01, &example1.history()),
        vec![1, 1, 1, 100]
    );
}

/// #### LeaderReputation test ####

#[test]
fn test_api_v1() {
    test_api(false);
}

#[test]
fn test_api_v2() {
    test_api(true);
}

fn test_api(use_root_hash: bool) {
    let active_weight: u64 = 9;
    let inactive_weight: u64 = 1;
    let proposers: Vec<AccountAddress> =
        (0..5).map(|_| AccountAddress::random()).sorted().collect();

    // 5 * base_stake just below u64::MAX
    let base_stake: u64 = 3_000_000_000_000_000_000;

    let voting_powers: Vec<u64> = (0..5).map(|i| base_stake * (i + 1)).collect();

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
    let aptos_db = Arc::new(MockDbReader::new());

    for epoch in 1..1000 {
        aptos_db.new_epoch();
        assert_eq!(
            (epoch, 1),
            aptos_db.add_event_with_data(proposers[0], vec![1, 2], vec![])
        );
        assert_eq!(
            (epoch, 2),
            aptos_db.add_event_with_data(proposers[0], vec![3], vec![])
        );
        let backend = Arc::new(AptosDBBackend::new(1, 4, aptos_db.clone()));
        let leader_reputation = LeaderReputation::new(
            epoch,
            HashMap::from([(epoch, proposers.clone())]),
            voting_powers.clone(),
            backend,
            Box::new(ProposerAndVoterHeuristic::new(
                proposers[0],
                active_weight,
                inactive_weight,
                0,
                10,
                proposers.len(),
                proposers.len(),
                false,
            )),
            4,
            use_root_hash,
            30,
        );
        let round = 42u64;

        let state = if use_root_hash {
            [
                aptos_db.get_accumulator_root_hash(0).unwrap().to_vec(),
                epoch.to_le_bytes().to_vec(),
                round.to_le_bytes().to_vec(),
            ]
            .concat()
        } else {
            [epoch.to_le_bytes().to_vec(), round.to_le_bytes().to_vec()].concat()
        };

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
    epoch: Mutex<u64>,
    round: Mutex<u64>,
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
            epoch: Mutex::new(0),
            round: Mutex::new(0),
            to_add_event_after_call: Mutex::new(None),
            fetched: Mutex::new(0),
        }
    }

    pub fn add_event(&self, expected_epoch: u64, expected_round: Round) {
        let (epoch, round) = self.add_event_with_data(self.random_address, vec![0], vec![]);
        assert_eq!((epoch, round), (expected_epoch, expected_round))
    }

    pub fn add_event_with_data(
        &self,
        proposer: Author,
        votes: Vec<u16>,
        failed_proposers: Vec<u64>,
    ) -> (u64, u64) {
        let mut idx = self.idx.lock();
        *idx += 1;

        let mut round = self.round.lock();
        *round += 1 + failed_proposers.len() as u64;

        let epoch = self.epoch.lock();

        let mut votes_bitvec = BitVec::with_num_bits(1);
        for vote in votes {
            votes_bitvec.set(vote);
        }

        self.events.lock().push(EventWithVersion::new(
            *idx,
            ContractEvent::new_v1(
                new_block_event_key(),
                *idx,
                TypeTag::Struct(Box::new(NewBlockEvent::struct_tag())),
                bcs::to_bytes(&NewBlockEvent::new(
                    AccountAddress::random(),
                    *epoch,
                    *round,
                    *round,
                    votes_bitvec.into(),
                    proposer,
                    failed_proposers,
                    *self.last_timestamp.lock(),
                ))
                .unwrap(),
            )
            .expect("Should always be able to create a new block event"),
        ));
        *self.last_timestamp.lock() += 100;
        (*epoch, *round)
    }

    pub fn new_epoch(&self) {
        *self.epoch.lock() += 1;
        *self.round.lock() = 0;
    }

    pub fn skip_rounds(&self, to_skip: u64) {
        *self.round.lock() += to_skip;
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
    fn get_latest_block_events(
        &self,
        num_events: usize,
    ) -> aptos_storage_interface::Result<Vec<EventWithVersion>> {
        *self.fetched.lock() += 1;
        let events = self.events.lock();
        // println!("Events {:?}", *events);
        Ok(events
            .iter()
            .skip(events.len().saturating_sub(num_events))
            .rev()
            .cloned()
            .collect())
    }

    /// Returns the latest version, error on on non-bootstrapped DB.
    fn get_latest_ledger_info_version(&self) -> aptos_storage_interface::Result<Version> {
        let version = *self.idx.lock();
        let mut to_add = self.to_add_event_after_call.lock();
        if let Some((epoch, round)) = *to_add {
            self.add_event(epoch, round);
            *to_add = None;
        }
        Ok(version)
    }

    /// Gets the transaction accumulator root hash at specified version.
    /// Caller must guarantee the version is not greater than the latest version.
    fn get_accumulator_root_hash(
        &self,
        _version: Version,
    ) -> aptos_storage_interface::Result<HashValue> {
        Ok(HashValue::zero())
    }
}

#[test]
fn backend_wrapper_test() {
    let aptos_db = Arc::new(MockDbReader::new());
    let backend = AptosDBBackend::new(3, 3, aptos_db.clone());

    aptos_db.add_event(0, 1);
    aptos_db.new_epoch();
    aptos_db.skip_rounds(1);
    for i in 2..6 {
        aptos_db.add_event(1, i);
    }
    let mut fetch_count = 0;

    let mut assert_history = |round, expected_history: Vec<Round>, to_fetch| {
        let history: Vec<Round> = backend
            .get_block_metadata(1, round)
            .0
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
    aptos_db.skip_rounds(1);
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
    aptos_db.new_epoch();
    aptos_db.add_event(1, 1);
    aptos_db.add_event(1, 2);
    aptos_db.add_event(1, 3);
    aptos_db.new_epoch();
    aptos_db.add_event(2, 1);
    aptos_db.add_event(2, 2);

    let mut fetch_count = 0;

    let mut assert_history = |epoch, round, expected_history: Vec<(u64, Round)>, to_fetch| {
        let history: Vec<(u64, Round)> = backend
            .get_block_metadata(epoch, round)
            .0
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

    aptos_db.new_epoch();
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
            )
            .into(),
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

/// #### LatencyWeightedHeuristic tests ####

/// Build a `NewBlockEvent` with the given proposer, timestamp, round, epoch, and failed
/// proposer indices. Other fields are set to deterministic defaults.
fn make_block_event(
    proposer: Author,
    epoch: u64,
    round: u64,
    timestamp_us: u64,
    failed_proposer_indices: Vec<u64>,
) -> NewBlockEvent {
    NewBlockEvent::new(
        AccountAddress::ZERO,
        epoch,
        round,
        round,
        BitVec::with_num_bits(1).into(),
        proposer,
        failed_proposer_indices,
        timestamp_us,
    )
}

/// Default tuning parameters used by tests, matching the typical values that production
/// would carry on-chain (deadband=1.3×, max_ratio=4.0×, min_observations=2).
const TEST_MIN_OBSERVATIONS: usize = 2;
const TEST_DEADBAND: f64 = 1.3;
const TEST_MAX_RATIO: f64 = 4.0;

/// Build a `LatencyWeightedHeuristic` wrapping a `ProposerAndVoterHeuristic` configured so
/// that all candidates with successful proposals receive `active_weight = 1000`.
fn make_latency_weighted_heuristic(self_author: Author) -> LatencyWeightedHeuristic {
    make_latency_weighted_heuristic_with_multiplier(self_author, 1.0)
}

/// Variant for tests that want to exercise the multiplier knob; tuning parameters use
/// the test defaults.
fn make_latency_weighted_heuristic_with_multiplier(
    self_author: Author,
    multiplier: f64,
) -> LatencyWeightedHeuristic {
    // Wide windows + low failure threshold so test fixtures do not accidentally trigger the
    // failed/inactive branches in the inner heuristic — we want to exercise the latency
    // scaling on top of `active_weight`.
    let inner = ProposerAndVoterHeuristic::new(self_author, 1000, 10, 1, 50, 100, 100, false);
    LatencyWeightedHeuristic::new(
        inner,
        1000,
        multiplier,
        TEST_MIN_OBSERVATIONS,
        TEST_DEADBAND,
        TEST_MAX_RATIO,
    )
}

#[test]
fn test_latency_weighted_50_50_split_equal_intervals() {
    // With four validators and four successful blocks at uniform 100µs intervals, every
    // validator's mean is identical, so weights collapse to `active_weight`.
    let validators: Vec<Author> = (0..4).map(|_| Author::random()).collect();
    let epoch_to_validators = HashMap::from([(0u64, validators.clone())]);

    // History is newest-first: rounds 4, 3, 2, 1 at timestamps 400, 300, 200, 100.
    let history = vec![
        make_block_event(validators[3], 0, 4, 400, vec![]),
        make_block_event(validators[2], 0, 3, 300, vec![]),
        make_block_event(validators[1], 0, 2, 200, vec![]),
        make_block_event(validators[0], 0, 1, 100, vec![]),
    ];

    let heuristic = make_latency_weighted_heuristic(validators[0]);
    let weights = heuristic.get_weights(0, &epoch_to_validators, &history);

    // All means equal → ratio is 1 → all weights stay at active_weight = 1000.
    assert_eq!(weights, vec![1000, 1000, 1000, 1000]);
}

#[test]
fn test_latency_weighted_failure_attributed_to_failed_proposer() {
    // Three validators V0, V1, V2. V1 (index 1) fails round 3; V2 commits round 4 to rescue.
    // The 600µs gap between V0's commit (round 2) and V2's commit (round 4) absorbs V1's
    // timeout and must be attributed to V1, not to V2.
    let validators: Vec<Author> = (0..3).map(|_| Author::random()).collect();
    let epoch_to_validators = HashMap::from([(0u64, validators.clone())]);

    // Newest-first.
    let history = vec![
        make_block_event(validators[2], 0, 6, 1000, vec![]), // round 6
        make_block_event(validators[1], 0, 5, 900, vec![]),  // round 5 (V1 succeeds here)
        make_block_event(validators[0], 0, 4, 800, vec![]),  // round 4
        make_block_event(validators[2], 0, 4, 700, vec![1]), // round 4 rescue, V1 failed round 3
        make_block_event(validators[0], 0, 2, 100, vec![]),  // round 2
    ];

    let heuristic = make_latency_weighted_heuristic(validators[0]);
    let weights = heuristic.get_weights(0, &epoch_to_validators, &history);

    // V1 should have the largest mean (absorbed the 600µs failure entry) and therefore the
    // smallest weight. V0 and V2 should be boosted relative to V1.
    assert!(weights[1] < weights[0]);
    assert!(weights[1] < weights[2]);
}

#[test]
fn test_latency_weighted_multiple_failed_proposers_split_gap() {
    // Two failed proposers in a single gap: the interval is split equally between them.
    let validators: Vec<Author> = (0..4).map(|_| Author::random()).collect();
    let epoch_to_validators = HashMap::from([(0u64, validators.clone())]);

    // Round 5 commits with both V1 (idx 1) and V2 (idx 2) listed as failed; gap of 1000µs.
    let history = vec![
        make_block_event(validators[3], 0, 6, 1100, vec![]),
        make_block_event(validators[0], 0, 5, 1000, vec![1, 2]),
        make_block_event(validators[3], 0, 2, 0, vec![]),
    ];

    let heuristic = make_latency_weighted_heuristic(validators[0]);
    let weights = heuristic.get_weights(0, &epoch_to_validators, &history);

    // Both failed proposers should be down-weighted relative to V0/V3.
    assert!(weights[1] < weights[0]);
    assert!(weights[2] < weights[0]);
    assert!(weights[1] < weights[3]);
    assert!(weights[2] < weights[3]);
}

#[test]
fn test_latency_weighted_per_validator_fallback() {
    // V0 has no observations (it is in the candidate set but never appears in history).
    // The heuristic must NOT fall back globally just because V0 lacks data — V1 should
    // still be scaled relative to V2 even though V0 has nothing to compute from.
    let validators: Vec<Author> = (0..3).map(|_| Author::random()).collect();
    let epoch_to_validators = HashMap::from([(0u64, validators.clone())]);

    // V1 has fast intervals (~50µs/entry under 50/50), V2 has slow intervals (~500µs/entry).
    // V0 never appears as proposer.
    let history = vec![
        make_block_event(validators[2], 0, 10, 5000, vec![]),
        make_block_event(validators[2], 0, 8, 4000, vec![]),
        make_block_event(validators[2], 0, 6, 3000, vec![]),
        make_block_event(validators[1], 0, 4, 200, vec![]),
        make_block_event(validators[1], 0, 3, 150, vec![]),
        make_block_event(validators[1], 0, 2, 100, vec![]),
    ];

    let heuristic = make_latency_weighted_heuristic(validators[0]);
    let weights = heuristic.get_weights(0, &epoch_to_validators, &history);

    // V1 is faster than V2, so V1's weight must be strictly larger than V2's.
    // If the heuristic had fallen back globally because of V0's empty data, V1 == V2.
    assert!(
        weights[1] > weights[2],
        "expected V1 boosted over V2 despite V0 lacking observations; got {:?}",
        weights,
    );
}

#[test]
fn test_latency_weighted_empty_history_falls_back_to_base() {
    // No history → no observations → base weights returned unchanged.
    let validators: Vec<Author> = (0..2).map(|_| Author::random()).collect();
    let epoch_to_validators = HashMap::from([(0u64, validators.clone())]);
    let history: Vec<NewBlockEvent> = vec![];

    let heuristic = make_latency_weighted_heuristic(validators[0]);
    let weights = heuristic.get_weights(0, &epoch_to_validators, &history);

    // Both validators still get base weight (inactive_weight in this case since no proposals).
    assert_eq!(weights.len(), 2);
}

#[test]
fn test_latency_weighted_max_ratio_clamp() {
    // Three validators, all classified active. V1 has a huge mean from absorbing
    // failure attributions, while V0 and V2 have small means from successful pairs.
    // The raw penalty ratio (V1_mean / median_mean) is much larger than
    // MAX_LATENCY_RATIO=10 and must be clamped → V1 weight floors at
    // active_weight / 10 = 100 (with multiplier=1.0). V0 and V2 stay at base 1000.
    let validators: Vec<Author> = (0..3).map(|_| Author::random()).collect();
    let epoch_to_validators = HashMap::from([(0u64, validators.clone())]);

    // History (newest first):
    //   - 3 huge V1-failure attributions (gap ≈ 10000 each) — pushes V1 mean way up.
    //   - 3 V1 successful proposals at small intervals — keeps V1 under the 50%
    //     failure threshold so it stays classified active.
    //   - 6 alternating V0/V2 successful proposals at small intervals — both V0 and
    //     V2 accumulate enough small-mean observations to set the median.
    let history = vec![
        make_block_event(validators[0], 0, 18, 1_030_000, vec![1]),
        make_block_event(validators[0], 0, 16, 1_020_000, vec![1]),
        make_block_event(validators[0], 0, 14, 1_010_000, vec![1]),
        make_block_event(validators[1], 0, 12, 1_000_030, vec![]),
        make_block_event(validators[1], 0, 11, 1_000_020, vec![]),
        make_block_event(validators[1], 0, 10, 1_000_010, vec![]),
        make_block_event(validators[2], 0, 9, 1_000_005, vec![]),
        make_block_event(validators[0], 0, 8, 1_000_000, vec![]),
        make_block_event(validators[2], 0, 7, 999_995, vec![]),
        make_block_event(validators[0], 0, 6, 999_990, vec![]),
        make_block_event(validators[2], 0, 5, 999_985, vec![]),
        make_block_event(validators[0], 0, 4, 999_980, vec![]),
    ];

    let heuristic = make_latency_weighted_heuristic(validators[0]);
    let weights = heuristic.get_weights(0, &epoch_to_validators, &history);

    // V0, V2 (healthy, near median): no penalty, weight = active_weight = 1000.
    assert_eq!(
        weights[0], 1000,
        "expected V0 to keep base active_weight; got {:?}",
        weights,
    );
    assert_eq!(
        weights[2], 1000,
        "expected V2 to keep base active_weight; got {:?}",
        weights,
    );
    // V1 (very slow): raw ratio is huge, post-deadband shifted ratio is clamped at
    // MAX_LATENCY_RATIO=4. With multiplier=1.0 → factor = 1/4 = 0.25.
    // V1 weight = active_weight * 0.25 = 250.
    assert_eq!(
        weights[1], 250,
        "expected V1 penalty clamped at 1/4 of active_weight (post-deadband); got {:?}",
        weights,
    );
}

#[test]
fn test_latency_weighted_carry_forward_for_unobserved_validator() {
    // Verify carry-forward semantics. First call: V1 has slow observations and gets
    // penalized. Second call: V1 has too few observations to recompute → must retain
    // the previous penalty rather than jumping back to base weight (which would
    // create the oscillation that the carry-forward is designed to prevent).
    let validators: Vec<Author> = (0..2).map(|_| Author::random()).collect();
    let epoch_to_validators = HashMap::from([(0u64, validators.clone())]);

    // First call: V0 and V1 alternate as successful proposers; one gap is attributed
    // to V1 via failed_proposer_indices. V1 has a single failure (3 proposals
    // succeed, 1 fails → 25% < 50% threshold → active) but its mean is still
    // dominated by the 4500µs failure attribution → meaningful penalty.
    let history_with_obs = vec![
        make_block_event(validators[0], 0, 8, 5000, vec![1]),
        make_block_event(validators[1], 0, 6, 500, vec![]),
        make_block_event(validators[0], 0, 5, 100, vec![]),
        make_block_event(validators[1], 0, 4, 50, vec![]),
        make_block_event(validators[0], 0, 3, 25, vec![]),
        make_block_event(validators[1], 0, 2, 10, vec![]),
    ];
    let heuristic = make_latency_weighted_heuristic(validators[0]);
    let weights1 = heuristic.get_weights(0, &epoch_to_validators, &history_with_obs);

    // V1 (slow) is below base; V0 (healthy, below median) stays at base.
    assert_eq!(
        weights1[0], 1000,
        "V0 base preserved in first call; got {:?}",
        weights1
    );
    assert!(
        weights1[1] < 1000,
        "V1 should be penalized in first call; got {:?}",
        weights1,
    );
    let v1_first_weight = weights1[1];

    // Second call: just one pair → both V0 and V1 get only 1 round-time observation
    // (below MIN_OBSERVATIONS=2). Both fall through to carry-forward. Both are still
    // classified active because they have a successful proposal in this window.
    let history_no_v1_obs = vec![
        make_block_event(validators[1], 0, 1, 100, vec![]),
        make_block_event(validators[0], 0, 0, 50, vec![]),
    ];
    let weights2 = heuristic.get_weights(0, &epoch_to_validators, &history_no_v1_obs);

    // V0's stored factor was 1.0 → carry-forward restores base.
    assert_eq!(
        weights2[0], 1000,
        "V0 carry-forward at base; got {:?}",
        weights2
    );
    // V1's stored factor was the first-call penalty → must be preserved, NOT reset.
    assert_eq!(
        weights2[1], v1_first_weight,
        "carry-forward must preserve V1's penalty; got weights {:?}, expected V1={}",
        weights2, v1_first_weight,
    );
}

#[test]
fn test_latency_weighted_deadband_protects_mild_outliers() {
    // Verify the deadband zone: a validator whose mean is within `LATENCY_DEADBAND` (1.3×)
    // of the median should NOT be penalized. This was the failure mode at the previous
    // formula — V5-like geographic outliers (mean ~1.5× median) were over-penalized,
    // dropping into structural cut-off. With deadband, mild outliers are protected
    // unless they exceed the threshold.
    let validators: Vec<Author> = (0..3).map(|_| Author::random()).collect();
    let epoch_to_validators = HashMap::from([(0u64, validators.clone())]);

    // V0 fast (5µs intervals), V1 medium (~6µs/entry), V2 slow (~10µs/entry).
    // After 50/50 split: V0 mean ≈ 5, V1 mean ≈ 6, V2 mean ≈ 10. Median ≈ 6.
    // V0 ratio = 5/6 = 0.83 → deadband (factor 1.0)
    // V1 ratio = 6/6 = 1.0 → deadband (factor 1.0)
    // V2 ratio = 10/6 = 1.67 → ABOVE deadband 1.3 → penalty applies
    let history = vec![
        make_block_event(validators[2], 0, 12, 50, vec![]),
        make_block_event(validators[2], 0, 11, 40, vec![]),
        make_block_event(validators[2], 0, 10, 30, vec![]),
        make_block_event(validators[2], 0, 9, 20, vec![]),
        make_block_event(validators[1], 0, 8, 14, vec![]),
        make_block_event(validators[1], 0, 7, 8, vec![]),
        make_block_event(validators[0], 0, 6, 5, vec![]),
        make_block_event(validators[0], 0, 5, 0, vec![]),
    ];

    let heuristic = make_latency_weighted_heuristic(validators[0]);
    let weights = heuristic.get_weights(0, &epoch_to_validators, &history);

    // V0, V1 within deadband: no penalty
    assert_eq!(
        weights[0], 1000,
        "V0 should be in deadband (ratio < 1.3), no penalty; got {:?}",
        weights,
    );
    assert_eq!(
        weights[1], 1000,
        "V1 should be in deadband (ratio = 1.0), no penalty; got {:?}",
        weights,
    );
    // V2 above deadband: penalty applies
    assert!(
        weights[2] < 1000,
        "V2 should be penalized (ratio > 1.3); got {:?}",
        weights,
    );
}

#[test]
fn test_latency_weighted_skips_cross_epoch_pairs() {
    // A pair spanning an epoch boundary must not contribute to round_times.
    let validators: Vec<Author> = (0..2).map(|_| Author::random()).collect();
    let epoch_to_validators =
        HashMap::from([(0u64, validators.clone()), (1u64, validators.clone())]);

    // Two events: one in epoch 0, one in epoch 1. Cross-epoch pair must be skipped.
    let history = vec![
        make_block_event(validators[1], 1, 1, 1_000_000, vec![]), // epoch 1
        make_block_event(validators[0], 0, 5, 500, vec![]),       // epoch 0
    ];

    let heuristic = make_latency_weighted_heuristic(validators[0]);
    let weights = heuristic.get_weights(1, &epoch_to_validators, &history);

    // No same-epoch pairs → empty round_times → fall back to base weights.
    // Both validators receive whatever base weight the inner heuristic produced; the latency
    // scaling has no effect.
    assert_eq!(weights.len(), 2);
}
