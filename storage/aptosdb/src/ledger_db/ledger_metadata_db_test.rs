// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{ledger_db::ledger_metadata_db::LedgerMetadataDb, AptosDB};
use aptos_schemadb::batch::SchemaBatch;
use aptos_storage_interface::AptosDbError;
use aptos_temppath::TempPath;
use aptos_types::{
    account_address::AccountAddress,
    account_config::events::new_block::{new_block_event_key, NewBlockEvent},
    contract_event::ContractEvent,
    ledger_info::LedgerInfoWithSignatures,
    proptest_types::{AccountInfoUniverse, LedgerInfoWithSignaturesGen},
    state_store::state_storage_usage::StateStorageUsage,
    transaction::Version,
};
use move_core_types::{language_storage::TypeTag, move_resource::MoveStructType};
use proptest::{
    arbitrary::{any, any_with},
    collection::vec,
    prelude::{Strategy, *},
};
use std::path::Path;

fn arb_ledger_infos_with_sigs() -> impl Strategy<Value = Vec<LedgerInfoWithSignatures>> {
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

fn get_first_epoch(ledger_infos_with_sigs: &[LedgerInfoWithSignatures]) -> u64 {
    ledger_infos_with_sigs
        .first()
        .unwrap()
        .ledger_info()
        .epoch()
}

fn get_last_epoch(ledger_infos_with_sigs: &[LedgerInfoWithSignatures]) -> u64 {
    ledger_infos_with_sigs.last().unwrap().ledger_info().epoch()
}

fn get_last_version(ledger_infos_with_sigs: &[LedgerInfoWithSignatures]) -> Version {
    ledger_infos_with_sigs
        .last()
        .unwrap()
        .ledger_info()
        .version()
}

fn set_up(path: &impl AsRef<Path>, ledger_infos_with_sigs: &[LedgerInfoWithSignatures]) -> AptosDB {
    let db = AptosDB::new_for_test(path);
    let ledger_metadata_db = db.ledger_db.metadata_db();

    let mut batch = SchemaBatch::new();
    ledger_infos_with_sigs
        .iter()
        .map(|info| ledger_metadata_db.put_ledger_info(info, &mut batch))
        .collect::<Result<Vec<_>, AptosDbError>>()
        .unwrap();
    ledger_metadata_db.write_schemas(batch).unwrap();
    ledger_metadata_db.set_latest_ledger_info(ledger_infos_with_sigs.last().unwrap().clone());
    db
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(20))]

    #[test]
    fn test_get_latest_ledger_info(ledger_infos_with_sigs in arb_ledger_infos_with_sigs()) {
        let tmp_dir = TempPath::new();
        let db = set_up(&tmp_dir, &ledger_infos_with_sigs);

        prop_assert_eq!(db.ledger_db.metadata_db().get_latest_ledger_info().unwrap(), ledger_infos_with_sigs.last().unwrap().clone());

        let tmp_dir = TempPath::new();
        let db = AptosDB::new_for_test(&tmp_dir);
        prop_assert!(db.ledger_db.metadata_db().get_latest_ledger_info().is_err());
    }

    #[test]
    fn test_get_latest_ledger_info_in_epoch(ledger_infos_with_sigs in arb_ledger_infos_with_sigs()) {
        let tmp_dir = TempPath::new();
        let db = set_up(&tmp_dir, &ledger_infos_with_sigs);

        let last_epoch = get_last_epoch(&ledger_infos_with_sigs);

        let expected_last_ledger_info = ledger_infos_with_sigs.last().unwrap().clone();
        let actual_last_ledger_info = db.ledger_db.metadata_db().get_latest_ledger_info_in_epoch(last_epoch).unwrap();
        prop_assert_eq!(actual_last_ledger_info, expected_last_ledger_info);

        prop_assert!(db.ledger_db.metadata_db().get_latest_ledger_info_in_epoch(last_epoch + 1).is_err());
    }

    #[test]
    fn test_epoch_ending_ledger_infos_iter(
        (ledger_infos_with_sigs, start_epoch, end_epoch) in arb_ledger_infos_with_sigs()
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
        let tmp_dir = TempPath::new();
        let db = set_up(&tmp_dir, &ledger_infos_with_sigs);

        let actual = db
            .ledger_db
            .metadata_db()
            .get_epoch_ending_ledger_info_iter(start_epoch, end_epoch)
            .unwrap()
            .collect::<Result<Vec<_>, AptosDbError>>()
            .unwrap();

        let expected: Vec<_> = ledger_infos_with_sigs
            .into_iter()
            .filter(|ledger_info_with_sigs| {
                let li = ledger_info_with_sigs.ledger_info();
                start_epoch <= li.epoch()
                    && li.epoch() < end_epoch
                    && li.next_epoch_state().is_some()
            }).collect();
        prop_assert_eq!(actual, expected);
    }

    #[test]
    fn test_get_epoch_state(ledger_infos_with_sigs in arb_ledger_infos_with_sigs()) {
        let tmp_dir = TempPath::new();
        let db = set_up(&tmp_dir, &ledger_infos_with_sigs);
        let ledger_metadata_db = db.ledger_db.metadata_db();

        assert!(ledger_metadata_db.get_epoch_state(0).is_err());

        for li_with_sigs in ledger_infos_with_sigs {
            let li = li_with_sigs.ledger_info();
            if li.next_epoch_state().is_some() {
                assert_eq!(
                    ledger_metadata_db.get_epoch_state(li.epoch()+1).unwrap(),
                    *li.next_epoch_state().unwrap(),
                );
            }

        }
    }

    #[test]
    fn test_get_epoch_ending_ledger_info(ledger_infos_with_sigs in arb_ledger_infos_with_sigs()) {
        let tmp_dir = TempPath::new();
        let db = set_up(&tmp_dir, &ledger_infos_with_sigs);

        let last_version = get_last_version(&ledger_infos_with_sigs);

        for ledger_info_with_sigs in ledger_infos_with_sigs {
            if ledger_info_with_sigs.ledger_info().ends_epoch() {
                let version = ledger_info_with_sigs.commit_info().version();
                let result = db.ledger_db.metadata_db().get_epoch_ending_ledger_info(version).unwrap();
                prop_assert_eq!(result, ledger_info_with_sigs);
            }
        }

        prop_assert!(db.ledger_db.metadata_db().get_epoch_ending_ledger_info(last_version + 1).is_err());
    }

    #[test]
    fn test_get_epoch(
        (ledger_infos_with_sigs, version) in arb_ledger_infos_with_sigs()
            .prop_flat_map(|ledger_infos_with_sigs| {
                let last_version = get_last_version(&ledger_infos_with_sigs);
                (
                    Just(ledger_infos_with_sigs),
                    0..=last_version,
                )
            })
    ) {
        let tmp_dir = TempPath::new();
        let db = set_up(&tmp_dir, &ledger_infos_with_sigs);

        let actual = db.ledger_db.metadata_db().get_epoch(version).unwrap();
        // Find the first LI that is at or after version.
        let index = ledger_infos_with_sigs
            .iter()
            .position(|x| x.ledger_info().version() >= version)
            .unwrap();
        let expected = ledger_infos_with_sigs[index].ledger_info().epoch();
        prop_assert_eq!(actual, expected);
    }

    #[test]
    fn test_get_previous_epoch_ending(ledger_infos_with_sigs in arb_ledger_infos_with_sigs()) {
        let tmp_dir = TempPath::new();
        let db = set_up(&tmp_dir, &ledger_infos_with_sigs);
        let ledger_metadata_db = db.ledger_db.metadata_db();

        let last_version = get_last_version(&ledger_infos_with_sigs);
        let last_epoch = get_last_epoch(&ledger_infos_with_sigs);

        let result = ledger_metadata_db.get_previous_epoch_ending(last_version).unwrap();
        if ledger_infos_with_sigs.len() < 2 {
            prop_assert_eq!(result, None);
        } else {
            let (version, epoch) = result.unwrap();
            prop_assert_eq!(epoch, last_epoch - 1);
            prop_assert_eq!(version, ledger_metadata_db.get_latest_ledger_info_in_epoch(epoch).unwrap().commit_info().version());
        }
    }
}

#[test]
fn test_block_api() {
    let tmp_dir = TempPath::new();
    let db = AptosDB::new_for_test(&tmp_dir);
    let ledger_metadata_db = db.ledger_db.metadata_db();

    let mut batch = SchemaBatch::new();
    let proposer_1 = AccountAddress::random();
    let proposer_2 = AccountAddress::random();
    let events = vec![
        NewBlockEvent::new(
            AccountAddress::random(),
            0,
            1,
            1,
            vec![],
            proposer_1,
            vec![],
            1000,
        ),
        NewBlockEvent::new(
            AccountAddress::random(),
            0,
            2,
            2,
            vec![],
            proposer_2,
            vec![],
            2000,
        ),
    ];
    LedgerMetadataDb::put_block_info(
        1,
        &ContractEvent::new_v1(
            new_block_event_key(),
            0,
            TypeTag::from(NewBlockEvent::struct_tag()),
            bcs::to_bytes(&events[0]).unwrap(),
        )
        .expect("Should always be able to create a new block event"),
        &mut batch,
    )
    .unwrap();
    LedgerMetadataDb::put_block_info(
        10,
        &ContractEvent::new_v1(
            new_block_event_key(),
            1,
            TypeTag::from(NewBlockEvent::struct_tag()),
            bcs::to_bytes(&events[1]).unwrap(),
        )
        .expect("Should always be able to create a new block event"),
        &mut batch,
    )
    .unwrap();
    ledger_metadata_db.write_schemas(batch).unwrap();

    assert_eq!(ledger_metadata_db.get_block_info(0).unwrap(), None);

    let block_info_1 = ledger_metadata_db.get_block_info(1).unwrap().unwrap();
    assert_eq!(block_info_1.id(), events[0].hash().unwrap());
    assert_eq!(block_info_1.epoch(), 0);
    assert_eq!(block_info_1.round(), 1);
    assert_eq!(block_info_1.proposer(), proposer_1);
    assert_eq!(block_info_1.timestamp_usecs(), 1000);

    let block_info_2 = ledger_metadata_db.get_block_info(2).unwrap().unwrap();
    assert_eq!(block_info_2.id(), events[1].hash().unwrap());
    assert_eq!(block_info_2.epoch(), 0);
    assert_eq!(block_info_2.round(), 2);
    assert_eq!(block_info_2.proposer(), proposer_2);
    assert_eq!(block_info_2.timestamp_usecs(), 2000);

    assert_eq!(ledger_metadata_db.get_block_info(3).unwrap(), None);

    assert!(ledger_metadata_db.get_block_height_by_version(0).is_err());
    for version in 1..10 {
        assert_eq!(
            ledger_metadata_db
                .get_block_height_by_version(version)
                .unwrap(),
            1
        );
    }
    for version in 10..20 {
        assert_eq!(
            ledger_metadata_db
                .get_block_height_by_version(version)
                .unwrap(),
            2
        );
    }
}

#[test]
fn test_usage() {
    let tmp_dir = TempPath::new();
    let db = AptosDB::new_for_test(&tmp_dir);
    let ledger_metadata_db = db.ledger_db.metadata_db();

    let usage = StateStorageUsage::new(7, 23);
    ledger_metadata_db.put_usage(1, usage).unwrap();
    assert_eq!(ledger_metadata_db.get_usage(1).unwrap(), usage);
    assert!(ledger_metadata_db.get_usage(0).is_err());
}
