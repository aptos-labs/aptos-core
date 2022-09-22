// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
use crate::AptosDB;
use anyhow::Result;
use aptos_types::{
    ledger_info::LedgerInfoWithSignatures,
    proptest_types::{AccountInfoUniverse, LedgerInfoWithSignaturesGen},
    transaction::Version,
};
use proptest::{
    arbitrary::{any, any_with},
    collection::vec,
    prelude::Strategy,
};
use schemadb::SchemaBatch;
use std::path::Path;

pub fn arb_ledger_infos_with_sigs() -> impl Strategy<Value = Vec<LedgerInfoWithSignatures>> {
    (
        any_with::<AccountInfoUniverse>(3),
        vec((any::<LedgerInfoWithSignaturesGen>(), 1..50usize), 1..50),
    )
        .prop_map(|(mut universe, gens)| {
            let ledger_infos_with_sigs: Vec<_> = gens
                .into_iter()
                .map(|(ledger_info_gen, block_size)| {
                    ledger_info_gen.materialize(&mut universe, block_size)
                })
                .collect();
            assert_eq!(get_first_epoch(&ledger_infos_with_sigs), 0);
            ledger_infos_with_sigs
        })
}

pub fn get_first_epoch(ledger_infos_with_sigs: &[LedgerInfoWithSignatures]) -> u64 {
    ledger_infos_with_sigs
        .first()
        .unwrap()
        .ledger_info()
        .epoch()
}

pub fn get_last_epoch(ledger_infos_with_sigs: &[LedgerInfoWithSignatures]) -> u64 {
    ledger_infos_with_sigs.last().unwrap().ledger_info().epoch()
}

pub fn get_last_version(ledger_infos_with_sigs: &[LedgerInfoWithSignatures]) -> Version {
    ledger_infos_with_sigs
        .last()
        .unwrap()
        .ledger_info()
        .version()
}

pub fn set_up(
    path: &impl AsRef<Path>,
    ledger_infos_with_sigs: &[LedgerInfoWithSignatures],
) -> AptosDB {
    let db = AptosDB::new_for_test(path);
    let store = &db.ledger_store;

    // Write LIs to DB.
    let mut batch = SchemaBatch::new();
    ledger_infos_with_sigs
        .iter()
        .map(|info| store.put_ledger_info(info, &mut batch))
        .collect::<Result<Vec<_>>>()
        .unwrap();
    store.db.write_schemas(batch).unwrap();
    store.set_latest_ledger_info(ledger_infos_with_sigs.last().unwrap().clone());
    db
}
