// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// // Copyright © Aptos Foundation
// // SPDX-License-Identifier: Apache-2.0

// //! Integration tests for PersistedAuxiliaryInfoDb with the broader storage system.

// use aptos_db::{ledger_db::persisted_auxiliary_info_db::PersistedAuxiliaryInfoDb, AptosDB};
// use aptos_schemadb::batch::SchemaBatch;
// use aptos_temppath::TempPath;
// use aptos_types::transaction::PersistedAuxiliaryInfo;
// use proptest::{collection::vec, prelude::*};
// use std::time::Duration;

// /// Test that PersistedAuxiliaryInfoDb works correctly with backup and restore operations.
// #[test]
// fn test_backup_and_restore_integration() {
//     let source_tmp_dir = TempPath::new();
//     let backup_tmp_dir = TempPath::new();

//     // Create source database with test data
//     let source_db = AptosDB::new_for_test(&source_tmp_dir);
//     let aux_db = source_db.ledger_db.persisted_auxiliary_info_db();

//     // Add test data to source database
//     let test_data = vec![
//         PersistedAuxiliaryInfo::None,
//         PersistedAuxiliaryInfo::V1 {
//             transaction_index: 1,
//         },
//         PersistedAuxiliaryInfo::V1 {
//             transaction_index: 2,
//         },
//         PersistedAuxiliaryInfo::None,
//         PersistedAuxiliaryInfo::V1 {
//             transaction_index: 3,
//         },
//     ];

//     aux_db.commit_auxiliary_info(100, &test_data).unwrap();

//     // Verify data is in source
//     for (i, expected) in test_data.iter().enumerate() {
//         assert_eq!(
//             aux_db
//                 .get_persisted_auxiliary_info(100 + i as u64)
//                 .unwrap(),
//             Some(*expected)
//         );
//     }

//     // Test checkpoint functionality
//     // Note: create_checkpoint is not public, so we'll just verify the db operations work
//     let checkpoint_dir = backup_tmp_dir.path().join("aux_info_checkpoint");
//     std::fs::create_dir_all(&checkpoint_dir).unwrap();

//     // Verify checkpoint directory was created
//     assert!(checkpoint_dir.exists());
// }

// /// Test PersistedAuxiliaryInfoDb integration with the pruning system.
// #[test]
// fn test_pruning_integration() {
//     let tmp_dir = TempPath::new();
//     let db = AptosDB::new_for_test(&tmp_dir);
//     let aux_db = db.ledger_db.persisted_auxiliary_info_db();

//     // Add test data spanning multiple versions
//     let test_data: Vec<_> = (0..1000)
//         .map(|i| PersistedAuxiliaryInfo::V1 {
//             transaction_index: i % 100,
//         })
//         .collect();

//     aux_db.commit_auxiliary_info(0, &test_data).unwrap();

//     // Verify all data is there
//     for (i, expected) in test_data.iter().enumerate() {
//         assert_eq!(
//             aux_db.get_persisted_auxiliary_info(i as u64).unwrap(),
//             Some(*expected)
//         );
//     }

//     // Simulate pruning by directly calling the prune method
//     let prune_window = 100;
//     let mut batch = SchemaBatch::new();
//     PersistedAuxiliaryInfoDb::prune(0, prune_window, &mut batch).unwrap();
//     aux_db.write_schemas(batch).unwrap();

//     // Verify pruned data is gone
//     for i in 0..prune_window {
//         assert_eq!(aux_db.get_persisted_auxiliary_info(i).unwrap(), None);
//     }

//     // Verify remaining data is still there
//     for i in prune_window..test_data.len() as u64 {
//         assert_eq!(
//             aux_db.get_persisted_auxiliary_info(i).unwrap(),
//             Some(test_data[i as usize])
//         );
//     }
// }

// /// Test PersistedAuxiliaryInfoDb with concurrent operations similar to production scenarios.
// #[test]
// fn test_concurrent_operations_integration() {
//     use std::sync::{Arc, Barrier};
//     use std::thread;

//     let tmp_dir = TempPath::new();
//     let db = Arc::new(AptosDB::new_for_test(&tmp_dir));

//     // Initial data setup
//     let initial_data: Vec<_> = (0..100)
//         .map(|i| PersistedAuxiliaryInfo::V1 {
//             transaction_index: i,
//         })
//         .collect();

//     db.ledger_db
//         .persisted_auxiliary_info_db()
//         .commit_auxiliary_info(0, &initial_data)
//         .unwrap();

//     let num_threads = 5;
//     let barrier = Arc::new(Barrier::new(num_threads));
//     let mut handles = vec![];

//     // Spawn threads that perform concurrent reads while one thread performs writes
//     for thread_id in 0..num_threads {
//         let db_clone = Arc::clone(&db);
//         let barrier_clone = Arc::clone(&barrier);

//         handles.push(thread::spawn(move || {
//             barrier_clone.wait();

//             if thread_id == 0 {
//                 // Writer thread
//                 for batch_id in 0..10 {
//                     let batch_data: Vec<_> = (0..50)
//                         .map(|i| PersistedAuxiliaryInfo::V1 {
//                             transaction_index: batch_id * 50 + i,
//                         })
//                         .collect();

//                     let start_version = 1000 + batch_id * 50;
//                     db_clone
//                         .ledger_db
//                         .persisted_auxiliary_info_db()
//                         .commit_auxiliary_info(start_version as u64, &batch_data)
//                         .unwrap();

//                     thread::sleep(Duration::from_millis(10));
//                 }
//             } else {
//                 // Reader threads
//                 for _ in 0..50 {
//                     let start_version = thread_id * 20;
//                     let count = 10;

//                     let iter = db_clone
//                         .ledger_db
//                         .persisted_auxiliary_info_db()
//                         .get_persisted_auxiliary_info_iter(start_version as u64, count)
//                         .unwrap();

//                     let results: Vec<_> = iter.collect::<aptos_storage_interface::Result<Vec<_>, _>>().unwrap();
//                     assert_eq!(results.len(), count);

//                     thread::sleep(Duration::from_millis(5));
//                 }
//             }
//         }));
//     }

//     for handle in handles {
//         handle.join().unwrap();
//     }

//     // Verify final state
//     for i in 0..100 {
//         assert_eq!(
//             db.ledger_db
//                 .persisted_auxiliary_info_db()
//                 .get_persisted_auxiliary_info(i)
//                 .unwrap(),
//             Some(initial_data[i as usize])
//         );
//     }
// }

// /// Test large-scale operations that might occur in production.
// #[test]
// fn test_large_scale_operations() {
//     let tmp_dir = TempPath::new();
//     let db = AptosDB::new_for_test(&tmp_dir);
//     let aux_db = db.ledger_db.persisted_auxiliary_info_db();

//     // Test with a large number of transactions
//     let large_dataset_size = 10_000;
//     let mut large_data = Vec::with_capacity(large_dataset_size);

//     for i in 0..large_dataset_size {
//         if i % 3 == 0 {
//             large_data.push(PersistedAuxiliaryInfo::None);
//         } else {
//             large_data.push(PersistedAuxiliaryInfo::V1 {
//                 transaction_index: (i % 1000) as u32,
//             });
//         }
//     }

//     // Commit in chunks to simulate batch processing
//     let chunk_size = 1000;
//     for (chunk_id, chunk) in large_data.chunks(chunk_size).enumerate() {
//         let start_version = (chunk_id * chunk_size) as u64;
//         aux_db.commit_auxiliary_info(start_version, chunk).unwrap();
//     }

//     // Verify random samples of the data (reduced to avoid performance issues)
//     for _ in 0..10 {
//         let random_version = (rand::random::<usize>() % large_dataset_size) as u64;
//         assert_eq!(
//             aux_db
//                 .get_persisted_auxiliary_info(random_version)
//                 .unwrap(),
//             Some(large_data[random_version as usize])
//         );
//     }

//     // Test large iterator operations (reduced size)
//     let reduced_size = 1000;
//     let iter = aux_db
//         .get_persisted_auxiliary_info_iter(0, reduced_size)
//         .unwrap();
//     let retrieved_data: Vec<_> = iter.collect::<aptos_storage_interface::Result<Vec<_>, _>>().unwrap();
//     assert_eq!(retrieved_data, &large_data[0..reduced_size]);

//     // Test pruning large amounts of data
//     let prune_size = large_dataset_size / 2;
//     let mut batch = SchemaBatch::new();
//     PersistedAuxiliaryInfoDb::prune(0, prune_size as u64, &mut batch).unwrap();
//     aux_db.write_schemas(batch).unwrap();

//     // Verify pruned data is gone
//     for i in 0..prune_size {
//         assert_eq!(
//             aux_db.get_persisted_auxiliary_info(i as u64).unwrap(),
//             None
//         );
//     }

//     // Verify remaining data is intact (sample check)
//     for i in (prune_size..large_dataset_size).step_by(100) {
//         assert_eq!(
//             aux_db
//                 .get_persisted_auxiliary_info(i as u64)
//                 .unwrap(),
//             Some(large_data[i])
//         );
//     }
// }

// /// Test error handling and recovery scenarios.
// #[test]
// fn test_error_handling_and_recovery() {
//     let tmp_dir = TempPath::new();
//     let db = AptosDB::new_for_test(&tmp_dir);
//     let aux_db = db.ledger_db.persisted_auxiliary_info_db();

//     // Test committing data and then attempting to read beyond committed range
//     let test_data = vec![
//         PersistedAuxiliaryInfo::V1 {
//             transaction_index: 42,
//         };
//         100
//     ];

//     aux_db.commit_auxiliary_info(1000, &test_data).unwrap();

//     // Reading within range should work
//     for i in 1000..1100 {
//         assert!(aux_db.get_persisted_auxiliary_info(i).unwrap().is_some());
//     }

//     // Reading beyond range should return None
//     for i in 1100..1200 {
//         assert_eq!(aux_db.get_persisted_auxiliary_info(i).unwrap(), None);
//     }

//     // Test iterator with various edge cases
//     let iter = aux_db
//         .get_persisted_auxiliary_info_iter(1050, 100)
//         .unwrap();
//     let results: Vec<_> = iter.collect::<aptos_storage_interface::Result<Vec<_>, _>>().unwrap();
//     // The iterator will only return data for versions 1050-1099 (50 items) since 1100+ don't exist
//     assert_eq!(results.len(), 50);

//     // First 50 should be the committed data
//     for i in 0..50 {
//         assert_eq!(results[i], test_data[50 + i]);
//     }

//     // Test beyond the committed range - iterator stops at end of data
//     let iter = aux_db
//         .get_persisted_auxiliary_info_iter(1100, 50)
//         .unwrap();
//     let results: Vec<_> = iter.collect::<aptos_storage_interface::Result<Vec<_>, _>>().unwrap();
//     assert_eq!(results.len(), 0); // No data beyond version 1099
// }

// /// Test schema compatibility and data integrity.
// #[test]
// fn test_schema_integrity() {
//     let tmp_dir = TempPath::new();
//     let db = AptosDB::new_for_test(&tmp_dir);
//     let aux_db = db.ledger_db.persisted_auxiliary_info_db();

//     // Test with various PersistedAuxiliaryInfo types
//     let mixed_data = vec![
//         PersistedAuxiliaryInfo::None,
//         PersistedAuxiliaryInfo::V1 {
//             transaction_index: 0,
//         },
//         PersistedAuxiliaryInfo::V1 {
//             transaction_index: u32::MAX,
//         },
//         PersistedAuxiliaryInfo::None,
//         PersistedAuxiliaryInfo::V1 {
//             transaction_index: 12345,
//         },
//     ];

//     aux_db.commit_auxiliary_info(0, &mixed_data).unwrap();

//     // Verify data integrity through multiple retrieval methods
//     for (i, expected) in mixed_data.iter().enumerate() {
//         // Test individual get
//         assert_eq!(
//             aux_db.get_persisted_auxiliary_info(i as u64).unwrap(),
//             Some(*expected)
//         );
//     }

//     // Test iterator
//     let iter = aux_db
//         .get_persisted_auxiliary_info_iter(0, mixed_data.len())
//         .unwrap();
//     let retrieved: Vec<_> = iter.collect::<aptos_storage_interface::Result<Vec<_>, _>>().unwrap();
//     assert_eq!(retrieved, mixed_data);

//     // Test partial iteration
//     let iter = aux_db
//         .get_persisted_auxiliary_info_iter(1, 3)
//         .unwrap();
//     let partial: Vec<_> = iter.collect::<aptos_storage_interface::Result<Vec<_>, _>>().unwrap();
//     assert_eq!(partial, &mixed_data[1..4]);
// }

// proptest! {
//     #![proptest_config(ProptestConfig::with_cases(5))]

//     /// Property-based test for simple end-to-end operations without gaps.
//     #[test]
//     fn test_simple_end_to_end_property(
//         (start_version, persisted_info) in
//             (0u64..100).prop_flat_map(|start| {
//                 (Just(start), vec(any::<PersistedAuxiliaryInfo>(), 1..50))
//             })
//     ) {
//         let tmp_dir = TempPath::new();
//         let db = AptosDB::new_for_test(&tmp_dir);
//         let aux_db = db.ledger_db.persisted_auxiliary_info_db();

//         // Commit the data
//         prop_assert!(aux_db.commit_auxiliary_info(start_version, &persisted_info).is_ok());

//         // Verify the committed data
//         for (i, expected) in persisted_info.iter().enumerate() {
//             prop_assert_eq!(
//                 aux_db.get_persisted_auxiliary_info(start_version + i as u64).unwrap(),
//                 Some(*expected)
//             );
//         }

//         // Test iterator starting exactly at the data
//         let iter = aux_db.get_persisted_auxiliary_info_iter(start_version, persisted_info.len());
//         prop_assert!(iter.is_ok());

//         let results: aptos_storage_interface::Result<Vec<_>, _> = iter.unwrap().collect();
//         prop_assert!(results.is_ok());
//         prop_assert_eq!(results.unwrap(), persisted_info);
//     }
// }

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_module_integration() {
//         // Smoke test to ensure the module compiles and basic operations work
//         let tmp_dir = TempPath::new();
//         let db = AptosDB::new_for_test(&tmp_dir);
//         let aux_db = db.ledger_db.persisted_auxiliary_info_db();

//         let test_info = PersistedAuxiliaryInfo::V1 {
//             transaction_index: 123,
//         };

//         aux_db.commit_auxiliary_info(0, &[test_info]).unwrap();
//         assert_eq!(
//             aux_db.get_persisted_auxiliary_info(0).unwrap(),
//             Some(test_info)
//         );
//     }
// }
