// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{native::native_invariant_violation, VMResult};
use fxhash::FxHashMap;
use std::{
    any::{Any, TypeId},
    cell::{RefCell, RefMut},
};

/// An extensible state defined by a native function. Persisted throughout
/// a transaction.
pub trait NativeExtension: Any {
    /// Relocates the extension's GC roots in place.
    ///
    /// Must call `relocate` on every heap pointer the extension holds and write
    /// back the new address whenever one is returned. Missing a pointer leaves a
    /// dangling root after collection. Extensions that hold no heap pointers
    /// implement this as a no-op.
    ///
    /// # Safety
    ///
    /// `relocate` must not re-enter the heap or GC (e.g. by allocating), and
    /// must return a valid relocated address for each live pointer (or `None`).
    unsafe fn relocate_roots(&mut self, relocate: &mut dyn FnMut(*mut u8) -> Option<*mut u8>);

    /// Called when a checkpoint is taken (the start of a new sub-session). An
    /// extension with rollback-able state snapshots it here; one that the legacy
    /// VM reset on session start resets the same state. Extensions with no such
    /// state implement this as a no-op.
    fn on_checkpoint(&mut self);

    /// Rolls back the effects of the `n` most recent checkpoints. Extensions
    /// with no rollback-able state implement this as a no-op returning `Ok`.
    fn on_rollback(&mut self, n: usize) -> VMResult<()>;
}

/// Type-keyed collection of [`NativeExtension`]s available to native
/// functions during a transaction.
///
/// Specifically, disjoint borrows of extensions are allowed, as long as
/// there is at most one borrow of the same extension at a time.
/// This is currently ensured by storing each extension in its own [`RefCell`];
/// the extra indirection is unlikely to matter performance-wise.
#[derive(Default)]
pub struct NativeExtensions {
    map: FxHashMap<TypeId, RefCell<Box<dyn NativeExtension>>>,
}

impl NativeExtensions {
    pub fn new() -> Self {
        Self {
            map: FxHashMap::default(),
        }
    }

    /// Adds the extension to the collection.
    ///
    /// Panics if an extension of the same type is already present.
    pub fn add<T: NativeExtension>(&mut self, ext: T) {
        let prev = self
            .map
            .insert(TypeId::of::<T>(), RefCell::new(Box::new(ext)));
        assert!(
            prev.is_none(),
            "duplicate native extension of type {}",
            std::any::type_name::<T>(),
        );
    }

    /// Gets a mutable access to the extension of type `T`.
    ///
    /// Errors if `T` is not installed, or if it is already borrowed.
    pub fn get_mut<T: NativeExtension>(&self) -> VMResult<RefMut<'_, T>> {
        let cell = self.map.get(&TypeId::of::<T>()).ok_or_else(|| {
            native_invariant_violation(format!(
                "native extension {} not installed for this transaction",
                std::any::type_name::<T>(),
            ))
        })?;
        let guard = cell.try_borrow_mut().map_err(|_| {
            native_invariant_violation(format!(
                "native extension {} is already borrowed",
                std::any::type_name::<T>(),
            ))
        })?;
        Ok(RefMut::map(guard, |ext| {
            // Upcast `dyn NativeExtension` to `dyn Any`, then downcast to `T`.
            let ext: &mut dyn Any = &mut **ext;
            ext.downcast_mut::<T>()
                .expect("TypeId key matches the stored extension type")
        }))
    }

    /// Relocate every extension's GC roots during collection.
    ///
    /// # Safety
    ///
    /// Same contract as [`NativeExtension::relocate_roots`]: `relocate` must not
    /// re-enter the heap/GC and must return valid relocated addresses.
    ///
    /// TODO(correctness): this currently has one major issue -- if this is called while a native extension is
    /// borrowed, it will error. Figure out how we can guarantee exclusive access to the pointers
    /// safely during GC.
    pub unsafe fn relocate_all_roots(
        &self,
        relocate: &mut dyn FnMut(*mut u8) -> Option<*mut u8>,
    ) -> VMResult<()> {
        for ext in self.map.values() {
            let mut ext = ext.try_borrow_mut().map_err(|_| {
                native_invariant_violation(
                    "a native extension is borrowed during GC (held across an allocation?)".into(),
                )
            })?;
            // SAFETY: forwarded from this function's contract.
            unsafe { ext.relocate_roots(&mut *relocate) };
        }
        Ok(())
    }

    /// Signals a checkpoint to every extension.
    pub fn checkpoint(&self) -> VMResult<()> {
        for ext in self.map.values() {
            ext.try_borrow_mut()
                .map_err(|_| {
                    native_invariant_violation(
                        "cannot checkpoint: a native extension is unexpectedly still borrowed"
                            .into(),
                    )
                })?
                .on_checkpoint();
        }
        Ok(())
    }

    /// Rolls back the `n` most recent checkpoints across every extension.
    /// `n == 0` is a no-op, so each `on_rollback` only ever sees `n >= 1`.
    pub fn rollback(&self, n: usize) -> VMResult<()> {
        if n == 0 {
            return Ok(());
        }
        for ext in self.map.values() {
            ext.try_borrow_mut()
                .map_err(|_| {
                    native_invariant_violation(
                        "cannot roll back: a native extension is unexpectedly still borrowed"
                            .into(),
                    )
                })?
                .on_rollback(n)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Counter(u64);
    impl NativeExtension for Counter {
        unsafe fn relocate_roots(&mut self, _relocate: &mut dyn FnMut(*mut u8) -> Option<*mut u8>) {
        }

        fn on_checkpoint(&mut self) {}

        fn on_rollback(&mut self, _n: usize) -> VMResult<()> {
            Ok(())
        }
    }

    struct Flag(bool);
    impl NativeExtension for Flag {
        unsafe fn relocate_roots(&mut self, _relocate: &mut dyn FnMut(*mut u8) -> Option<*mut u8>) {
        }

        fn on_checkpoint(&mut self) {}

        fn on_rollback(&mut self, _n: usize) -> VMResult<()> {
            Ok(())
        }
    }

    #[test]
    fn add_and_get_mut_by_type() {
        let mut exts = NativeExtensions::new();
        exts.add(Counter(0));
        exts.add(Flag(false));

        exts.get_mut::<Counter>().unwrap().0 += 5;
        exts.get_mut::<Flag>().unwrap().0 = true;

        assert_eq!(exts.get_mut::<Counter>().unwrap().0, 5);
        assert!(exts.get_mut::<Flag>().unwrap().0);
    }

    #[test]
    fn get_mut_missing_errors() {
        let exts = NativeExtensions::new();
        assert!(exts.get_mut::<Counter>().is_err());
    }

    #[test]
    fn distinct_extensions_borrow_simultaneously() {
        let mut exts = NativeExtensions::new();
        exts.add(Counter(0));
        exts.add(Flag(false));

        // Holding two distinct extensions at once is allowed.
        let mut counter = exts.get_mut::<Counter>().unwrap();
        let mut flag = exts.get_mut::<Flag>().unwrap();
        counter.0 += 1;
        flag.0 = true;
        assert_eq!(counter.0, 1);
        assert!(flag.0);
    }

    #[test]
    fn same_extension_double_borrow_errors() {
        let mut exts = NativeExtensions::new();
        exts.add(Counter(0));
        let _first = exts.get_mut::<Counter>().unwrap();
        // Second borrow while the first is live errors rather than panicking.
        assert!(exts.get_mut::<Counter>().is_err());
    }

    #[test]
    #[should_panic(expected = "duplicate native extension")]
    fn duplicate_add_panics() {
        let mut exts = NativeExtensions::new();
        exts.add(Counter(1));
        exts.add(Counter(2));
    }

    /// Extension whose value is checkpointed on each new sub-session and
    /// restored on rollback.
    #[derive(Default)]
    struct Checkpointed {
        value: u64,
        snapshots: Vec<u64>,
    }
    impl NativeExtension for Checkpointed {
        unsafe fn relocate_roots(&mut self, _relocate: &mut dyn FnMut(*mut u8) -> Option<*mut u8>) {
        }

        fn on_checkpoint(&mut self) {
            self.snapshots.push(self.value);
        }

        fn on_rollback(&mut self, n: usize) -> VMResult<()> {
            let keep = self.snapshots.len().checked_sub(n).ok_or_else(|| {
                native_invariant_violation("rollback past the first checkpoint".into())
            })?;
            self.value = self.snapshots[keep];
            self.snapshots.truncate(keep);
            Ok(())
        }
    }

    #[test]
    fn checkpoint_and_rollback_hooks() {
        let mut exts = NativeExtensions::new();
        exts.add(Checkpointed::default());

        exts.checkpoint().unwrap(); // snapshot value = 0
        exts.get_mut::<Checkpointed>().unwrap().value = 10;
        exts.checkpoint().unwrap(); // snapshot value = 10
        exts.get_mut::<Checkpointed>().unwrap().value = 20;

        exts.rollback(1).unwrap();
        assert_eq!(exts.get_mut::<Checkpointed>().unwrap().value, 10);

        exts.rollback(1).unwrap();
        assert_eq!(exts.get_mut::<Checkpointed>().unwrap().value, 0);

        // Nothing left to roll back.
        assert!(exts.rollback(1).is_err());
    }
}
