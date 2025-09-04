// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::db::VelorDB;
use velor_schemadb::batch::SchemaBatch;
use velor_storage_interface::Result;
use velor_temppath::TempPath;
use velor_types::contract_event::ContractEvent;
use proptest::{collection::vec, prelude::*, proptest};

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))]

    #[test]
    fn test_put_get(events in vec(any::<ContractEvent>().no_shrink(), 1..100)) {
        let tmp_dir = TempPath::new();
        let db = VelorDB::new_for_test(&tmp_dir);
        let event_db = &db.ledger_db.event_db();

        prop_assert_eq!(event_db.latest_version().unwrap(), None);

        let mut batch = SchemaBatch::new();
        event_db.put_events(100, &events, /*skip_index=*/false, &mut batch).unwrap();
        event_db.write_schemas(batch).unwrap();

        prop_assert_eq!(event_db.latest_version().unwrap(), Some(100));

        let events_100 = event_db.get_events_by_version(100).unwrap();
        prop_assert_eq!(events_100, events);
    }

    #[test]
    fn test_put_get_batch(
        events1 in vec(any::<ContractEvent>().no_shrink(), 1..100),
        events2 in vec(any::<ContractEvent>().no_shrink(), 1..100),
        events3 in vec(any::<ContractEvent>().no_shrink(), 1..100),
    ) {

        let tmp_dir = TempPath::new();
        let db = VelorDB::new_for_test(&tmp_dir);
        let event_db = &db.ledger_db.event_db();
        let mut batch = SchemaBatch::new();
        event_db.put_events_multiple_versions(99, &[events1.clone(), events2.clone(), events3.clone()], &mut batch).unwrap();
        event_db.write_schemas(batch).unwrap();

        let events_99 = event_db.get_events_by_version(99).unwrap();
        prop_assert_eq!(events_99, events1.clone());

        let events_100 = event_db.get_events_by_version(100).unwrap();
        prop_assert_eq!(events_100, events2.clone());

        let events_101 = event_db.get_events_by_version(101).unwrap();
        prop_assert_eq!(events_101, events3.clone());

        let events_102 = event_db.get_events_by_version(102).unwrap();
        prop_assert_eq!(events_102.len(), 0);

        prop_assert_eq!(
            event_db
            .get_events_by_version_iter(99, 3)
            .unwrap()
            .collect::<Result<Vec<_>>>()
            .unwrap(),
            vec![events1, events2, events3]
        );
    }
}
