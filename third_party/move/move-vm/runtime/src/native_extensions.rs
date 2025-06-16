// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use better_any::{Tid, TidAble, TidExt};
use smallvec::{smallvec, SmallVec};
use std::{any::TypeId, collections::HashMap};

pub trait Versioned {
    // todo: use result?
    fn clone_value(&self) -> Self;
}

impl<V> Versioned for V
where
    V: Clone,
{
    fn clone_value(&self) -> Self {
        self.clone()
    }
}

struct VersionedValue<V> {
    value: V,
    version: u32,
}

pub struct ValueHistory<V> {
    history: SmallVec<[VersionedValue<V>; 3]>,
}

impl<V> ValueHistory<V> {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            history: smallvec![],
        }
    }

    fn sync_for_read(&mut self, current_version: u32) {
        while self
            .history
            .last()
            .map_or(false, |v| v.version > current_version)
        {
            self.history.pop();
        }
    }

    // fn sync_for_write(&mut self, current_version: u32) {
    //     if let Some(last) = self.history.last() {
    //         if last.version > current_version {
    //             // Rollback: drop unused future versions.
    //             while self
    //                 .history
    //                 .last()
    //                 .map_or(false, |v| *v.version > current_version)
    //             {
    //                 self.history.pop();
    //             }
    //         } else if last.version < current_version {
    //             let value = VersionedValue {
    //                 value: last.value.clone(),
    //                 version: current_version,
    //             };
    //             self.history.push(value);
    //         }
    //         // Note: on equality, there is nothing to do.
    //     }
    // }

    /// returns false on override
    pub fn set(&mut self, current_version: u32, value: V) -> bool {
        match self.history.last_mut() {
            Some(v) if v.version == current_version => {
                v.value = value;
                false
            },
            _ => {
                self.history.push(VersionedValue {
                    value,
                    version: current_version,
                });
                true
            },
        }
    }

    pub fn into_last(mut self, current_version: u32) -> Option<V> {
        self.sync_for_read(current_version);
        Some(self.history.pop()?.value)
    }

    pub fn last(&mut self, current_version: u32) -> Option<&V> {
        self.sync_for_read(current_version);
        Some(&self.history.last()?.value)
    }

    pub fn last_mut(&mut self, _current_version: u32) -> Option<&mut V> {
        // TODO: CoW
        // self.sync_for_write(current_version);
        Some(&mut self.history.last_mut()?.value)
    }
}

#[cfg(any(test, feature = "testing"))]
impl<V> ValueHistory<V> {
    pub fn versions(&self) -> Vec<u32> {
        self.history.iter().map(|v| v.version).collect()
    }
}

/// Trait for extensions which can be saved at specific version, and later restored to it. The
/// history is linear, i.e., when saving at some state S, and then doing more changes, it is
/// possible to undo this changes and return to state S.
pub trait VersionControlledNativeExtension {
    /// Restores this extension to its latest saved version. If the current version is the first
    /// one - a no-op.
    fn undo(&mut self);

    /// Saves the data modified by the current extension. Before the next save, it is possible to
    /// return to this saved state by calling undo.
    fn save(&mut self);

    /// Updates or resets extension internal configs or data.
    fn update(&mut self, txn_hash: &[u8; 32], script_hash: &[u8]);
}

/// Any native extension should implement its version control. This way when a new extension gets
/// added there is a compile-time error when one tries to add it to the native context.
pub trait NativeExtension<'a>: VersionControlledNativeExtension + Tid<'a> {}

impl<'a, T> NativeExtension<'a> for T where T: VersionControlledNativeExtension + Tid<'a> {}

/// A data type to represent a heterogeneous collection of extensions which are available to
/// native functions. A value to this is passed into the session function execution.
///
/// The implementation uses the crate `better_any` which implements a version of the `Any`
/// type, called `Tid<`a>`, which allows for up to one lifetime parameter. This
/// avoids that extensions need to have `'static` lifetime, which `Any` requires. In order to make a
/// struct suitable to be a 'Tid', use `#[derive(Tid)]` in the struct declaration. (See also
/// tests at the end of this module.)
#[derive(Default)]
pub struct NativeContextExtensions<'a> {
    map: HashMap<TypeId, Box<dyn NativeExtension<'a>>>,
}

impl<'a> NativeContextExtensions<'a> {
    pub fn add<T: VersionControlledNativeExtension + TidAble<'a>>(&mut self, ext: T) {
        assert!(
            self.map.insert(T::id(), Box::new(ext)).is_none(),
            "multiple extensions of the same type not allowed"
        )
    }

    pub fn get<T: VersionControlledNativeExtension + TidAble<'a>>(&self) -> &T {
        self.map
            .get(&T::id())
            .expect("extension unknown")
            .as_ref()
            .downcast_ref::<T>()
            .unwrap()
    }

    pub fn get_mut<T: VersionControlledNativeExtension + TidAble<'a>>(&mut self) -> &mut T {
        self.map
            .get_mut(&T::id())
            .expect("extension unknown")
            .as_mut()
            .downcast_mut::<T>()
            .unwrap()
    }

    pub fn apply_to_all<F>(&mut self, f: F)
    where
        F: Fn(&mut dyn VersionControlledNativeExtension),
    {
        for extension in self.map.values_mut() {
            f(extension.as_mut());
        }
    }

    pub fn remove<T: VersionControlledNativeExtension + TidAble<'a>>(&mut self) -> T {
        // can't use expect below because it requires `T: Debug`.
        match self
            .map
            .remove(&T::id())
            .expect("extension unknown")
            .downcast_box::<T>()
        {
            Ok(val) => *val,
            Err(_) => panic!("downcast error"),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::native_extensions::{NativeContextExtensions, VersionControlledNativeExtension};
    use better_any::{Tid, TidAble};

    #[derive(Tid)]
    struct Ext<'a> {
        a: &'a mut u64,
    }

    impl<'a> VersionControlledNativeExtension for Ext<'a> {
        fn undo(&mut self) {
            unimplemented!("Irrelevant for test")
        }

        fn save(&mut self) {
            unimplemented!("Irrelevant for test")
        }

        fn update(&mut self, _txn_hash: &[u8; 32], _script_hash: &[u8]) {
            unimplemented!("Irrelevant for test")
        }
    }

    #[test]
    fn non_static_ext() {
        let mut v: u64 = 23;
        let e = Ext { a: &mut v };
        let mut exts = NativeContextExtensions::default();
        exts.add(e);
        *exts.get_mut::<Ext>().a += 1;
        assert_eq!(*exts.get_mut::<Ext>().a, 24);
        *exts.get_mut::<Ext>().a += 1;
        let e1 = exts.remove::<Ext>();
        assert_eq!(*e1.a, 25)
    }
}
