// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    epoch_by_version::EpochByVersionSchema, ledger_store::ledger_info_test_utils::*,
    pruner::PrunerIndex, Pruner,
};
use aptos_config::config::StoragePrunerConfig;
use aptos_temppath::TempPath;
use aptos_types::ledger_info::LedgerInfoWithSignatures;
use proptest::prelude::*;
use std::{collections::BTreeMap, sync::Arc};

use crate::schema::ledger_info::LedgerInfoSchema;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))]

    #[test]
    fn test_ledger_store_pruner(
        (ledger_infos_with_sigs, _start_epoch, _end_epoch) in arb_ledger_infos_with_sigs()
            .prop_flat_map(|ledger_infos_with_sigs| {
                let first_epoch = get_first_epoch(&ledger_infos_with_sigs);
                let last_epoch = get_last_epoch(&ledger_infos_with_sigs);
                (
                    Just(ledger_infos_with_sigs),
                    first_epoch..=last_epoch,
                )
            })
            .prop_flat_map(|(ledger_infos_with_sigs, start_epoch)| {
                let last_epoch = get_last_epoch(&ledger_infos_with_sigs);
                (
                    Just(ledger_infos_with_sigs),
                    Just(start_epoch),
                    (start_epoch..=last_epoch),
                )
            })
    ) {
        verify_ledger_store_pruner(ledger_infos_with_sigs)
    }
}

fn verify_ledger_store_pruner(ledger_info_with_sigs: Vec<LedgerInfoWithSignatures>) {
    let tmp_dir = TempPath::new();
    let aptos_db = set_up(&tmp_dir, &ledger_info_with_sigs);
    let pruner = Pruner::new(
        Arc::clone(&aptos_db.db),
        StoragePrunerConfig {
            state_store_prune_window: Some(0),
            default_prune_window: Some(0),
        },
        Arc::clone(&aptos_db.transaction_store),
        Arc::clone(&aptos_db.ledger_store),
        Arc::clone(&aptos_db.event_store),
    );
    // Get the vector of epoch to the latest version in the epoch
    let epoch_to_latest_version: BTreeMap<u64, u64> = ledger_info_with_sigs
        .iter()
        .map(|x| (x.ledger_info().epoch(), x.ledger_info().version()))
        .collect();

    for (epoch, version) in &epoch_to_latest_version {
        pruner
            .wake_and_wait(*version, PrunerIndex::EpochStorePrunerIndex as usize)
            .unwrap();

        for (inner_epoch, inner_version) in &epoch_to_latest_version {
            // LedgerInfoSchema is not pruned, so we can read the ledger info schema and epoch by version
            // from the DB.
            assert!(aptos_db
                .db
                .get::<LedgerInfoSchema>(inner_epoch)
                .unwrap()
                .is_some());
            if inner_epoch < epoch {
                assert!(aptos_db
                    .db
                    .get::<EpochByVersionSchema>(inner_version)
                    .unwrap()
                    .is_none())
            } else if *inner_epoch < &(epoch_to_latest_version.len() as u64) - 1 {
                // Check for epoch by version exists for all epoch except for the last one. For last
                // one this is not valid as the next epoch is not defined.
                assert!(aptos_db
                    .db
                    .get::<EpochByVersionSchema>(inner_version)
                    .unwrap()
                    .is_some())
            }
        }
    }
}
