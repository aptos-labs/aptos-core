// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Implements versioning data structure to support linear history.

use move_binary_format::errors::PartialVMResult;
use move_vm_types::values::Copyable;
use smallvec::{smallvec, SmallVec};
use std::cmp::Ordering;

/// Implement version control for data structures where keys need to be versioned, and its state
/// can be saved or rolled back to previously saved state.
pub struct VersionController {
    /// Next version to use on save / undo. Monotonically increases and cannot be reused to prevent
    /// ABA problems.
    next_version: u32,
    /// Checkpoints for saved versions. The invariant is that there it always stores at least one
    /// (current) version.
    saved_versions: SmallVec<[u32; 3]>,
    /// Current version of the data structure.
    current_version: u32,
}

impl Default for VersionController {
    fn default() -> Self {
        Self::new()
    }
}

impl VersionController {
    /// Creates a new control structure with current empty state being saved.
    pub fn new() -> Self {
        Self {
            next_version: 1,
            saved_versions: smallvec![0],
            current_version: 0,
        }
    }

    /// Returns the current version.
    pub fn current_version(&self) -> u32 {
        self.current_version
    }

    /// Saves the current version as a checkpoint, incrementing the current version.
    pub fn save(&mut self) {
        self.current_version = self.next_version;
        self.saved_versions.push(self.current_version);
        self.next_version += 1;
    }

    /// Rollbacks the current version to previously saved version.
    pub fn undo(&mut self) {
        if self.saved_versions.len() > 1 {
            self.saved_versions.pop();
            self.current_version = *self
                .saved_versions
                .last()
                .expect("Saved version must exist");
        }
    }
}

#[cfg(test)]
impl VersionController {
    fn saved_versions(&self) -> Vec<u32> {
        self.saved_versions.iter().cloned().collect()
    }
}

/// Inner representation of each possible version of a value.
struct VersionedValue<V> {
    /// The actual value the entry contains. Copied on mutable access.
    value: V,
    /// The version when this entry was inserted (possible to rollback).
    version: u32,
    /// Incarnation of the value within a single version (not possible to rollback).
    incarnation: u32,
}

/// In practice, we expect a small number of versions (and hence, we do not worry about memory
/// consumption).
const EXPECTED_NUM_VERSIONS: usize = 3;

/// Contains different (monotonically increasing) versions of the value.
pub struct VersionedSlot<V: Copyable> {
    versions: SmallVec<[VersionedValue<V>; EXPECTED_NUM_VERSIONS]>,
}

impl<V: Copyable> VersionedSlot<V> {
    pub fn empty() -> Self {
        Self {
            versions: smallvec![],
        }
    }

    /// Returns a versioned slot with a single value.
    pub fn new(value: V, version: u32) -> Self {
        let v = VersionedValue {
            value,
            version,
            incarnation: 0,
        };
        Self {
            versions: smallvec![v],
        }
    }

    pub fn set(&mut self, value: V, version: u32, incarnation: u32) -> PartialVMResult<&mut V> {
        // TODO: change to error.
        assert!(self
            .versions
            .last()
            .map(|v| v.version < version || v.version == version && v.incarnation < incarnation)
            .unwrap_or(true));

        if self.versions.last().map_or(true, |v| v.version < version) {
            self.versions.push(VersionedValue {
                value,
                version,
                incarnation,
            });
        } else {
            let last = self.versions.last_mut().expect("Already checked");
            last.value = value;
            last.incarnation = incarnation;
        }

        Ok(self
            .versions
            .last_mut()
            .map(|v| &mut v.value)
            .expect("Last version was just inserted"))
    }

    /// Pops and returns the latest value whose version is at most the current version. Returns
    /// [None] is such a value does not exist.
    pub fn take_latest(&mut self, current_version: u32) -> Option<V> {
        self.sync_for_read(current_version);
        self.versions.pop().map(|v| v.value)
    }

    /// Returns the reference to the latest value whose version is at most the current version.
    /// Returns [None] is such a value does not exist.
    pub fn latest(&mut self, current_version: u32) -> Option<&V> {
        self.sync_for_read(current_version);
        self.versions.last().map(|v| &v.value)
    }

    pub fn latest_versioned(&mut self, current_version: u32) -> Option<(&V, u32, u32)> {
        self.sync_for_read(current_version);
        self.versions
            .last()
            .map(|v| (&v.value, v.version, v.incarnation))
    }

    /// Returns the mutable reference to the value if its version is equal to the current version.
    /// If the version is larger, performs a copy-on-write and returns the mutable reference to the
    /// copied value which was inserted at the current version.
    ///
    /// Returns [None] is such a value does not exist.
    /// Returns an error is copying the value failed.
    pub fn latest_mut(&mut self, current_version: u32) -> PartialVMResult<Option<&mut V>> {
        self.sync_for_write(current_version)?;
        Ok(self.versions.last_mut().map(|v| &mut v.value))
    }

    // Note: materialization only, as it patches metadata.
    pub fn latest_mut_sync_for_read(&mut self, current_version: u32) -> Option<&mut V> {
        self.sync_for_read(current_version);
        self.versions.last_mut().map(|v| &mut v.value)
    }

    /// Checks if derived information needs to be recomputed by comparing the source value's
    /// version/incarnation against the derived slot. Returns the source value data and old
    /// derived value if recomputation is needed, or None if the derived data is up-to-date.
    ///
    /// This is used for materialization where the derived slot contains computed results
    /// (like WriteOps) that depend on the source value and need updating when the source changes.
    ///
    /// # Panics
    ///
    /// Panics if the derived slot has a newer version than the source slot, or if derived
    /// data exists without a corresponding source value. These indicate invariant violations
    /// in the versioning system.
    pub fn needs_derived_recomputation<'a, U: Copyable>(
        &'a mut self,
        derived_slot: &'a mut VersionedSlot<U>,
        current_version: u32,
    ) -> Option<(&'a V, u32, u32, Option<&'a U>)> {
        let source = self.latest_versioned(current_version);
        let derived = derived_slot.latest_versioned(current_version);

        match (source, derived) {
            // No-op: source does not exist / there was an undo which pruned existing source and its
            // derived data.
            (None, None) => None,
            // Invariant violation: derived data cannot exist without a source value.
            (None, Some(_)) => {
                unreachable!("Derived data exists without corresponding source value")
            },
            // Source exists but no derived data - need to compute.
            (Some((source, source_version, source_incarnation)), None) => {
                Some((source, source_version, source_incarnation, None))
            },
            // Both source and derived exist - check if derived is outdated.
            (
                Some((source, source_version, source_incarnation)),
                Some((old_derived, derived_version, derived_incarnation)),
            ) => {
                match source_version.cmp(&derived_version) {
                    // Invariant violation: derived data cannot be newer than source.
                    Ordering::Less => {
                        unreachable!("Derived version cannot be larger than source version")
                    },
                    // Same version, check incarnation to see if source was updated.
                    Ordering::Equal if source_incarnation > derived_incarnation => Some((
                        source,
                        source_version,
                        source_incarnation,
                        Some(old_derived),
                    )),
                    // Derived data is up-to-date.
                    Ordering::Equal => {
                        debug_assert_eq!(source_incarnation, derived_incarnation);
                        None
                    },
                    // Source is newer version - need to recompute derived data.
                    Ordering::Greater => Some((
                        source,
                        source_version,
                        source_incarnation,
                        Some(old_derived),
                    )),
                }
            },
        }
    }
}

// Private interfaces.
impl<V: Copyable> VersionedSlot<V> {
    /// Should be called on read access. Synchronizes the value by removing any "dead" versions
    /// (i.e., versions greater than the current version).
    fn sync_for_read(&mut self, current_version: u32) {
        while self
            .versions
            .last()
            .map_or(false, |v| v.version > current_version)
        {
            self.versions.pop();
        }
    }

    /// Should be called on write access. Synchronizes the value by:
    ///   1. Removing any "dead" versions (same as synchronizing for read).
    ///   2. If the current version is larger than the latest version in a slot, performs a copy-
    ///      on-write and inserts a new version.
    fn sync_for_write(&mut self, current_version: u32) -> PartialVMResult<()> {
        self.sync_for_read(current_version);

        // After syncing for read, the last value is at most current version. If it is smaller, we
        // need to do CoW.
        if let Some(last) = self.versions.last_mut() {
            match last.version.cmp(&current_version) {
                Ordering::Less => {
                    let value = last.value.clone_value()?;
                    self.versions.push(VersionedValue {
                        value,
                        version: current_version,
                        incarnation: 0,
                    });
                },
                Ordering::Equal => {
                    last.incarnation += 1;
                },
                Ordering::Greater => unreachable!("Latest value cannot have larger version"),
            }
        }
        Ok(())
    }
}

#[cfg(test)]
impl<V: Copyable> VersionedSlot<V> {
    pub fn new_for_test(versions: impl IntoIterator<Item = (V, u32)>) -> Self {
        let versions = versions
            .into_iter()
            .map(|(value, version)| VersionedValue {
                value,
                version,
                incarnation: 0,
            })
            .collect();
        Self { versions }
    }

    fn view(&self) -> Vec<(V, u32)> {
        self.versions
            .iter()
            .map(|v| (v.value.clone_value().unwrap(), v.version))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::assert_ok;

    #[derive(Debug, Eq, PartialEq)]
    struct MockValue(u64);

    impl Copyable for MockValue {
        fn clone_value(&self) -> PartialVMResult<Self> {
            Ok(Self(self.0))
        }
    }

    #[test]
    fn test_sync_for_read() {
        let mut v = VersionedSlot::new_for_test(vec![
            (MockValue(1), 1),
            (MockValue(2), 2),
            (MockValue(3), 3),
            (MockValue(5), 5),
        ]);

        assert_eq!(v.view(), vec![
            (MockValue(1), 1),
            (MockValue(2), 2),
            (MockValue(3), 3),
            (MockValue(5), 5)
        ]);

        v.sync_for_read(4);
        assert_eq!(v.view(), vec![
            (MockValue(1), 1),
            (MockValue(2), 2),
            (MockValue(3), 3)
        ]);

        v.sync_for_read(2);
        assert_eq!(v.view(), vec![(MockValue(1), 1), (MockValue(2), 2)]);

        v.sync_for_read(0);
        assert_eq!(v.view(), vec![]);
    }

    #[test]
    fn test_sync_for_write() {
        let mut v = VersionedSlot::new_for_test(vec![
            (MockValue(1), 1),
            (MockValue(2), 2),
            (MockValue(3), 3),
            (MockValue(5), 5),
        ]);

        assert_eq!(v.view(), vec![
            (MockValue(1), 1),
            (MockValue(2), 2),
            (MockValue(3), 3),
            (MockValue(5), 5)
        ]);

        assert_ok!(v.sync_for_write(3));
        assert_eq!(v.view(), vec![
            (MockValue(1), 1),
            (MockValue(2), 2),
            (MockValue(3), 3)
        ]);

        assert_ok!(v.sync_for_write(4));
        assert_eq!(v.view(), vec![
            (MockValue(1), 1),
            (MockValue(2), 2),
            (MockValue(3), 3),
            (MockValue(3), 4)
        ]);

        v.sync_for_read(0);
        assert_eq!(v.view(), vec![]);

        assert_ok!(v.sync_for_write(3));
        assert_eq!(v.view(), vec![]);
    }

    #[test]
    fn test_version_control() {
        let mut vc = VersionController::new();
        assert_eq!(vc.next_version, 1);
        assert_eq!(vc.current_version, 0);
        assert_eq!(vc.saved_versions(), vec![0]);

        // No-op.
        vc.undo();
        assert_eq!(vc.next_version, 1);
        assert_eq!(vc.current_version, 0);
        assert_eq!(vc.saved_versions(), vec![0]);

        vc.save();
        assert_eq!(vc.next_version, 2);
        assert_eq!(vc.current_version, 1);
        assert_eq!(vc.saved_versions(), vec![0, 1]);

        vc.save();
        assert_eq!(vc.next_version, 3);
        assert_eq!(vc.current_version, 2);
        assert_eq!(vc.saved_versions(), vec![0, 1, 2]);

        vc.undo();
        assert_eq!(vc.next_version, 3);
        assert_eq!(vc.current_version, 1);
        assert_eq!(vc.saved_versions(), vec![0, 1]);

        vc.undo();
        assert_eq!(vc.next_version, 3);
        assert_eq!(vc.current_version, 0);
        assert_eq!(vc.saved_versions(), vec![0]);

        vc.save();
        assert_eq!(vc.next_version, 4);
        assert_eq!(vc.current_version, 3);
        assert_eq!(vc.saved_versions(), vec![0, 3]);
    }
}
