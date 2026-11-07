// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! [`MemAddr`]: a raw address into VM memory.

use crate::{align::MAX_ALIGN, scalar::Scalar};
use std::{fmt, mem::MaybeUninit, ptr, ptr::NonNull};

/// A raw address into VM memory. Every raw memory access in the VM goes
/// through this type.
#[repr(transparent)]
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MemAddr(*mut u8);

impl fmt::Debug for MemAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MemAddr({:#x})", self.0.addr())
    }
}

// ---------------------------------------------------------------------------
// Construction and inspection
// ---------------------------------------------------------------------------

impl MemAddr {
    /// The null address.
    pub const NULL: MemAddr = MemAddr(ptr::null_mut());

    /// Wraps a raw pointer. Used at allocator and FFI boundaries; VM code
    /// receives addresses rather than making them.
    #[inline(always)]
    pub const fn from_ptr(p: *mut u8) -> MemAddr {
        MemAddr(p)
    }

    /// The address of a `T` behind a typed pointer: a code pointer (a
    /// `Function` in the global arena) about to be stored in a VM word.
    #[inline(always)]
    pub const fn from_typed_ptr<T>(p: *const T) -> MemAddr {
        MemAddr(p.cast_mut().cast())
    }

    /// [`MemAddr::from_typed_ptr`] for a [`NonNull`].
    #[inline(always)]
    pub const fn from_typed_non_null<T>(p: NonNull<T>) -> MemAddr {
        MemAddr(p.as_ptr().cast())
    }

    /// Wraps a raw `*const` pointer.
    #[inline(always)]
    pub const fn from_const_ptr(p: *const u8) -> MemAddr {
        MemAddr(p.cast_mut())
    }

    /// The address of a borrowed Rust value, for reading it through the
    /// VM memory API. The address must not be written through while the
    /// borrow is live.
    #[inline(always)]
    pub fn from_ref<T: ?Sized>(r: &T) -> MemAddr {
        MemAddr(ptr::from_ref(r).cast::<u8>().cast_mut())
    }

    /// The address of a mutably borrowed Rust value, for reading or writing
    /// it through the VM memory API.
    #[inline(always)]
    pub fn from_mut<T: ?Sized>(r: &mut T) -> MemAddr {
        MemAddr(ptr::from_mut(r).cast::<u8>())
    }

    /// Views the address as a `&T`. There is deliberately no method that
    /// yields a raw byte pointer.
    ///
    /// # Safety
    ///
    /// `self` must point at a live, initialized, aligned `T` that is not
    /// mutated for `'a`.
    #[inline(always)]
    pub unsafe fn as_ref<'a, T>(self) -> &'a T {
        // SAFETY: forwarded to the caller.
        unsafe { &*self.0.cast::<T>() }
    }

    /// The address as an integer, for comparisons and bounds. There is no
    /// inverse: an address is never rebuilt from an integer.
    #[inline(always)]
    pub fn as_usize(self) -> usize {
        self.0.addr()
    }

    #[inline(always)]
    pub fn is_null(self) -> bool {
        self.0.is_null()
    }

    /// Whether the address is a multiple of `align` (a power of two).
    #[inline(always)]
    pub fn is_aligned_to(self, align: usize) -> bool {
        debug_assert!(align.is_power_of_two());
        self.as_usize() & (align - 1) == 0
    }
}

// ---------------------------------------------------------------------------
// Address formation
// ---------------------------------------------------------------------------

impl MemAddr {
    /// `self + bytes`.
    ///
    /// # Safety
    ///
    /// Same as `<*mut u8>::add`: the result must stay within, or one past
    /// the end of, the same allocation.
    #[inline(always)]
    pub unsafe fn add(self, bytes: usize) -> MemAddr {
        // SAFETY: forwarded to the caller.
        MemAddr(unsafe { self.0.add(bytes) })
    }

    /// `self - bytes`.
    ///
    /// # Safety
    ///
    /// Same as `<*mut u8>::sub`: the result must stay within, or one past
    /// the end of, the same allocation.
    #[inline(always)]
    pub unsafe fn sub(self, bytes: usize) -> MemAddr {
        // SAFETY: forwarded to the caller.
        MemAddr(unsafe { self.0.sub(bytes) })
    }

    /// `self + bytes` without the in-bounds requirement of [`MemAddr::add`].
    /// For computing bounds that are compared but never dereferenced, such
    /// as a region's one-past-the-end address.
    #[inline(always)]
    pub fn wrapping_add(self, bytes: usize) -> MemAddr {
        MemAddr(self.0.wrapping_add(bytes))
    }

    /// `self - bytes` without the in-bounds requirement; see [`MemAddr::wrapping_add`].
    #[inline(always)]
    pub fn wrapping_sub(self, bytes: usize) -> MemAddr {
        MemAddr(self.0.wrapping_sub(bytes))
    }

    /// The number of bytes from `origin` to `self`.
    ///
    /// # Safety
    ///
    /// Both must be within the same allocation, with `self >= origin`.
    #[inline(always)]
    pub unsafe fn offset_from(self, origin: MemAddr) -> usize {
        debug_assert!(self >= origin);
        // SAFETY: forwarded to the caller; `self >= origin` makes the result
        // non-negative.
        unsafe { self.0.offset_from(origin.0) as usize }
    }
}

// ---------------------------------------------------------------------------
// Typed access
// ---------------------------------------------------------------------------

/// Types whose Rust alignment exceeds [`MAX_ALIGN`] are stored at
/// `MAX_ALIGN` alignment and accessed unaligned.
#[inline(always)]
const fn aligned_access<T: Scalar>() -> bool {
    T::ALIGN <= MAX_ALIGN
}

impl MemAddr {
    /// Loads a `T`. Aligned when `T::ALIGN <= MAX_ALIGN`, unaligned otherwise.
    ///
    /// # Safety
    ///
    /// The bytes must be readable and initialized, aligned to `T::ALIGN` when
    /// that is within [`MAX_ALIGN`], and not a pointer slot (see
    /// [`MemAddr::load_ptr`]).
    #[inline(always)]
    pub unsafe fn load<T: Scalar>(self) -> T {
        let p = self.0.cast::<T>();
        // SAFETY: forwarded to the caller; the alignment branch is a
        // compile-time constant.
        let v = unsafe {
            if aligned_access::<T>() {
                p.read()
            } else {
                p.read_unaligned()
            }
        };
        T::from_le(v)
    }

    /// Stores a `T`. Aligned when `T::ALIGN <= MAX_ALIGN`, unaligned otherwise.
    ///
    /// # Safety
    ///
    /// The bytes must be writable and aligned to `T::ALIGN` when that is
    /// within [`MAX_ALIGN`].
    #[inline(always)]
    pub unsafe fn store<T: Scalar>(self, v: T) {
        let p = self.0.cast::<T>();
        let v = T::to_le(v);
        // SAFETY: forwarded to the caller; the alignment branch is a
        // compile-time constant.
        unsafe {
            if aligned_access::<T>() {
                p.write(v)
            } else {
                p.write_unaligned(v)
            }
        }
    }

    /// Loads a `T` with no alignment requirement.
    ///
    /// # Safety
    ///
    /// The bytes must be readable and initialized, and not a pointer slot.
    #[inline(always)]
    pub unsafe fn load_unaligned<T: Scalar>(self) -> T {
        // SAFETY: forwarded to the caller.
        T::from_le(unsafe { self.0.cast::<T>().read_unaligned() })
    }

    /// Stores a `T` with no alignment requirement.
    ///
    /// # Safety
    ///
    /// The bytes must be writable.
    #[inline(always)]
    pub unsafe fn store_unaligned<T: Scalar>(self, v: T) {
        // SAFETY: forwarded to the caller.
        unsafe { self.0.cast::<T>().write_unaligned(T::to_le(v)) }
    }

    /// Loads a pointer slot. Pointers must be loaded this way, not as
    /// integers, so they keep their provenance.
    ///
    /// # Safety
    ///
    /// The word must be readable, initialized, and pointer-aligned.
    #[inline(always)]
    pub unsafe fn load_ptr(self) -> MemAddr {
        // SAFETY: forwarded to the caller.
        MemAddr(unsafe { self.0.cast::<*mut u8>().read() })
    }

    /// Stores into a pointer slot; see [`MemAddr::load_ptr`].
    ///
    /// # Safety
    ///
    /// The word must be writable and pointer-aligned.
    #[inline(always)]
    pub unsafe fn store_ptr(self, p: MemAddr) {
        // SAFETY: forwarded to the caller.
        unsafe { self.0.cast::<*mut u8>().write(p.0) }
    }

    /// Loads a boolean. A boolean slot holds `0` or `1`: every writer
    /// (compiler immediates, [`MemAddr::store_bool`], BCS deserialization)
    /// establishes that, and this is checked in debug builds.
    ///
    /// # Safety
    ///
    /// The byte must be readable and initialized.
    #[inline(always)]
    pub unsafe fn load_bool(self) -> bool {
        // SAFETY: forwarded to the caller.
        let byte = unsafe { self.load::<u8>() };
        debug_assert!(byte <= 1, "boolean slot holds non-boolean byte {byte}");
        byte != 0
    }

    /// Stores a boolean as `0` or `1`.
    ///
    /// # Safety
    ///
    /// The byte must be writable.
    #[inline(always)]
    pub unsafe fn store_bool(self, v: bool) {
        // SAFETY: forwarded to the caller.
        unsafe { self.store::<u8>(v as u8) }
    }
}

// ---------------------------------------------------------------------------
// Bulk operations
// ---------------------------------------------------------------------------

impl MemAddr {
    /// Copies one 8-byte word to `dst`, preserving provenance and
    /// uninitialized bytes. The word may be a pointer, a scalar, or padding;
    /// alignment and overlap are both allowed.
    ///
    /// # Safety
    ///
    /// `self` must be readable and `dst` writable for 8 bytes.
    #[inline(always)]
    pub unsafe fn copy_word(self, dst: MemAddr) {
        // SAFETY: forwarded to the caller. Loading all eight bytes before
        // writing them permits overlap.
        unsafe {
            let bytes = self.0.cast::<MaybeUninit<u64>>().read_unaligned();
            dst.0.cast::<MaybeUninit<u64>>().write_unaligned(bytes);
        }
    }

    /// `memcpy`: copies `n` bytes to `dst`; the ranges must not overlap.
    ///
    /// # Safety
    ///
    /// `self` must be readable and `dst` writable for `n` bytes, and the
    /// ranges must not overlap.
    #[inline(always)]
    pub unsafe fn copy_to_nonoverlapping(self, dst: MemAddr, n: usize) {
        debug_assert!(
            self.as_usize().abs_diff(dst.as_usize()) >= n,
            "copy_to_nonoverlapping ranges overlap"
        );
        // SAFETY: forwarded to the caller.
        unsafe { ptr::copy_nonoverlapping(self.0, dst.0, n) }
    }

    /// `memcpy` from a borrowed Rust slice.
    ///
    /// # Safety
    ///
    /// `self` must be writable for `src.len()` bytes and must not overlap
    /// `src`; a `&[u8]` live across the call already forbids writes to its
    /// bytes, so this holds whenever `src` is a valid borrow.
    #[inline(always)]
    pub unsafe fn copy_from_slice(self, src: &[u8]) {
        // SAFETY: forwarded to the caller for the destination; `src` is
        // readable for its length as a live borrow.
        unsafe { MemAddr::from_ref(src).copy_to_nonoverlapping(self, src.len()) }
    }

    /// `memmove`: copies `n` bytes to `dst`; the ranges may overlap.
    ///
    /// # Safety
    ///
    /// `self` must be readable and `dst` writable for `n` bytes.
    #[inline(always)]
    pub unsafe fn copy_to(self, dst: MemAddr, n: usize) {
        // SAFETY: forwarded to the caller.
        unsafe { ptr::copy(self.0, dst.0, n) }
    }

    /// Swaps `n` bytes with `other`; the ranges must not overlap.
    ///
    /// # Safety
    ///
    /// Both ranges must be readable and writable for `n` bytes, and must not
    /// overlap.
    #[inline(always)]
    pub unsafe fn swap_nonoverlapping_with(self, other: MemAddr, n: usize) {
        debug_assert!(
            self.as_usize().abs_diff(other.as_usize()) >= n,
            "swap_nonoverlapping_with ranges overlap"
        );
        // SAFETY: forwarded to the caller.
        unsafe { ptr::swap_nonoverlapping(self.0, other.0, n) }
    }

    /// Writes `n` zero bytes.
    ///
    /// # Safety
    ///
    /// `self` must be writable for `n` bytes.
    #[inline(always)]
    pub unsafe fn zero(self, n: usize) {
        // SAFETY: forwarded to the caller.
        unsafe { self.fill(0, n) }
    }

    /// Writes `n` copies of `byte`.
    ///
    /// # Safety
    ///
    /// `self` must be writable for `n` bytes.
    #[inline(always)]
    pub unsafe fn fill(self, byte: u8, n: usize) {
        // SAFETY: forwarded to the caller.
        unsafe { ptr::write_bytes(self.0, byte, n) }
    }
}

// ---------------------------------------------------------------------------
// Slice views
// ---------------------------------------------------------------------------

impl MemAddr {
    /// Views `n` initialized, pointer-free bytes as a slice.
    ///
    /// # Safety
    ///
    /// The bytes must be readable, initialized, free of pointer slots, and not
    /// written through any other path for `'a`.
    #[inline(always)]
    pub unsafe fn as_bytes<'a>(self, n: usize) -> &'a [u8] {
        // SAFETY: forwarded to the caller.
        unsafe { std::slice::from_raw_parts(self.0, n) }
    }

    /// Mutable variant of [`MemAddr::as_bytes`].
    ///
    /// # Safety
    ///
    /// As [`MemAddr::as_bytes`], plus writable and not accessed through any
    /// other path for `'a`.
    #[inline(always)]
    pub unsafe fn as_bytes_mut<'a>(self, n: usize) -> &'a mut [u8] {
        // SAFETY: forwarded to the caller.
        unsafe { std::slice::from_raw_parts_mut(self.0, n) }
    }

    /// Views `n` bytes that may be uninitialized or hold pointers.
    ///
    /// # Safety
    ///
    /// The bytes must be readable and not written through any other path
    /// for `'a`.
    #[inline(always)]
    pub unsafe fn as_uninit_bytes<'a>(self, n: usize) -> &'a [MaybeUninit<u8>] {
        // SAFETY: forwarded to the caller; `MaybeUninit<u8>` permits any byte.
        unsafe { std::slice::from_raw_parts(self.0.cast(), n) }
    }

    /// Mutable variant of [`MemAddr::as_uninit_bytes`].
    ///
    /// # Safety
    ///
    /// As [`MemAddr::as_uninit_bytes`], plus writable and not accessed
    /// through any other path for `'a`.
    #[inline(always)]
    pub unsafe fn as_uninit_bytes_mut<'a>(self, n: usize) -> &'a mut [MaybeUninit<u8>] {
        // SAFETY: forwarded to the caller; `MaybeUninit<u8>` permits any byte.
        unsafe { std::slice::from_raw_parts_mut(self.0.cast(), n) }
    }
}

// SAFETY: a `MemAddr` is a bare pointer; every dereference is `unsafe` with
// its own justification, so sharing the value itself is sound.
unsafe impl Send for MemAddr {}
unsafe impl Sync for MemAddr {}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `MAX_ALIGN`-aligned byte buffer, as every VM region is.
    #[repr(C, align(8))]
    struct Buf<const N: usize>([u8; N]);

    impl<const N: usize> Buf<N> {
        fn zeroed() -> Self {
            Buf([0; N])
        }

        fn addr(&mut self) -> MemAddr {
            MemAddr::from_ptr(self.0.as_mut_ptr())
        }
    }

    #[test]
    fn round_trip_scalars() {
        let mut buf = Buf::<64>::zeroed();
        let base = buf.addr();
        unsafe {
            base.store::<u64>(0x0102_0304_0506_0708);
            assert_eq!(base.load::<u64>(), 0x0102_0304_0506_0708);
            // Little-endian in memory regardless of host.
            assert_eq!(base.load::<u8>(), 0x08);
            assert_eq!(base.add(7).load::<u8>(), 0x01);

            base.add(8).store::<u128>(u128::MAX - 1);
            assert_eq!(base.add(8).load::<u128>(), u128::MAX - 1);

            base.add(1).store_unaligned::<u32>(0xDEAD_BEEF);
            assert_eq!(base.add(1).load_unaligned::<u32>(), 0xDEAD_BEEF);

            base.add(40).store_bool(true);
            assert!(base.add(40).load_bool());
            base.add(40).store_bool(false);
            assert!(!base.add(40).load_bool());

            base.add(48).store::<[u8; 3]>([1, 2, 3]);
            assert_eq!(base.add(48).load::<[u8; 3]>(), [1, 2, 3]);
        }
    }

    #[test]
    fn pointer_slots_keep_provenance() {
        let mut target = 7u64;
        let mut slot = Buf::<8>::zeroed();
        let p = MemAddr::from_mut(&mut target);
        let s = slot.addr();
        unsafe {
            s.store_ptr(p);
            let q = s.load_ptr();
            assert_eq!(q, p);
            q.store::<u64>(9);
        }
        assert_eq!(target, 9);
    }

    #[test]
    fn copies() {
        // Initialize before taking the address: under Stacked Borrows a later
        // `&mut buf` reborrow would invalidate `base`, exactly as it would in
        // the VM if a region were re-borrowed after handing out its base.
        let mut buf = Buf(std::array::from_fn::<u8, 32, _>(|i| i as u8));
        let base = buf.addr();
        // All reads go through `base` too: a `&buf[..]` reborrow would
        // retag the allocation under Stacked Borrows.
        let bytes = |at: usize, n: usize| unsafe { base.add(at).as_bytes(n) };
        unsafe {
            base.copy_to_nonoverlapping(base.add(16), 8);
            assert_eq!(bytes(16, 8), &[0, 1, 2, 3, 4, 5, 6, 7]);
            // Overlapping move.
            base.copy_to(base.add(4), 8);
            assert_eq!(bytes(4, 8), &[0, 1, 2, 3, 4, 5, 6, 7]);
            base.add(24).copy_word(base);
            assert_eq!(bytes(0, 8), &[24, 25, 26, 27, 28, 29, 30, 31]);
            base.swap_nonoverlapping_with(base.add(16), 8);
            assert_eq!(bytes(16, 8), &[24, 25, 26, 27, 28, 29, 30, 31]);
            base.zero(4);
            assert_eq!(bytes(0, 4), &[0, 0, 0, 0]);
            base.fill(0xAA, 2);
            assert_eq!(bytes(0, 2), &[0xAA, 0xAA]);
        }
    }

    #[test]
    fn arithmetic_and_bounds() {
        let mut buf = Buf::<16>::zeroed();
        let base = buf.addr();
        let end = base.wrapping_add(16);
        assert_eq!(end.as_usize() - base.as_usize(), 16);
        assert_eq!(end.wrapping_sub(16), base);
        unsafe {
            assert_eq!(base.add(16), end);
            assert_eq!(end.sub(16), base);
            assert_eq!(end.offset_from(base), 16);
        }
        assert!(base.is_aligned_to(1));
        assert!(MemAddr::NULL.is_null());
        assert!(!base.is_null());
    }
}
