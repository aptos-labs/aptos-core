// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{AptosDB, EventStore, LedgerPrunerManager, PrunerManager};
use aptos_config::config::LedgerPrunerConfig;
use aptos_proptest_helpers::Index;
use aptos_temppath::TempPath;
use aptos_types::{
    contract_event::ContractEvent,
    proptest_types::{AccountInfoUniverse, ContractEventGen},
    transaction::Version,
};
use proptest::{collection::vec, prelude::*, proptest};
use schemadb::SchemaBatch;
use std::sync::Arc;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))]

    #[test]
    fn test_event_store_pruner(
        mut universe in any_with::<AccountInfoUniverse>(3),
        gen_batches in vec(vec((any::<Index>(), any::<ContractEventGen>()), 0..=2), 0..100),
    ) {
        let event_batches = gen_batches
            .into_iter()
            .map(|gens| {
                gens.into_iter()
                    .map(|(index, gen)| gen.materialize(*index, &mut universe))
                    .collect()
            })
            .collect();

        verify_event_store_pruner(event_batches);
    }

        #[test]
    fn test_event_store_pruner_disabled(
        mut universe in any_with::<AccountInfoUniverse>(3),
        gen_batches in vec(vec((any::<Index>(), any::<ContractEventGen>()), 0..=2), 0..4),
    ) {
        let event_batches = gen_batches
            .into_iter()
            .map(|gens| {
                gens.into_iter()
                    .map(|(index, gen)| gen.materialize(*index, &mut universe))
                    .collect()
            })
            .collect();

        verify_event_store_pruner_disabled(event_batches);
    }
}

fn verify_event_store_pruner(events: Vec<Vec<ContractEvent>>) {
    let tmp_dir = TempPath::new();
    let aptos_db = AptosDB::new_for_test(&tmp_dir);
    let event_store = &aptos_db.event_store;
    let mut batch = SchemaBatch::new();
    let num_versions = events.len();

    // Write events to DB
    for (version, events_for_version) in events.iter().enumerate() {
        event_store
            .put_events(version as u64, events_for_version, &mut batch)
            .unwrap();
    }
    aptos_db.ledger_db.write_schemas(batch).unwrap();

    let pruner = LedgerPrunerManager::new(
        Arc::clone(&aptos_db.ledger_db),
        Arc::clone(&aptos_db.state_store),
        LedgerPrunerConfig {
            enable: true,
            prune_window: 0,
            batch_size: 1,
            user_pruning_window_offset: 0,
        },
    );
    // start pruning events batches of size 2 and verify transactions have been pruned from DB
    for i in (0..=num_versions).step_by(2) {
        pruner
            .wake_and_wait_pruner(i as u64 /* latest_version */)
            .unwrap();
        // ensure that all events up to i has been pruned
        for j in 0..i {
            verify_events_not_in_store(j as u64, event_store);
            verify_event_by_key_not_in_store(&events, j as u64, event_store);
            verify_event_by_version_not_in_store(&events, j as u64, event_store);
        }
        // ensure all other events are valid in DB
        for j in i..num_versions {
            verify_events_in_store(&events, j as u64, event_store);
            verify_event_by_key_in_store(&events, j as u64, event_store);
            verify_event_by_version_in_store(&events, j as u64, event_store);
        }
    }
}

fn verify_event_store_pruner_disabled(events: Vec<Vec<ContractEvent>>) {
    let tmp_dir = TempPath::new();
    let aptos_db = AptosDB::new_for_test(&tmp_dir);
    let event_store = &aptos_db.event_store;
    let mut batch = SchemaBatch::new();
    let num_versions = events.len();

    // Write events to DB
    for (version, events_for_version) in events.iter().enumerate() {
        event_store
            .put_events(version as u64, events_for_version, &mut batch)
            .unwrap();
    }
    aptos_db.ledger_db.write_schemas(batch).unwrap();

    // Verify no pruning has happened.
    for _i in (0..=num_versions).step_by(2) {
        // ensure that all events up to i * 2 are valid in DB
        for version in 0..num_versions {
            verify_events_in_store(&events, version as u64, event_store);
            verify_event_by_key_in_store(&events, version as u64, event_store);
            verify_event_by_version_in_store(&events, version as u64, event_store);
        }
    }
}

fn verify_event_by_key_not_in_store(
    events: &[Vec<ContractEvent>],
    version: Version,
    event_store: &Arc<EventStore>,
) {
    for event in &events[version as usize] {
        assert!(event_store
            .get_txn_ver_by_seq_num(event.key(), event.sequence_number())
            .is_err())
    }
}

fn verify_event_by_key_in_store(
    events: &[Vec<ContractEvent>],
    version: Version,
    event_store: &Arc<EventStore>,
) {
    for event in events.get(version as usize).unwrap() {
        assert_eq!(
            event_store
                .get_txn_ver_by_seq_num(event.key(), event.sequence_number())
                .unwrap(),
            version
        );
    }
}

fn verify_event_by_version_not_in_store(
    events: &[Vec<ContractEvent>],
    version: Version,
    event_store: &Arc<EventStore>,
) {
    for event in events.get(version as usize).unwrap() {
        assert!(event_store
            .get_latest_sequence_number(version, event.key())
            .unwrap()
            .is_none());
    }
}

fn verify_event_by_version_in_store(
    events: &[Vec<ContractEvent>],
    version: Version,
    event_store: &Arc<EventStore>,
) {
    for event in events.get(version as usize).unwrap() {
        assert!(event_store
            .get_latest_sequence_number(version, event.key())
            .unwrap()
            .is_some());
    }
}

fn verify_events_not_in_store(version: Version, event_store: &Arc<EventStore>) {
    assert!(event_store
        .get_events_by_version(version)
        .unwrap()
        .is_empty());
}

fn verify_events_in_store(
    events: &[Vec<ContractEvent>],
    version: Version,
    event_store: &Arc<EventStore>,
) {
    let events_from_db = event_store.get_events_by_version(version as u64).unwrap();
    assert_eq!(
        events_from_db.len(),
        events.get(version as usize).unwrap().len()
    );
    assert_eq!(events_from_db, events[version as usize]);
}
