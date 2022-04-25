// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{pruner::*, AptosDB, ChangeSet};
use aptos_proptest_helpers::Index;
use aptos_temppath::TempPath;
use aptos_types::{
    contract_event::ContractEvent,
    proptest_types::{AccountInfoUniverse, ContractEventGen},
};
use proptest::{collection::vec, prelude::*, proptest};

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

}

fn verify_event_store_pruner(events: Vec<Vec<ContractEvent>>) {
    let tmp_dir = TempPath::new();
    let aptos_db = AptosDB::new_for_test(&tmp_dir);
    let event_store = &aptos_db.event_store;
    let mut cs = ChangeSet::new();
    let num_versions = events.len();
    let pruner = Pruner::new(
        Arc::clone(&aptos_db.db),
        StoragePrunerConfig {
            state_store_prune_window: Some(0),
            ledger_prune_window: Some(0),
            pruning_batch_size: 1,
        },
        Arc::clone(&aptos_db.transaction_store),
        Arc::clone(&aptos_db.ledger_store),
        Arc::clone(&aptos_db.event_store),
    );

    // Write events to DB
    for (version, events_for_version) in events.iter().enumerate() {
        event_store
            .put_events(version as u64, events_for_version, &mut cs)
            .unwrap();
    }
    aptos_db.db.write_schemas(cs.batch).unwrap();

    // start pruning events batches of size 2 and verify transactions have been pruned from DB
    for i in (0..=num_versions).step_by(2) {
        pruner
            .wake_and_wait(
                i as u64, /* latest_version */
                PrunerIndex::LedgerPrunerIndex as usize,
            )
            .unwrap();
        // ensure that all events up to i * 2 has been pruned
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
