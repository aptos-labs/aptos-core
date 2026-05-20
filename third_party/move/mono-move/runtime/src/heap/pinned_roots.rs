// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Pinned GC roots for temporary heap pointers.
//!
//! Some interpreter operations — notably multi-allocation micro-ops like
//! `PackClosure`, and (in the future) native functions — need to keep a
//! freshly allocated heap object alive across a subsequent allocation that
//! may trigger GC. Stack frames are the usual root set, but at the point of
//! the second allocation the first object may not yet be reachable from any
//! frame slot.
//!
//! [`PinnedRoots`] is an auxiliary root set that the GC scans alongside the
//! call stack. [`PinnedRoots::pin`] records a heap pointer and returns a
//! [`PinGuard`]; on drop the guard releases the slot. The GC updates the
//! slot in-place when it relocates the pinned object.
//!
//! # Properties
//!
//! * **Arbitrary drop order** — guards can be moved, dropped non-LIFO, or
//!   held in collections. Each guard has a stable slot index; drop
//!   frees that slot regardless of order. Freed slots are null and get
//!   reused on the next `pin`.
//!
//! * **Multiple pins coexist** — `pin` takes `&self` via interior mutability,
//!   so many guards can be live simultaneously without borrow conflicts.
//!
//! # Interior mutability: `UnsafeCell`
//!
//! Internal state lives in an [`UnsafeCell`] rather than a `RefCell`. The
//! public `pin` / `get` / drop APIs remain safe because each of them only
//! touches `inner` briefly (a single `&mut Inner` reborrow for the span of
//! the method body), and those spans never overlap in safe code —
//! single-threaded execution plus the fact that `PinnedRoots` is not
//! `Sync` rule out any racing access.
//!
//! The one place where an overlap *can* happen is [`update_in_place`]:
//! it takes a raw pointer into `inner.slots` and iterates through it,
//! calling a user-supplied callback on each entry. If that callback
//! touched `inner` mutably (via `pin` or by dropping a `PinGuard`), the
//! raw pointer could be invalidated by a `Vec` reallocation. The function
//! is therefore `unsafe`, with a contract forbidding the callback from
//! pinning or dropping guards. Because this dangerous window exists only
//! inside an `unsafe` block, the external `pin` / `get` / drop APIs stay
//! safe — safe code alone cannot construct the bad interleaving.

use std::{
    cell::UnsafeCell,
    ptr::{self, NonNull},
};

/// Initial capacity for the slots and free-list vectors. Chosen to cover
/// typical concurrent-pin counts (a fused micro-op pins 1-2; a native
/// function a handful) without reallocation. Exceeding this capacity is
/// fine — `Vec` will grow — but a sufficiently-sized initial reservation
/// avoids surprising mid-execution reallocations.
const INITIAL_CAPACITY: usize = 8;

// ---------------------------------------------------------------------------
// PinnedRoots
// ---------------------------------------------------------------------------

/// An auxiliary GC root set for heap pointers that must survive across
/// allocations without being rooted through a frame slot.
///
/// # Internal invariants
///
/// * `inner` is accessed through short-lived `&mut Inner` reborrows
///   produced by `unsafe { &mut *self.inner.get() }`. No such reborrow
///   outlives the body of the method that created it.
/// * The public `pin` / drop / `get` methods never hold a reborrow across
///   a call that could produce another reborrow. Since `PinnedRoots` is
///   `!Sync` and execution is single-threaded, safe callers cannot create
///   overlapping reborrows.
/// * [`update_in_place`] is the only API that takes a raw pointer into
///   the `slots` Vec's buffer and uses it after the `&mut Inner` reborrow
///   has ended. Its callback must not call `pin` or drop any `PinGuard`
///   belonging to this `PinnedRoots` — both would mutate `inner`, and a
///   `Vec` reallocation would invalidate the iteration pointer. This is
///   why `update_in_place` is `unsafe`; the constraint is a contract on
///   its caller.
pub struct PinnedRoots {
    inner: UnsafeCell<Inner>,
}

struct Inner {
    /// Active slots. A null entry means the slot is free (reusable).
    slots: Vec<*mut u8>,
    /// Indices of free slots, for O(1) reuse.
    free_list: Vec<usize>,
}

impl PinnedRoots {
    pub fn new() -> Self {
        Self {
            inner: UnsafeCell::new(Inner {
                slots: Vec::with_capacity(INITIAL_CAPACITY),
                free_list: Vec::with_capacity(INITIAL_CAPACITY),
            }),
        }
    }

    /// Pin a non-null heap pointer. The returned guard unpins on drop.
    ///
    /// The pinned pointer is treated as a GC root until the guard is
    /// dropped. If GC runs while the guard is alive, the slot is updated
    /// in-place and [`PinGuard::get`] returns the relocated pointer.
    pub fn pin(&self, ptr: NonNull<u8>) -> PinGuard<'_> {
        // SAFETY: the `&mut Inner` reborrow lives only for the body of this
        // function. No other reborrow of `inner` can be live at the same
        // time: (1) `PinnedRoots` is `!Sync` so no thread races with us;
        // (2) the other internal APIs (`get`, `Drop for PinGuard`) each do
        // the same short-lived reborrow pattern, non-overlapping in
        // single-threaded execution; (3) a call to this method from
        // inside `update_in_place`'s callback is forbidden by that
        // function's `unsafe` contract — safe code alone cannot construct
        // that interleaving.
        let inner = unsafe { &mut *self.inner.get() };
        let ptr = ptr.as_ptr();
        let idx = if let Some(idx) = inner.free_list.pop() {
            inner.slots[idx] = ptr;
            idx
        } else {
            inner.slots.push(ptr);
            inner.slots.len() - 1
        };
        PinGuard { roots: self, idx }
    }

    /// Apply `f` to every currently-pinned slot in-place. `f` receives the
    /// current pointer and returns the (possibly updated) replacement.
    ///
    /// **Intended for the garbage collector only.** Visibility is scoped
    /// to the parent `heap` module (`pub(super)`) to enforce that at the
    /// type system level. No other code in the crate may call this
    /// function; the GC is the sole client. If a new use case appears in
    /// the future, it should be added alongside the GC path rather than
    /// by widening the visibility here.
    ///
    /// # Safety
    ///
    /// `f` must not call [`PinnedRoots::pin`] or drop any [`PinGuard`]
    /// belonging to this `PinnedRoots`. Either would mutate `inner.slots`
    /// and, if it triggers a reallocation, invalidate the raw pointer we
    /// iterate with here — causing UB on subsequent reads/writes. The GC
    /// satisfies this contract because its scan neither pins new objects
    /// nor drops any guards.
    pub(super) unsafe fn update_in_place(&self, mut f: impl FnMut(*mut u8) -> *mut u8) {
        // SAFETY: We project through the `UnsafeCell`'s raw pointer
        // directly. Going via `&*self.inner.get()` first would derive a
        // read-only Stacked Borrows tag, and writing through a pointer
        // produced from `slots.as_ptr()` would then be UB. `as_mut_ptr`
        // on the `Vec` reached through `*mut Inner` keeps write-capable
        // provenance for the iteration below.
        let cell: *mut Inner = self.inner.get();
        let len = unsafe { (*cell).slots.len() };
        let slots_ptr: *mut *mut u8 = unsafe { (*cell).slots.as_mut_ptr() };
        for i in 0..len {
            // SAFETY: `slots_ptr` points into `(*cell).slots`'s
            // allocation, which is valid as long as the Vec has not been
            // reallocated. The caller's contract forbids `f` from doing
            // anything that could reallocate it, so the pointer remains
            // valid through this loop.
            unsafe {
                let slot = slots_ptr.add(i);
                let old = *slot;
                if !old.is_null() {
                    *slot = f(old);
                }
            }
        }
    }

    /// Number of currently pinned slots (including freed-but-not-reclaimed).
    /// For tests and debugging.
    #[cfg(test)]
    pub fn len(&self) -> usize {
        // SAFETY: short-lived `&Inner` reborrow, same argument as `pin`.
        unsafe { (*self.inner.get()).slots.len() }
    }

    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for PinnedRoots {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// PinGuard
// ---------------------------------------------------------------------------

/// RAII guard for a pinned heap pointer. Drop releases the slot.
///
/// Lifetime-bound to the [`PinnedRoots`] it was created from; multiple
/// guards can coexist because they share the outer immutable borrow and
/// mutation is routed through the inner [`UnsafeCell`].
pub struct PinGuard<'a> {
    roots: &'a PinnedRoots,
    idx: usize,
}

impl PinGuard<'_> {
    /// Read the current (possibly GC-relocated) value of the pinned pointer.
    pub fn get(&self) -> NonNull<u8> {
        // SAFETY: short-lived `&Inner` reborrow; see `PinnedRoots::pin` for
        // the full argument.
        let inner = unsafe { &*self.roots.inner.get() };
        let ptr = inner.slots[self.idx];
        // SAFETY: slot is written non-null by `pin` and only replaced with
        // a non-null relocation by the GC (via `update_in_place`); the
        // null sentinel is installed by `Drop` at the end of the guard's
        // lifetime, which is after the last possible call to `get`.
        unsafe { NonNull::new_unchecked(ptr) }
    }
}

impl Drop for PinGuard<'_> {
    fn drop(&mut self) {
        // SAFETY: short-lived `&mut Inner` reborrow; see `PinnedRoots::pin`.
        let inner = unsafe { &mut *self.roots.inner.get() };
        inner.slots[self.idx] = ptr::null_mut();
        inner.free_list.push(self.idx);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_ptr(addr: usize) -> NonNull<u8> {
        NonNull::new(addr as *mut u8).unwrap()
    }

    #[test]
    fn pin_returns_pointer_via_get() {
        let roots = PinnedRoots::new();
        let guard = roots.pin(fake_ptr(0x1000));
        assert_eq!(guard.get(), fake_ptr(0x1000));
    }

    #[test]
    fn drop_frees_slot_for_reuse() {
        let roots = PinnedRoots::new();
        {
            let _g = roots.pin(fake_ptr(0x1000));
            assert_eq!(roots.len(), 1);
        }
        // Slot is freed; next pin reuses the same slot.
        let _g2 = roots.pin(fake_ptr(0x2000));
        assert_eq!(roots.len(), 1);
    }

    #[test]
    fn multiple_pins_coexist() {
        let roots = PinnedRoots::new();
        let a = roots.pin(fake_ptr(0x1000));
        let b = roots.pin(fake_ptr(0x2000));
        let c = roots.pin(fake_ptr(0x3000));
        assert_eq!(a.get(), fake_ptr(0x1000));
        assert_eq!(b.get(), fake_ptr(0x2000));
        assert_eq!(c.get(), fake_ptr(0x3000));
    }

    #[test]
    fn drop_order_arbitrary() {
        let roots = PinnedRoots::new();
        let a = roots.pin(fake_ptr(0x1000));
        let b = roots.pin(fake_ptr(0x2000));
        let c = roots.pin(fake_ptr(0x3000));
        drop(a); // non-LIFO
        drop(c);
        // `b` still valid.
        assert_eq!(b.get(), fake_ptr(0x2000));
        drop(b);
    }

    #[test]
    fn update_in_place_relocates() {
        let roots = PinnedRoots::new();
        let g = roots.pin(fake_ptr(0x1000));
        // SAFETY: f doesn't pin or drop guards.
        unsafe {
            roots.update_in_place(|p| {
                if p == fake_ptr(0x1000).as_ptr() {
                    fake_ptr(0x9000).as_ptr()
                } else {
                    p
                }
            });
        }
        assert_eq!(g.get(), fake_ptr(0x9000));
    }

    #[test]
    fn update_in_place_skips_freed_slots() {
        let roots = PinnedRoots::new();
        let a = roots.pin(fake_ptr(0x1000));
        let b = roots.pin(fake_ptr(0x2000));
        drop(a);
        let mut visited = Vec::new();
        // SAFETY: f doesn't pin or drop guards.
        unsafe {
            roots.update_in_place(|p| {
                visited.push(p);
                p
            });
        }
        assert_eq!(visited, vec![fake_ptr(0x2000).as_ptr()]);
        drop(b);
    }
}
