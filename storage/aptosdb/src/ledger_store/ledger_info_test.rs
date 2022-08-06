// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::*;
use aptos_temppath::TempPath;
use ledger_info_test_utils::*;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(20))]

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
            .ledger_store
            .get_epoch_ending_ledger_info_iter(start_epoch, end_epoch)
            .unwrap()
            .collect::<Result<Vec<_>>>()
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

        let actual = db.ledger_store.get_epoch(version).unwrap();
        // Find the first LI that is at or after version.
        let index = ledger_infos_with_sigs
            .iter()
            .position(|x| x.ledger_info().version() >= version)
            .unwrap();
        let expected = ledger_infos_with_sigs[index].ledger_info().epoch();
        prop_assert_eq!(actual, expected);
    }

    #[test]
    fn test_get_epoch_state(ledger_infos_with_sigs in arb_ledger_infos_with_sigs()) {
        let tmp_dir = TempPath::new();
        let db = set_up(&tmp_dir, &ledger_infos_with_sigs);

        assert!(db.ledger_store.get_epoch_state(0).is_err());

        for li_with_sigs in ledger_infos_with_sigs {
            let li = li_with_sigs.ledger_info();
            if li.next_epoch_state().is_some() {
                assert_eq!(
                    db.ledger_store.get_epoch_state(li.epoch()+1).unwrap(),
                    *li.next_epoch_state().unwrap(),
                );
            }

        }
    }
}
