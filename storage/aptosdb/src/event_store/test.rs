// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::*;
use crate::{db::AptosDB, event_store::EventStore};
use aptos_crypto::hash::ACCUMULATOR_PLACEHOLDER_HASH;
use aptos_proptest_helpers::Index;
use aptos_temppath::TempPath;
use aptos_types::{
    account_address::AccountAddress,
    contract_event::ContractEvent,
    event::EventKey,
    proptest_types::{AccountInfoUniverse, ContractEventGen},
};
use itertools::Itertools;
use move_core_types::{language_storage::TypeTag, move_resource::MoveStructType};
use proptest::{
    collection::{hash_set, vec},
    prelude::*,
    strategy::Union,
};
use rand::Rng;
use std::collections::HashMap;

#[test]
fn test_error_on_get_from_empty() {
    let tmp_dir = TempPath::new();
    let db = AptosDB::new_for_test(&tmp_dir);
    let store = &db.event_store;

    assert!(store.get_event_by_version_and_index(100, 0).is_err());
}

fn traverse_events_by_key(
    store: &EventStore,
    event_key: &EventKey,
    ledger_version: Version,
) -> Vec<ContractEvent> {
    const LIMIT: u64 = 3;

    let mut seq_num = 0;

    let mut event_keys = Vec::new();
    let mut last_batch_len = LIMIT;
    loop {
        let mut batch = store
            .lookup_events_by_key(event_key, seq_num, LIMIT, ledger_version)
            .unwrap();
        if last_batch_len < LIMIT {
            assert!(batch.is_empty());
        }
        if batch.is_empty() {
            break;
        }

        last_batch_len = batch.len() as u64;
        let first_seq = batch.first().unwrap().0;
        let last_seq = batch.last().unwrap().0;

        assert!(last_batch_len <= LIMIT);
        assert_eq!(seq_num, first_seq);
        assert_eq!(seq_num + last_batch_len - 1, last_seq);

        event_keys.extend(batch.iter());
        seq_num = last_seq + 1;
    }

    event_keys
        .into_iter()
        .map(|(_seq, ver, idx)| store.get_event_by_version_and_index(ver, idx).unwrap())
        .collect()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))]

    #[test]
    fn test_index_get(
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

        test_index_get_impl(event_batches);
    }
}

fn test_index_get_impl(event_batches: Vec<Vec<ContractEvent>>) {
    // Put into db.
    let tmp_dir = TempPath::new();
    let db = AptosDB::new_for_test(&tmp_dir);
    let store = &db.event_store;
    let event_db = &db.ledger_db.event_db();

    let mut batch = SchemaBatch::new();
    event_batches.iter().enumerate().for_each(|(ver, events)| {
        event_db
            .put_events(ver as u64, events, /*skip_index=*/ false, &mut batch)
            .unwrap();
    });
    event_db.write_schemas(batch);
    let ledger_version_plus_one = event_batches.len() as u64;

    assert_eq!(
        event_db
            .get_events_by_version_iter(0, event_batches.len())
            .unwrap()
            .collect::<Result<Vec<_>>>()
            .unwrap(),
        event_batches,
    );

    // Calculate expected event sequence per access_path.
    let mut events_by_event_key = HashMap::new();
    event_batches
        .into_iter()
        .enumerate()
        .for_each(|(ver, batch)| {
            batch
                .into_iter()
                .filter(|e| matches!(e, ContractEvent::V1(_)))
                .for_each(|e| {
                    let mut events_and_versions = events_by_event_key
                        .entry(*e.v1().unwrap().key())
                        .or_insert_with(Vec::new);
                    assert_eq!(
                        events_and_versions.len() as u64,
                        e.v1().unwrap().sequence_number()
                    );
                    events_and_versions.push((e, ver as Version));
                })
        });

    // Fetch and check.
    events_by_event_key
        .into_iter()
        .for_each(|(path, events_and_versions)| {
            // Check sequence number
            let mut prev_ver = 0;
            let mut iter = events_and_versions.iter().enumerate().peekable();
            while let Some((mut seq, (_, ver))) = iter.next() {
                let mid = prev_ver + (*ver - prev_ver) / 2;
                if mid < *ver {
                    assert_eq!(
                        store.get_next_sequence_number(mid, &path).unwrap(),
                        seq as u64,
                        "next_seq equals this since last seq bump.",
                    );
                }
                // possible multiple emits of the event in the same version
                let mut last_seq_in_same_version = seq;
                while let Some((next_seq, (_, next_ver))) = iter.peek() {
                    if next_ver != ver {
                        break;
                    }
                    last_seq_in_same_version = *next_seq;
                    iter.next();
                }

                assert_eq!(
                    store.get_latest_sequence_number(*ver, &path).unwrap(),
                    Some(last_seq_in_same_version as u64),
                    "latest_seq equals this at its version.",
                );

                prev_ver = *ver;
            }

            // Fetch by key
            let events = events_and_versions
                .into_iter()
                .map(|(e, _)| e)
                .collect::<Vec<_>>();
            let traversed = traverse_events_by_key(store, &path, ledger_version_plus_one);
            assert_eq!(events, traversed);
        });
}

prop_compose! {
    fn arb_new_block_events()(
        hash in any::<AccountAddress>(),
        address in any::<AccountAddress>(),
        mut version in 1..10000u64,
        mut timestamp in 0..1000000u64, // initial timestamp
        block_bumps in vec(
            prop_oneof![
                Just((1, 0)), // NIL Block
                (1..100u64, 1..100u64) // normal block
            ], // version and timestamp bump
            1..100,
        )
    ) -> Vec<(Version, ContractEvent)> {
        let mut seq = 0;
        block_bumps.into_iter().map(|(v, t)| {
            version += v;
            timestamp += t;
            let new_block_event = NewBlockEvent::new(
                hash,
                0, // epoch
                seq, // round
                seq, // height
                vec![], // prev block voters
                address, // proposer
                Vec::new(), // failed_proposers
                timestamp,
            );
            let event = ContractEvent::new_v1(
                new_block_event_key(),
                seq,
                TypeTag::Struct(Box::new(NewBlockEvent::struct_tag())),
                bcs::to_bytes(&new_block_event).unwrap(),
            );
            seq += 1;
            (version, event)
        }).collect()
    }
}

fn test_get_last_version_before_timestamp_impl(new_block_events: Vec<(Version, ContractEvent)>) {
    let tmp_dir = TempPath::new();
    let db = AptosDB::new_for_test(&tmp_dir);
    let store = &db.event_store;
    let event_db = &db.ledger_db.event_db();
    // error on no blocks
    assert!(store.get_last_version_before_timestamp(1000, 2000).is_err());

    // save events to db
    let mut batch = SchemaBatch::new();
    new_block_events.iter().for_each(|(ver, event)| {
        event_db
            .put_events(
                *ver,
                &[event.clone()],
                /*skip_index=*/ false,
                &mut batch,
            )
            .unwrap();
    });
    event_db.write_schemas(batch);

    let ledger_version = new_block_events.last().unwrap().0;

    // error on no block before timestamp
    let (first_block_version, first_event) = new_block_events.first().unwrap();
    let first_new_block_event: NewBlockEvent = first_event.try_into().unwrap();
    let first_block_ts = first_new_block_event.proposed_time();
    assert!(store
        .get_last_version_before_timestamp(1000, *first_block_version)
        .is_err());
    assert!(store
        .get_last_version_before_timestamp(first_block_ts, Version::max_value())
        .is_err());

    let mut last_block_ts = first_block_ts;
    let mut last_block_version = *first_block_version;
    for (version, event) in new_block_events.iter().skip(1) {
        let new_block_event: NewBlockEvent = event.try_into().unwrap();
        let ts = new_block_event.proposed_time();
        if ts == last_block_ts {
            // skip NIL blocks
            continue;
        }
        assert_eq!(
            store
                .get_last_version_before_timestamp((last_block_ts + ts + 1) / 2, ledger_version)
                .unwrap(),
            version - 1,
        );
        assert_eq!(
            store
                .get_last_version_before_timestamp(ts, ledger_version)
                .unwrap(),
            version - 1,
        );

        last_block_version = *version;
        last_block_ts = ts;
    }

    // error on no block after required ts
    assert!(store
        .get_last_version_before_timestamp(last_block_ts + 1, ledger_version)
        .is_err());
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))]

    #[test]
    fn test_get_last_version_before_timestamp(new_block_events in arb_new_block_events()) {
        test_get_last_version_before_timestamp_impl(new_block_events)
    }
}
