// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use smallvec::SmallVec;
use std::{cell::UnsafeCell, ptr};

/// A rooted pointer the GC tracks.
#[derive(Clone, Copy)]
struct Entry {
    base: *mut u8,
    /// Byte offset into `base`; `0` for a direct object pointer.
    offset: u64,
}

/// Pool of GC roots for pointers that must survive a GC without being reachable
/// from a frame slot. Needed by micro-ops and native functions that allocate
/// more than once.
///
/// A root is handed out as an RAII handle ([`ReferenceHandle`] / [`ObjectHandle`])
/// that frees its slot on drop; freed slots are recycled through a free list.
///
/// # Interior mutability & safety invariants
///
/// All public methods take `&self`, with the backing store behind a single
/// `UnsafeCell`, so multiple handles (each borrowing the pool) can coexist.
/// Soundness relies on
/// 1. the pool being single-threaded (`!Sync`), and
/// 2. every method holding at most one borrow of the inner store at a time, never
///    across a call that could reborrow it. [`RootPool::relocate_each`]
///    additionally requires the caller's closure not to re-enter the pool.
pub struct RootPool {
    inner: UnsafeCell<RootPoolInner>,
}

const INLINE_CAPACITY: usize = 16;

/// Backing store of a [`RootPool`].
struct RootPoolInner {
    entries: SmallVec<[Entry; INLINE_CAPACITY]>,
    /// Indices of freed slots, recycled by the next root.
    free: SmallVec<[usize; INLINE_CAPACITY]>,
}

impl RootPool {
    pub fn new() -> Self {
        Self {
            inner: UnsafeCell::new(RootPoolInner {
                entries: SmallVec::new(),
                free: SmallVec::new(),
            }),
        }
    }

    /// Roots the Move reference `base + offset`.
    /// Returns a handle that can be used to access the reference safely across GC, and
    /// and frees its slot on drop.
    ///
    /// # Safety
    ///
    /// (base, offset) must be a valid Move reference (fat pointer) pointing to a
    /// Move value.
    ///
    /// A null or non-heap `base` is technically sound but effectively tracks nothing.
    pub unsafe fn root_reference(&self, base: *mut u8, offset: u64) -> ReferenceHandle<'_> {
        ReferenceHandle {
            pool: self,
            idx: self.alloc(Entry { base, offset }),
        }
    }

    /// Roots a heap object pointer.
    /// Returns a handle that can be used to access the object safely across GC, and
    /// and frees its slot on drop.
    ///
    /// # Safety
    ///
    /// `ptr` must be a valid data pointer of a live heap object.
    ///
    /// A null or non-heap `ptr` is technically sound but tracks nothing. This is
    /// relied upon for empty vectors, which are represented as a null data pointer.
    //
    // TODO(cleanup): every object gets a slot, including empty vectors (a null pointer),
    // for a uniform representation. We could reserve a placeholder (e.g.
    // `usize::MAX`) for "nothing to track", but the exact rule needs thought —
    // exclude only null, or all non-heap pointers?
    pub unsafe fn root_object(&self, ptr: *mut u8) -> ObjectHandle<'_> {
        ObjectHandle {
            pool: self,
            idx: self.alloc(Entry {
                base: ptr,
                offset: 0,
            }),
        }
    }

    /// Relocate every entry's base through `relocate` (which returns the new
    /// address, or `None` to leave it). Invoked by the GC while scanning roots.
    ///
    /// # Safety
    ///
    /// `relocate` is responsible for deciding what to relocate: it returns the
    /// new address for a live heap object, or `None` to skip — including for the
    /// null bases of freed slots and empty vectors, and for non-heap pointers.
    /// Every entry is therefore passed through unconditionally.
    ///
    /// It must also guarantee that it does NOT call any other methods on this pool,
    /// as that kind of re-entrancy breaks interior mutability -- only one mutable
    /// borrow is allowed at a time.
    pub unsafe fn relocate_each(&self, mut relocate: impl FnMut(*mut u8) -> Option<*mut u8>) {
        // SAFETY: short, exclusive reborrow; the caller guarantees `relocate`
        // does not re-enter, and GC is stop-the-world, so no handle read overlaps.
        let inner = unsafe { &mut *self.inner.get() };
        for entry in inner.entries.iter_mut() {
            if let Some(new) = relocate(entry.base) {
                entry.base = new;
            }
        }
    }

    /// Allocates a slot for `entry`, recycling a freed index when one exists.
    fn alloc(&self, entry: Entry) -> usize {
        // SAFETY: short, exclusive reborrow; single-threaded, leaf call.
        let inner = unsafe { &mut *self.inner.get() };
        match inner.free.pop() {
            Some(idx) => {
                inner.entries[idx] = entry;
                idx
            },
            None => {
                inner.entries.push(entry);
                inner.entries.len() - 1
            },
        }
    }

    /// Frees slot at `idx`.
    /// The base needs to be nulled so the GC skips it.
    fn release(&self, idx: usize) {
        // SAFETY: short, exclusive reborrow; single-threaded.
        let inner = unsafe { &mut *self.inner.get() };
        inner.entries[idx].base = ptr::null_mut();
        inner.free.push(idx);
    }

    fn entry(&self, idx: usize) -> Entry {
        // SAFETY: short shared reborrow; `idx` is a live slot and `Entry: Copy`.
        let inner = unsafe { &*self.inner.get() };
        inner.entries[idx]
    }

    /// Number of slots ever allocated (live + freed-but-not-reclaimed). For tests.
    #[cfg(test)]
    fn slot_count(&self) -> usize {
        // SAFETY: short shared reborrow; single-threaded.
        unsafe { &*self.inner.get() }.entries.len()
    }
}

impl Default for RootPool {
    fn default() -> Self {
        Self::new()
    }
}

/// Handle to a Move reference (`&T` / `&mut T`) rooted in a [`RootPool`].
///
/// Kept current by the GC (while the pool is scanned); frees its slot on drop.
pub struct ReferenceHandle<'a> {
    pool: &'a RootPool,
    idx: usize,
}

impl<'a> ReferenceHandle<'a> {
    /// The pool this handle is rooted in.
    pub(crate) fn pool(&self) -> &'a RootPool {
        self.pool
    }

    /// Reads the current pointer to the referenced value.
    ///
    /// # Safety
    ///
    /// The returned pointer is invalidated by any GC; the caller must use it only
    /// transiently, before any allocation that could trigger collection.
    pub fn ptr(&self) -> *mut u8 {
        let entry = self.pool.entry(self.idx);
        // SAFETY: `base` is a live (possibly relocated) pointer and `offset`
        // lies within the referenced allocation.
        unsafe { entry.base.add(entry.offset as usize) }
    }

    /// Returns the raw `(base, offset)` fat pointer.
    pub fn fat(&self) -> (*mut u8, u64) {
        let entry = self.pool.entry(self.idx);
        (entry.base, entry.offset)
    }
}

impl Drop for ReferenceHandle<'_> {
    fn drop(&mut self) {
        self.pool.release(self.idx);
    }
}

/// Handle to a heap object rooted in a [`RootPool`].
///
/// Kept current by the GC (while the pool is scanned); frees its slot on drop.
pub struct ObjectHandle<'a> {
    pool: &'a RootPool,
    idx: usize,
}

impl ObjectHandle<'_> {
    /// Reads the current pointer to the object.
    ///
    /// # Safety
    ///
    /// The returned pointer is invalidated by any GC; the caller must use it only
    /// transiently, before any allocation that could trigger collection.
    pub fn ptr(&self) -> *mut u8 {
        self.pool.entry(self.idx).base
    }
}

impl Drop for ObjectHandle<'_> {
    fn drop(&mut self) {
        self.pool.release(self.idx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(addr: usize) -> *mut u8 {
        addr as *mut u8
    }

    // The pointers below are never dereferenced, so rooting and relocating them
    // is sound despite being fabricated.

    #[test]
    fn object_handle_reads_back() {
        let pool = RootPool::new();
        let h = unsafe { pool.root_object(p(0x1000)) };
        assert_eq!(h.ptr(), p(0x1000));
    }

    #[test]
    fn reference_handle_adds_offset() {
        let pool = RootPool::new();
        let h = unsafe { pool.root_reference(p(0x1000), 24) };
        assert_eq!(h.ptr(), p(0x1000 + 24));
        assert_eq!(h.fat(), (p(0x1000), 24));
    }

    #[test]
    fn handles_coexist() {
        let pool = RootPool::new();
        let a = unsafe { pool.root_object(p(0x1000)) };
        let b = unsafe { pool.root_reference(p(0x2000), 8) };
        let c = unsafe { pool.root_object(p(0x3000)) };
        assert_eq!(a.ptr(), p(0x1000));
        assert_eq!(b.ptr(), p(0x2008));
        assert_eq!(c.ptr(), p(0x3000));
    }

    #[test]
    fn relocate_updates_live_handles() {
        let pool = RootPool::new();
        let a = unsafe { pool.root_object(p(0x1000)) };
        let b = unsafe { pool.root_reference(p(0x2000), 16) };
        // Simulate a GC that moves 0x1000 -> 0x5000 and 0x2000 -> 0x6000.
        unsafe {
            pool.relocate_each(|base| match base as usize {
                0x1000 => Some(p(0x5000)),
                0x2000 => Some(p(0x6000)),
                _ => None,
            })
        };
        assert_eq!(a.ptr(), p(0x5000));
        assert_eq!(b.ptr(), p(0x6000 + 16));
    }

    #[test]
    fn dropped_handle_recycles_its_slot() {
        let pool = RootPool::new();
        {
            let _h = unsafe { pool.root_object(p(0x1000)) };
            assert_eq!(pool.slot_count(), 1);
        }
        // The slot is freed on drop and reused by the next root.
        let _h2 = unsafe { pool.root_object(p(0x2000)) };
        assert_eq!(pool.slot_count(), 1);
    }

    #[test]
    fn relocate_offers_every_slot() {
        let pool = RootPool::new();
        let a = unsafe { pool.root_object(p(0x1000)) };
        drop(unsafe { pool.root_object(p(0x2000)) });
        let mut seen = Vec::new();
        unsafe {
            pool.relocate_each(|base| {
                seen.push(base);
                None
            })
        };
        // Every entry is offered to `relocate`, including the freed slot (now a
        // null base) — skipping null/non-heap bases is `relocate`'s job.
        assert_eq!(seen, vec![p(0x1000), p(0)]);
        assert_eq!(a.ptr(), p(0x1000));
    }
}
