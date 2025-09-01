// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::values::{Container, ContainerRef, Value, ValueImpl, DEFAULT_MAX_VM_VALUE_NESTED_DEPTH};
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::vm_status::StatusCode;
use smallvec::{smallvec, SmallVec};
use std::{cell::RefCell, cmp::Ordering, mem, rc::Rc};

/// Represents the state of a slot in global storage.
#[derive(Debug)]
pub enum GlobalValueState {
    /// A global value has been read only, not yet modified.
    Read(Value),
    /// A global value has been created and moved into the slot.
    Creation(Value),
    /// A global value has been modified.
    Modification(Value),
    /// A global value has been moved from the slot (deleted).
    Deletion,
    /// A global value never existed in the slot.
    None,
}

impl GlobalValueState {
    /// On modification, returns a copy of the stored value (if exists) also performing the state
    /// transition. Specifically:
    ///   - Read values become modifications.
    ///   - Created values stay creations.
    ///   - Modified values stay modifications.
    ///   - Deleted and non-existent values stay as is.
    fn copy_on_modification(&self) -> PartialVMResult<Self> {
        let max_depth = Some(DEFAULT_MAX_VM_VALUE_NESTED_DEPTH);
        Ok(match self {
            GlobalValueState::Read(v) | GlobalValueState::Modification(v) => {
                GlobalValueState::Modification(Value(v.0.copy_value(1, max_depth)?))
            },
            GlobalValueState::Creation(v) => {
                GlobalValueState::Creation(Value(v.0.copy_value(1, max_depth)?))
            },
            GlobalValueState::Deletion => GlobalValueState::Deletion,
            GlobalValueState::None => GlobalValueState::None,
        })
    }
}

/// A global slot that supports linear history with versioning.
pub struct VersionedGlobalValue {
    pub versions: Rc<RefCell<VersionedCell<GlobalValueState>>>,
}

impl VersionedGlobalValue {
    pub fn new(gv: GlobalValueState, version: u32) -> Self {
        Self {
            versions: Rc::new(RefCell::new(VersionedCell::new(gv, version))),
        }
    }

    pub fn check_if_initialized(&mut self, current_version: u32) -> bool {
        self.versions.borrow_mut().latest(current_version).is_some()
    }

    pub fn set(&mut self, value: GlobalValueState, current_version: u32) -> PartialVMResult<()> {
        self.versions.borrow_mut().set(value, current_version)?;
        Ok(())
    }

    pub fn exists(&self, current_version: u32) -> PartialVMResult<Value> {
        let mut versions = self.versions.borrow_mut();
        let v = versions
            .latest(current_version)
            .ok_or_else(|| PartialVMError::new_invariant_violation("Non-initialized slot"))?;
        let result = match v {
            GlobalValueState::Read(_)
            | GlobalValueState::Creation(_)
            | GlobalValueState::Modification(_) => true,
            GlobalValueState::Deletion | GlobalValueState::None => false,
        };
        Ok(Value::bool(result))
    }

    pub fn borrow_global(&self, current_version: u32) -> PartialVMResult<Value> {
        let ptr = ValuePtr::new(self.versions.clone(), current_version)?;

        let mut versions = self.versions.borrow_mut();
        let gv = versions
            .latest(current_version)
            .ok_or_else(|| PartialVMError::new_invariant_violation("Non-initialized slot"))?;
        match gv {
            GlobalValueState::Read(v)
            | GlobalValueState::Creation(v)
            | GlobalValueState::Modification(v) => {
                if let ValueImpl::Container(Container::Struct(fields)) = &v.0 {
                    Ok(Value(ValueImpl::ContainerRef(ContainerRef::Global {
                        container: Container::Struct(Rc::clone(fields)),
                        ptr,
                    })))
                } else {
                    Err(PartialVMError::new_invariant_violation("All global values are structs"))
                }
            },
            GlobalValueState::Deletion | GlobalValueState::None => {
                Err(PartialVMError::new(StatusCode::MISSING_DATA))
            },
        }
    }

    pub fn move_from(&self, current_version: u32) -> PartialVMResult<Value> {
        let mut versions = self.versions.borrow_mut();
        versions.cleanup_versions_after(current_version);
        let (sv, version) = versions
            .latest_versioned(current_version)
            .ok_or_else(|| PartialVMError::new_invariant_violation("Non-initialized slot"))?;
        match sv {
            GlobalValueState::Read(_)
            | GlobalValueState::Creation(_)
            | GlobalValueState::Modification(_) => (),
            GlobalValueState::Deletion | GlobalValueState::None => {
                return Err(PartialVMError::new(StatusCode::MISSING_DATA));
            },
        };

        match version.cmp(&current_version) {
            Ordering::Less => {
                let sv = versions
                    .latest(current_version)
                    .ok_or_else(|| PartialVMError::new_invariant_violation("Non-initialized slot"))?
                    .copy_on_modification()?;
                versions.versions.push(VersionedValue {
                    value: GlobalValueState::Deletion,
                    version: current_version,
                });
                match sv {
                    GlobalValueState::Creation(v) | GlobalValueState::Modification(v) => {
                        if let ValueImpl::Container(Container::Struct(fields)) = v.0 {
                            Ok(Value(ValueImpl::Container(Container::Struct(
                                fields,
                            ))))
                        } else {
                            Err(PartialVMError::new_invariant_violation("todo"))
                        }
                    },
                    GlobalValueState::Read(_)
                    | GlobalValueState::Deletion
                    | GlobalValueState::None => {
                        unreachable!()
                    },
                }
            },
            Ordering::Equal => {
                let idx = versions.versions.len() - 1;
                let v = mem::replace(
                    &mut versions.versions[idx].value,
                    GlobalValueState::Deletion,
                );
                match v {
                    GlobalValueState::Read(v)
                    | GlobalValueState::Creation(v)
                    | GlobalValueState::Modification(v) => {
                        if let ValueImpl::Container(Container::Struct(fields)) = v.0 {
                            Ok(Value(ValueImpl::Container(Container::Struct(fields))))
                        } else {
                            Err(PartialVMError::new_invariant_violation("todo"))
                        }
                    },
                    GlobalValueState::Deletion | GlobalValueState::None => unreachable!(),
                }
            },
            Ordering::Greater => unreachable!("Latest value cannot have larger version"),
        }
    }

    pub fn move_to(&self, current_version: u32, value: Value) -> PartialVMResult<()> {
        let mut versions = self.versions.borrow_mut();
        versions.cleanup_versions_after(current_version);
        let (sv, version) = versions
            .latest_versioned(current_version)
            .ok_or_else(|| PartialVMError::new_invariant_violation("Non-initialized slot"))?;
        match sv {
            GlobalValueState::Read(_)
            | GlobalValueState::Creation(_)
            | GlobalValueState::Modification(_) => {
                return Err(PartialVMError::new(StatusCode::RESOURCE_ALREADY_EXISTS));
            },
            GlobalValueState::Deletion | GlobalValueState::None => (),
        };

        match version.cmp(&current_version) {
            Ordering::Less => {
                versions.versions.push(VersionedValue {
                    value: GlobalValueState::Creation(value),
                    version: current_version,
                });
            },
            Ordering::Equal => {
                let idx = versions.versions.len() - 1;
                let _ = mem::replace(
                    &mut versions.versions[idx].value,
                    GlobalValueState::Creation(value),
                );
            },
            Ordering::Greater => unreachable!("Latest value cannot have larger version"),
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ValuePtr {
    value: Rc<RefCell<VersionedCell<GlobalValueState>>>,
    version: Rc<RefCell<u32>>,
}

impl ValuePtr {

    fn new(value: Rc<RefCell<VersionedCell<GlobalValueState>>>, version: u32) -> PartialVMResult<Self> {
        if value.borrow_mut().latest(version).is_none() {
            return Err(PartialVMError::new_invariant_violation("Value pointer cannot point to empty slot"));
        }
        Ok(Self {
            value,
            version: Rc::new(RefCell::new(version)),
        })
    }

    pub fn copy_on_write(&self, current_version: u32) -> PartialVMResult<bool> {
        let mut value = self.value.borrow_mut();
        value.cleanup_versions_after(current_version);

        let mut version = self.version.borrow_mut();
        match version.cmp(&current_version) {
            Ordering::Greater => unreachable!("Latest value cannot have larger version"),
            Ordering::Equal => Ok(false),
            Ordering::Less => {
                let vv = value.versions.last_mut().expect("Should exist");
                vv.version = current_version;
                let new = vv.value.copy_on_modification()?;
                value.versions.push(VersionedValue {
                    value: new,
                    version: *version, // old version
                });

                // at this point, we have current with
                let len = value.versions.len();
                value.versions.swap(len - 1, len - 2);
                *version = current_version;
                Ok(true)
            },
        }
    }
}

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
#[derive(Debug)]
struct VersionedValue<V> {
    /// The actual value the entry contains. Copied on mutable access.
    value: V,
    /// The version when this entry was inserted (possible to rollback).
    version: u32,
}

/// In practice, we expect a small number of versions (and hence, we do not worry about memory
/// consumption).
const EXPECTED_NUM_VERSIONS: usize = 3;

/// Contains different (monotonically increasing) versions of the value.
#[derive(Debug)]
pub struct VersionedCell<V> {
    versions: SmallVec<[VersionedValue<V>; EXPECTED_NUM_VERSIONS]>,
}

impl<V> VersionedCell<V> {
    pub fn empty() -> Self {
        Self {
            versions: smallvec![],
        }
    }

    /// Returns a versioned slot with a single value.
    pub fn new(value: V, version: u32) -> Self {
        let v = VersionedValue { value, version };
        Self {
            versions: smallvec![v],
        }
    }

    pub fn set(&mut self, value: V, version: u32) -> PartialVMResult<&mut V> {
        if self.versions.last().map_or(true, |v| v.version < version) {
            self.versions.push(VersionedValue { value, version });
        } else {
            let msg = format!(
                "Setting version {}, but current latest version is higher",
                version
            );
            return Err(PartialVMError::new_invariant_violation(msg));
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
        self.cleanup_versions_after(current_version);
        self.versions.pop().map(|v| v.value)
    }

    /// Returns the reference to the latest value whose version is at most the current version.
    /// Returns [None] is such a value does not exist.
    pub fn latest(&mut self, current_version: u32) -> Option<&V> {
        self.cleanup_versions_after(current_version);
        self.versions.last().map(|v| &v.value)
    }

    pub fn latest_versioned(&mut self, current_version: u32) -> Option<(&V, u32)> {
        self.cleanup_versions_after(current_version);
        self.versions.last().map(|v| (&v.value, v.version))
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
    pub fn needs_derived_recomputation<'a, U>(
        &'a mut self,
        derived_slot: &'a mut VersionedCell<U>,
        current_version: u32,
    ) -> bool {
        let source = self.latest_versioned(current_version);
        let derived = derived_slot.latest_versioned(current_version);

        match (source, derived) {
            // No-op: source does not exist / there was an undo which pruned existing source and its
            // derived data.
            (None, None) => false,
            // Invariant violation: derived data cannot exist without a source value.
            (None, Some(_)) => {
                unreachable!("Derived data exists without corresponding source value")
            },
            // Source exists but no derived data - need to compute.
            (Some(_), None) => true,
            // Both source and derived exist - check if derived is outdated.
            (Some((_, source_version)), Some((_, derived_version))) => {
                match source_version.cmp(&derived_version) {
                    // Invariant violation: derived data cannot be newer than source.
                    Ordering::Less => {
                        unreachable!("Derived version cannot be larger than source version")
                    },
                    // Same version, check incarnation to see if source was updated.
                    Ordering::Equal => false,
                    // Source is newer version - need to recompute derived data.
                    Ordering::Greater => true,
                }
            },
        }
    }
}

pub struct VersionedOnceCell<V> {
    inner: VersionedCell<V>,
}

impl<V> VersionedOnceCell<V> {
    pub fn empty() -> Self {
        Self {
            inner: VersionedCell::empty(),
        }
    }

    pub fn needs_derived_recomputation<U>(
        &mut self,
        slot: &mut VersionedCell<U>,
        current_version: u32,
    ) -> bool {
        slot.needs_derived_recomputation(&mut self.inner, current_version)
    }

    pub fn set(&mut self, v: V, current_version: u32) -> PartialVMResult<()> {
        self.inner.set(v, current_version)?;
        Ok(())
    }

    pub fn get_mut(&mut self, current_version: u32) -> Option<&mut V> {
        self.inner.cleanup_versions_after(current_version);
        self.inner.versions.last_mut().and_then(|v| {
            if v.version == current_version {
                Some(&mut v.value)
            } else {
                None
            }
        })
    }

    pub fn take_latest(&mut self, current_version: u32) -> Option<V> {
        self.inner.take_latest(current_version)
    }
}

impl<V: Clone> VersionedCell<V> {
    /// Returns the mutable reference to the value if its version is equal to the current version.
    /// If the version is larger, performs a copy-on-write and returns the mutable reference to the
    /// copied value which was inserted at the current version.
    ///
    /// Returns [None] is such a value does not exist.
    /// Returns an error is copying the value failed.
    pub fn latest_cow(&mut self, current_version: u32) -> Option<&mut V> {
        self.sync_for_write(current_version);
        self.versions.last_mut().map(|v| &mut v.value)
    }
}

// Private interfaces.
impl<V> VersionedCell<V> {
    /// Should be called on read access. Synchronizes the value by removing any "dead" versions
    /// (i.e., versions greater than the current version).
    fn cleanup_versions_after(&mut self, current_version: u32) {
        while self
            .versions
            .last()
            .map_or(false, |v| v.version > current_version)
        {
            self.versions.pop();
        }
    }
}

impl<V: Clone> VersionedCell<V> {
    /// Should be called on write access. Synchronizes the value by:
    ///   1. Removing any "dead" versions (same as synchronizing for read).
    ///   2. If the current version is larger than the latest version in a slot, performs a copy-
    ///      on-write and inserts a new version.
    fn sync_for_write(&mut self, current_version: u32) {
        self.cleanup_versions_after(current_version);

        // After syncing for read, the last value is at most current version. If it is smaller, we
        // need to do CoW.
        if let Some(last) = self.versions.last_mut() {
            match last.version.cmp(&current_version) {
                Ordering::Less => {
                    let value = last.value.clone();
                    self.versions.push(VersionedValue {
                        value,
                        version: current_version,
                    });
                },
                Ordering::Equal => (),
                Ordering::Greater => unreachable!("Latest value cannot have larger version"),
            }
        }
    }
}

#[cfg(test)]
impl<V: Clone> VersionedCell<V> {
    pub fn new_for_test(versions: impl IntoIterator<Item = (V, u32)>) -> Self {
        let versions = versions
            .into_iter()
            .map(|(value, version)| VersionedValue { value, version })
            .collect();
        Self { versions }
    }

    fn view(&self) -> Vec<(V, u32)> {
        self.versions
            .iter()
            .map(|v| (v.value.clone(), v.version))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct MockValue(u64);

    #[test]
    fn test_sync() {
        let mut v = VersionedCell::new_for_test(vec![
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

        v.cleanup_versions_after(4);
        assert_eq!(v.view(), vec![
            (MockValue(1), 1),
            (MockValue(2), 2),
            (MockValue(3), 3)
        ]);

        v.cleanup_versions_after(2);
        assert_eq!(v.view(), vec![(MockValue(1), 1), (MockValue(2), 2)]);

        v.cleanup_versions_after(0);
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
