// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::types::{Incarnation, TxnIndex};
use aptos_infallible::Mutex;
use aptos_types::error::{code_invariant_error, PanicError};
use std::collections::{btree_map::Entry, BTreeMap};

// Checks the invariant that the lowest dependency is strictly greater than
// provided txn_idx. This is a sanity check e.g. for dependencies stored at
// an entry at txn_idx in the multi-versioned data structure.
pub(crate) fn check_lowest_dependency_idx(
    dependencies: &BTreeMap<TxnIndex, Incarnation>,
    txn_idx: TxnIndex,
) -> Result<(), PanicError> {
    if let Some((lowest_dep_idx, _)) = dependencies.first_key_value() {
        if *lowest_dep_idx <= txn_idx {
            return Err(code_invariant_error(format!(
                "Dependency for txn {} recorded at idx {}",
                *lowest_dep_idx, txn_idx
            )));
        }
    }
    Ok(())
}

// A wrapper around recorded read dependencies used in multi-versioned data structures.
// Does not expose the inner field to avoid bugs, such as merging different sets of
// dependencies, when it is important to keep the latest incarnation of each txn.
#[derive(Debug)]
pub(crate) struct RegisteredReadDependencies {
    /// A map of txn_idx to incarnation that have registered a read of this entry.
    /// The reason for using the map is to store at most one (latest) incarnation
    /// per txn_idx (since a dependency on an outdated incarnation can safely be removed).
    // TODO(BlockSTMv2): Add support for behavioral validation (read kind).
    dependencies: BTreeMap<TxnIndex, Incarnation>,
}

impl RegisteredReadDependencies {
    pub(crate) fn new() -> Self {
        Self {
            dependencies: BTreeMap::new(),
        }
    }

    pub(crate) fn from_dependencies(dependencies: BTreeMap<TxnIndex, Incarnation>) -> Self {
        Self { dependencies }
    }

    // Returns a PanicError if the incarnation is lower than the previous incarnation,
    // as an invariant that caller monotonically increases the incarnation is assumed.
    pub(crate) fn insert(
        &mut self,
        txn_idx: TxnIndex,
        incarnation: Incarnation,
    ) -> Result<(), PanicError> {
        if let Some(prev_incarnation) = self.dependencies.insert(txn_idx, incarnation) {
            if prev_incarnation > incarnation {
                // A higher incarnation may not have been recorded before, as
                // incarnations for each txn index are monotonically incremented.
                //
                // TODO(BlockSTMv2): Consider also checking the cases when the
                // incarnations are equal, but local caching should have ensured that the
                // read with the same incarnation was not performed twice.
                return Err(code_invariant_error(format!(
                    "Recording dependency on txn {} incarnation {}, found incarnation {}",
                    txn_idx, incarnation, prev_incarnation
                )));
            }
        }

        Ok(())
    }

    fn extend_impl(
        self_dependencies: &mut BTreeMap<TxnIndex, Incarnation>,
        other_dependencies: BTreeMap<TxnIndex, Incarnation>,
    ) {
        for (txn_idx, incarnation) in other_dependencies {
            match self_dependencies.entry(txn_idx) {
                Entry::Occupied(mut entry) => {
                    if *entry.get() < incarnation {
                        entry.insert(incarnation);
                    }
                },
                Entry::Vacant(entry) => {
                    entry.insert(incarnation);
                },
            }
        }
    }

    // When we extend recorded dependencies with other dependencies in a general sense
    // (e.g. these might be invalidated dependencies from different data-structures),
    // we need to make sure to keep the latest incarnation per txn index.
    pub(crate) fn extend(&mut self, other: BTreeMap<TxnIndex, Incarnation>) {
        Self::extend_impl(&mut self.dependencies, other);
    }

    // This method merges other dependencies, but expects that it contains strictly
    // larger txn indices. This invariant holds when removing an entry from a data-structure,
    // and migrating dependencies (that still pass validation) to a different entry.
    // The index of the entry acts as a separator between the indices in both sets.
    pub(crate) fn extend_with_higher_dependencies(
        &mut self,
        other: BTreeMap<TxnIndex, Incarnation>,
    ) -> Result<(), PanicError> {
        let dependencies = &mut self.dependencies;
        if let Some((highest_dep_idx, _)) = dependencies.last_key_value() {
            // Highest dependency in self should be strictly less than other dependencies.
            check_lowest_dependency_idx(&other, *highest_dep_idx)?;
        }

        Self::extend_impl(dependencies, other);

        Ok(())
    }

    // Split off dependencies above (and including) txn_idx and return as BTreeMap.
    pub(crate) fn split_off(&mut self, txn_idx: TxnIndex) -> BTreeMap<TxnIndex, Incarnation> {
        self.dependencies.split_off(&txn_idx)
    }

    pub(crate) fn take(self) -> BTreeMap<TxnIndex, Incarnation> {
        self.dependencies
    }

    #[cfg(test)]
    pub(crate) fn clone_dependencies_for_test(&self) -> BTreeMap<TxnIndex, Incarnation> {
        self.dependencies.clone()
    }
}

pub(crate) fn take_dependencies(
    dependencies_in_mutex: &Mutex<RegisteredReadDependencies>,
) -> BTreeMap<TxnIndex, Incarnation> {
    std::mem::take(&mut dependencies_in_mutex.lock().dependencies)
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::{assert_err, assert_ok};

    #[test]
    fn test_check_lowest_dependency_idx() {
        let mut deps = BTreeMap::new();
        // Ok on empty.
        assert_ok!(check_lowest_dependency_idx(&deps, 5));

        deps.insert(10, 1);
        deps.insert(12, 1);

        // Ok when lowest dependency (10) > txn_idx (5).
        assert_ok!(check_lowest_dependency_idx(&deps, 5));
        // Ok when lowest dependency (10) > txn_idx (9).
        assert_ok!(check_lowest_dependency_idx(&deps, 9));

        // Err when lowest dependency (10) == txn_idx (10).
        assert_err!(check_lowest_dependency_idx(&deps, 10));
        // Err when lowest dependency (10) < txn_idx (11).
        assert_err!(check_lowest_dependency_idx(&deps, 11));
    }

    #[test]
    fn test_dependencies_construction_and_insertion() {
        // Test `new` and `from_dependencies`.
        let deps = RegisteredReadDependencies::new();
        assert!(deps.dependencies.is_empty());

        let initial_map = BTreeMap::from([(10, 1), (20, 2)]);
        let mut deps = RegisteredReadDependencies::from_dependencies(initial_map.clone());
        assert_eq!(deps.dependencies, initial_map);

        // Test `insert`.
        // Insert new dependency.
        assert_ok!(deps.insert(30, 3));
        assert_eq!(deps.dependencies.get(&30), Some(&3));

        // Insert with higher incarnation for existing dependency.
        assert_ok!(deps.insert(20, 5));
        assert_eq!(deps.dependencies.get(&20), Some(&5));

        // Insert with the same incarnation for existing dependency
        // (should not error for the time being).
        assert_ok!(deps.insert(20, 5));
        assert_eq!(deps.dependencies.get(&20), Some(&5));

        // Insert with lower incarnation for existing dependency (must error).
        assert_err!(deps.insert(20, 4));
    }

    #[test]
    fn test_extend_and_extend_with_higher() {
        let mut deps = RegisteredReadDependencies::from_dependencies(BTreeMap::from([
            (10, 5),
            (11, 3),
            (20, 5),
        ]));

        // Test `extend`.
        let other1 = BTreeMap::from([(11, 4), (15, 6), (20, 3), (25, 6)]);
        deps.extend(other1);
        let final_map = deps.clone_dependencies_for_test();
        assert_eq!(final_map.get(&10), Some(&5)); // Unchanged.
        assert_eq!(final_map.get(&11), Some(&4)); // Took higher incarnation from other1.
        assert_eq!(final_map.get(&15), Some(&6)); // New.
        assert_eq!(final_map.get(&20), Some(&5)); // Kept higher incarnation.
        assert_eq!(final_map.get(&25), Some(&6)); // New.
        assert_eq!(final_map.len(), 5);

        // Test `extend_with_higher_dependencies`.
        // Success case.
        let other2 = BTreeMap::from([(30, 1), (40, 1)]);
        assert_ok!(deps.extend_with_higher_dependencies(other2));
        assert_eq!(deps.clone_dependencies_for_test().len(), 7);

        // Failure case: overlapping index.
        let other3 = BTreeMap::from([(40, 2), (50, 2)]);
        assert_err!(deps.extend_with_higher_dependencies(other3));

        // Failure case: lower index.
        let other4 = BTreeMap::from([(35, 2)]);
        assert_err!(deps.extend_with_higher_dependencies(other4));
    }

    #[test]
    fn test_split_off() {
        let mut deps = RegisteredReadDependencies::from_dependencies(BTreeMap::from([
            (10, 1),
            (20, 1),
            (30, 1),
            (40, 1),
            (50, 1),
        ]));

        // Split at an existing key.
        let split_map = deps.split_off(30);
        assert_eq!(
            deps.clone_dependencies_for_test(),
            BTreeMap::from([(10, 1), (20, 1)])
        );
        assert_eq!(split_map, BTreeMap::from([(30, 1), (40, 1), (50, 1)]));

        // Split at a non-existing key.
        let split_map_2 = deps.split_off(15);
        assert_eq!(
            deps.clone_dependencies_for_test(),
            BTreeMap::from([(10, 1)])
        );
        assert_eq!(split_map_2, BTreeMap::from([(20, 1)]));

        // Split everything off.
        let split_map_3 = deps.split_off(0);
        assert!(deps.clone_dependencies_for_test().is_empty());
        assert_eq!(split_map_3, BTreeMap::from([(10, 1)]));
    }

    #[test]
    fn test_take_and_clone() {
        let initial_map = BTreeMap::from([(10, 1), (20, 2)]);
        let deps = RegisteredReadDependencies::from_dependencies(initial_map.clone());

        // Test clone.
        let cloned_deps = deps.clone_dependencies_for_test();
        assert_eq!(cloned_deps, initial_map);
        // Original dependencies should be untouched.
        assert_eq!(deps.dependencies, initial_map);

        // Test take.
        let deps = Mutex::new(deps);
        let taken_deps = take_dependencies(&deps);
        assert_eq!(taken_deps, initial_map);
        // Original dependencies should now be empty.
        assert!(deps.lock().dependencies.is_empty());
    }
}
