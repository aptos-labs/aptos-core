// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Implements versioning data structure to support linear history for Aptos VM data caches and
//! stateful extensions (like tables).

use move_binary_format::errors::PartialVMResult;
use move_vm_types::values::Copyable;
use smallvec::{smallvec, SmallVec};
use std::cmp::Ordering;

/// In practice, we expect a small number of versions (and hence, we do not worry about memory
/// consumption).
const EXPECTED_NUM_VERSIONS: usize = 3;

/// Controls the current version for data structures where keys need to be versioned, and their
/// state can be saved or rolled back to the previously saved state.
pub struct VersionController {
    /// Next version to use on save / undo. Monotonically increases and cannot be reused to prevent
    /// ABA problems.
    next_version: u32,
    /// Checkpoints for saved versions. If empty, it means we are in the initial empty state (0).
    saved_versions: SmallVec<[u32; EXPECTED_NUM_VERSIONS]>,
    /// Current (working) version of the data structure.
    current_version: u32,
}

#[derive(Debug, Copy, Clone)]
pub struct CurrentVersion {
    /// Current (working) version of the data structure.
    current: u32,
    /// Version of the latest saved checkpoint. Always exists (initial empty state is considered to
    /// be saved).
    latest_saved: u32,
}

#[cfg(test)]
impl CurrentVersion {
    fn test_only(current: u32, latest_saved: u32) -> Self {
        Self {
            current,
            latest_saved,
        }
    }
}

impl Default for VersionController {
    fn default() -> Self {
        Self::new()
    }
}

impl VersionController {
    /// Creates a new control structure with current empty state being saved.
    pub fn new() -> Self {
        // Note: we start at version 1 because version 0 corresponds to initial empty state that is
        // always saved.
        Self {
            next_version: 2,
            saved_versions: smallvec![],
            current_version: 1,
        }
    }

    /// Returns the current version information.
    pub fn current_version(&self) -> CurrentVersion {
        CurrentVersion {
            current: self.current_version,
            latest_saved: self.saved_versions.last().copied().unwrap_or(0),
        }
    }

    /// Saves the current version as a checkpoint, incrementing the current version.
    pub fn save(&mut self) {
        self.saved_versions.push(self.current_version);
        self.current_version = self.next_version;
        self.next_version += 1;
    }

    /// Rollbacks the current version to previously saved version.
    pub fn undo(&mut self) {
        if let Some(last_saved) = self.saved_versions.pop() {
            self.current_version = last_saved;
        } else {
            // No saved versions, advance to new unused version.
            self.current_version = self.next_version;
            self.next_version += 1;
        }
    }

    #[cfg(test)]
    fn saved_versions(&self) -> Vec<u32> {
        self.saved_versions.iter().cloned().collect()
    }
}

struct VersionedValue<V> {
    version: u32,
    value: V,
}

/// A container for multiple versions of the same value. Allows to return an immutable reference to
/// the latest value (w.r.t. some current working version). When returning a mutable reference, the
/// value may be copied to a newer version, in order to allow for rollbacks.
///
/// Not accessible versions are actively garbage collected on every access to the slot, ensuring
/// the invariant that all versions stored are always accessible. In practice, we do not expect to
/// see many versions and so GC should not be too expensive.
pub struct VersionedSlot<V> {
    /// Inner versioned values.
    ///
    /// INVARIANT: always stored in monotonically increasing version order.
    versions: SmallVec<[VersionedValue<V>; EXPECTED_NUM_VERSIONS]>,
}

impl<V> Default for VersionedSlot<V>
where
    V: Copyable,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<V> VersionedSlot<V>
where
    V: Copyable,
{
    /// Creates a new empty versioned slot.
    pub fn new() -> Self {
        Self {
            versions: smallvec![],
        }
    }

    /// Returns true if no accessible versions exist.
    pub fn check_empty(&mut self, version: CurrentVersion) -> bool {
        self.gc(version);
        self.versions.is_empty()
    }

    /// Sets value at current version for an empty slot. Panics if slot is not empty under current
    /// version. Returns reference to the newly inserted value.
    pub fn set_empty(&mut self, value: V, version: CurrentVersion) -> &mut V {
        assert!(self.check_empty(version));
        self.versions.push(VersionedValue {
            version: version.current,
            value,
        });
        &mut self
            .versions
            .last_mut()
            .expect("Value has just been inserted")
            .value
    }

    /// Returns immutable reference to current value.
    pub fn get(&mut self, version: CurrentVersion) -> Option<&V> {
        self.gc(version);
        self.versions.last().map(|v| &v.value)
    }

    /// Returns mutable reference to the current value if its version is the same as the current
    /// working version, otherwise performs copy-on-write and returns a mutable reference to the
    /// new copy.
    pub fn get_mut(&mut self, version: CurrentVersion) -> PartialVMResult<Option<&mut V>> {
        self.gc(version);
        self.maybe_cow(version.current)?;
        Ok(self.versions.last_mut().map(|v| &mut v.value))
    }

    /// Takes the current value.
    pub fn take(&mut self, version: CurrentVersion) -> Option<V> {
        self.gc(version);
        self.versions.pop().map(|v| v.value)
    }

    /// Garbage collect versions that are no longer accessible.
    fn gc(&mut self, version: CurrentVersion) {
        // Step 1: pop all versions larger than the current.
        while self
            .versions
            .last()
            .map_or(false, |v| v.version > version.current)
        {
            self.versions.pop();
        }

        // Step 2: if last version is smaller than the current, we might still need to garbage
        // collect it by comparing with the latest saved version. We would keep removing values
        // that have version larger than the latest saved.
        if let Some(last) = self.versions.last() {
            if last.version < version.current {
                while self
                    .versions
                    .last()
                    .map_or(false, |v| v.version > version.latest_saved)
                {
                    self.versions.pop();
                }
            }
        }
    }

    /// Performs copy-on-write if the latest version is smaller than the current working version.
    ///
    /// INVARIANT: Should only be called after garbage collection.
    fn maybe_cow(&mut self, version: u32) -> PartialVMResult<()> {
        if let Some(last) = self.versions.last_mut() {
            match last.version.cmp(&version) {
                Ordering::Greater => {
                    // The caller should garbage-collect unused versions.
                    unreachable!("Latest value cannot have larger version")
                },
                Ordering::Equal => (),
                Ordering::Less => {
                    let value = last.value.clone_value()?;
                    self.versions.push(VersionedValue { value, version });
                },
            }
        }
        Ok(())
    }

    #[cfg(any(test, feature = "testing"))]
    pub fn to_versions_vec(&self) -> Vec<u32> {
        self.versions.iter().map(|v| v.version).collect()
    }
}

#[cfg(test)]
impl<V> VersionedSlot<V>
where
    V: Clone,
{
    fn to_vec(&self) -> Vec<(u32, V)> {
        self.versions
            .iter()
            .map(|v| (v.version, v.value.clone()))
            .collect()
    }

    fn from_vec(versions: Vec<(u32, V)>) -> Self {
        let versions = versions
            .into_iter()
            .map(|(version, value)| VersionedValue { version, value })
            .collect();
        Self { versions }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::{collection::vec, prelude::*};
    use std::collections::HashSet;

    #[test]
    fn test_initial_state() {
        let vc = VersionController::new();
        assert_eq!(vc.current_version, 1);
        assert_eq!(vc.next_version, 2);
        assert!(vc.saved_versions().is_empty());

        let version = vc.current_version();
        assert_eq!(version.current, 1);
        assert_eq!(version.latest_saved, 0);
    }

    #[test]
    fn test_undo_from_empty_state() {
        let mut vc = VersionController::new();

        vc.undo();
        let version = vc.current_version();
        assert_eq!(vc.current_version, 2);
        assert_eq!(vc.next_version, 3);
        assert!(vc.saved_versions().is_empty());
        assert_eq!(version.current, 2);
        assert_eq!(version.latest_saved, 0);

        vc.undo();
        vc.undo();
        vc.undo();
        let version = vc.current_version();
        assert_eq!(vc.current_version, 5);
        assert_eq!(vc.next_version, 6);
        assert!(vc.saved_versions().is_empty());
        assert_eq!(version.current, 5);
        assert_eq!(version.latest_saved, 0);
    }

    #[test]
    fn test_saves_only() {
        let mut vc = VersionController::new();

        // Save current version 1, transition to version 2.
        vc.save();
        let version = vc.current_version();
        assert_eq!(vc.current_version, 2);
        assert_eq!(vc.next_version, 3);
        assert_eq!(vc.saved_versions(), vec![1]);
        assert_eq!(version.current, 2);
        assert_eq!(version.latest_saved, 1);

        // Save current version 2, transition to version 3.
        vc.save();
        let version = vc.current_version();
        assert_eq!(vc.current_version, 3);
        assert_eq!(vc.next_version, 4);
        assert_eq!(vc.saved_versions(), vec![1, 2]);
        assert_eq!(version.current, 3);
        assert_eq!(version.latest_saved, 2);

        // Save current versions 3, 4 and 5, transition to version 6.
        vc.save();
        vc.save();
        vc.save();
        let version = vc.current_version();
        assert_eq!(vc.current_version, 6);
        assert_eq!(vc.next_version, 7);
        assert_eq!(vc.saved_versions(), vec![1, 2, 3, 4, 5]);
        assert_eq!(version.current, 6);
        assert_eq!(version.latest_saved, 5);
    }

    #[test]
    fn test_save_undo_save() {
        let mut vc = VersionController::new();
        // Current is 2, saved: 1.
        vc.save();
        // Current is 1 again, nothing is saved.
        vc.undo();

        let version = vc.current_version();
        assert_eq!(vc.current_version, 1);
        assert_eq!(vc.next_version, 3);
        assert!(vc.saved_versions().is_empty());
        assert_eq!(version.current, 1);
        assert_eq!(version.latest_saved, 0);

        // We save 1, current is now 3.
        vc.save();
        let version = vc.current_version();
        assert_eq!(vc.current_version, 3);
        assert_eq!(vc.next_version, 4);
        assert_eq!(vc.saved_versions(), vec![1]);
        assert_eq!(version.current, 3);
        assert_eq!(version.latest_saved, 1);
    }

    #[test]
    fn test_complex_sequence() {
        let mut vc = VersionController::new();

        // Saved 1, current is 2.
        vc.save();
        // Saved 1, 2, current is 3.
        vc.save();
        // Saved 1, current is 2.
        vc.undo();
        // Saved 1, 2, current is 4.
        vc.save();
        // Saved 1, current is 2.
        vc.undo();
        // Nothing saved (i.e., 0 base state), current is 1.
        vc.undo();
        // Nothing saved (i.e., 0 base state), current is 5.
        vc.undo();
        // Saved 5, current is 6.
        vc.save();

        assert_eq!(vc.current_version, 6);
        assert_eq!(vc.saved_versions(), vec![5]);
        assert_eq!(vc.next_version, 7);
        let version = vc.current_version();
        assert_eq!(version.current, 6);
        assert_eq!(version.latest_saved, 5);
    }

    #[derive(Debug, Clone)]
    enum Operation {
        Save,
        Undo,
    }

    impl Arbitrary for Operation {
        type Parameters = ();
        type Strategy = BoxedStrategy<Self>;

        fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
            prop_oneof![Just(Operation::Save), Just(Operation::Undo),].boxed()
        }
    }

    proptest! {
        #[test]
        fn test_arbitrary_ops(ops in vec(any::<Operation>(), 0..300)) {
            let mut vc = VersionController::new();

            for op in ops {
                match op {
                    Operation::Save => vc.save(),
                    Operation::Undo => vc.undo(),
                }
                let version = vc.current_version();

                // INVARIANT: current_version is never 0 (0 is conceptual empty state).
                prop_assert!(version.current > 0);

                // INVARIANT: current version is always ahead of latest saved.
                prop_assert!(version.current > version.latest_saved);

                let saved_versions = vc.saved_versions();
                let mut versions_set = HashSet::new();
                for i in 0..saved_versions.len() {
                    // INVARIANT: saved versions have no duplicates.
                    prop_assert!(versions_set.insert(saved_versions[i]));

                    // INVARIANT: saved versions are monotonically increasing.
                    if i > 0 {
                        prop_assert!(saved_versions[i - 1] < saved_versions[i]);
                    }
                }

                // INVARIANT: saved versions never contain 0 (conceptual state).
                prop_assert!(!saved_versions.contains(&0));
            }
        }
    }

    #[test]
    fn test_gc_removes_old_versions() {
        let mut slot = VersionedSlot::from_vec(vec![(1, 10), (7, 70), (8, 80), (10, 100)]);
        slot.gc(CurrentVersion::test_only(10, 0));
        assert_eq!(slot.to_vec(), vec![(1, 10), (7, 70), (8, 80), (10, 100)]);

        let mut slot = VersionedSlot::from_vec(vec![(1, 10), (7, 70), (8, 80), (10, 100)]);
        slot.gc(CurrentVersion::test_only(7, 0));
        assert_eq!(slot.to_vec(), vec![(1, 10), (7, 70)]);

        let mut slot = VersionedSlot::from_vec(vec![(1, 10), (5, 50), (8, 80), (10, 100)]);
        slot.gc(CurrentVersion::test_only(7, 6));
        assert_eq!(slot.to_vec(), vec![(1, 10), (5, 50)]);

        let mut slot = VersionedSlot::from_vec(vec![(1, 10), (5, 50), (8, 80), (10, 100)]);
        slot.gc(CurrentVersion::test_only(7, 2));
        assert_eq!(slot.to_vec(), vec![(1, 10)]);

        let mut slot = VersionedSlot::from_vec(vec![(1, 10), (5, 50), (8, 80), (10, 100)]);
        slot.gc(CurrentVersion::test_only(7, 1));
        assert_eq!(slot.to_vec(), vec![(1, 10)]);

        let mut slot = VersionedSlot::from_vec(vec![(1, 10), (5, 50), (8, 80), (10, 100)]);
        slot.gc(CurrentVersion::test_only(7, 0));
        assert!(slot.to_vec().is_empty());
    }

    #[test]
    fn test_cow() {
        let mut slot = VersionedSlot::from_vec(vec![(1, 10), (7, 70)]);

        slot.maybe_cow(7).unwrap();
        assert_eq!(slot.to_vec(), vec![(1, 10), (7, 70)]);

        slot.maybe_cow(8).unwrap();
        assert_eq!(slot.to_vec(), vec![(1, 10), (7, 70), (8, 70)]);

        slot.maybe_cow(20).unwrap();
        assert_eq!(slot.to_vec(), vec![(1, 10), (7, 70), (8, 70), (20, 70)]);
    }

    #[test]
    fn test_versioned_slot() {
        let mut vc = VersionController::new();

        let mut slot = VersionedSlot::new();
        slot.set_empty(42, vc.current_version());
        assert_eq!(slot.get(vc.current_version()).copied().unwrap(), 42);
        assert_eq!(
            slot.get_mut(vc.current_version())
                .unwrap()
                .copied()
                .unwrap(),
            42
        );
        assert_eq!(slot.to_vec(), vec![(1, 42)]);

        // Advance to version 2.
        vc.save();

        assert_eq!(slot.get(vc.current_version()).copied().unwrap(), 42);
        assert_eq!(slot.to_vec(), vec![(1, 42)]);

        assert_eq!(
            slot.get_mut(vc.current_version())
                .unwrap()
                .copied()
                .unwrap(),
            42
        );
        assert_eq!(slot.to_vec(), vec![(1, 42), (2, 42)]);

        // Advance to version 3 and then undo back to version 1.
        vc.save();
        vc.undo();
        vc.undo();

        assert_eq!(slot.get(vc.current_version()).copied().unwrap(), 42);
        assert_eq!(
            slot.get_mut(vc.current_version())
                .unwrap()
                .copied()
                .unwrap(),
            42
        );
        assert_eq!(slot.to_vec(), vec![(1, 42)]);

        // Advance to version 4.
        vc.save();

        assert_eq!(
            slot.get_mut(vc.current_version())
                .unwrap()
                .copied()
                .unwrap(),
            42
        );
        assert_eq!(slot.to_vec(), vec![(1, 42), (4, 42)]);

        // Undo back to 1, then advance to 5, back to 1, and advance to 6 (so latest saved is 1,
        // and current is 6)
        vc.undo();
        vc.save();
        vc.undo();
        vc.save();

        assert_eq!(slot.get(vc.current_version()).copied().unwrap(), 42);
        assert_eq!(slot.to_vec(), vec![(1, 42)]);

        assert_eq!(
            slot.get_mut(vc.current_version())
                .unwrap()
                .copied()
                .unwrap(),
            42
        );
        assert_eq!(slot.to_vec(), vec![(1, 42), (6, 42)]);
    }
}
