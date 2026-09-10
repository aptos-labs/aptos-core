// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Read / write of native arg and return values through a frame slot.

use crate::{
    align::MAX_ALIGN,
    memory::{read_fat_ptr, read_ptr, read_u64, write_fat_ptr, write_ptr},
    native::native_vector_index_out_of_bounds,
    root_pool::{ObjectHandle, ReferenceHandle, RootPool},
    VMResult, VEC_DATA_OFFSET, VEC_LENGTH_OFFSET,
};
use move_core_types::{
    account_address::AccountAddress,
    int256::{I256, U256},
};
use std::marker::PhantomData;

/// Defines how a type is read from a native arg slot and written to a return
/// slot. Once implemented, a type can be used as a native argument or return.
///
/// This is a low-level API only intended for the native context to extend
/// its data access.
///
/// It is NOT meant to be used by native functions themselves to access their
/// arguments and returns directly -- for that purpose, use [`NativeContext::arg`]
/// and [`NativeContext::set_return`] instead.
///
/// Native functions should, however, implement this trait for their own custom
/// data types if they are using them as arguments or returns.
pub trait VMValue<'a>: Sized {
    /// Byte size of the slot that holds this value. (Heap memory not included.)
    const FRAME_SLOT_SIZE: usize;

    /// Read a value out of a frame at the given byte offset.
    ///
    /// If the value requires heap allocation, its pointer needs to be rooted in the
    /// [`RootPool`] so it can survive GC. A handle is then returned, which
    /// can be used for safe access throughout the lifetime of the native call.
    ///
    /// # Safety
    ///
    /// Memory access must be within the current frame's arg / return region.
    unsafe fn read_from_frame(pool: &'a RootPool, frame_ptr: *const u8, offset: usize) -> Self;

    /// Write a value into a frame at the given byte offset.
    ///
    /// # Safety
    ///
    /// Memory access must be within the current frame's arg / return region.
    unsafe fn write_to_frame(self, frame_ptr: *mut u8, offset: usize);
}

/// Implements [`VMValue`] for an integer type.
///
/// Uses aligned or unaligned access based on the the VM's [`MAX_ALIGN`].
macro_rules! impl_vm_value_for_integer {
    ($t:ty) => {
        impl<'a> VMValue<'a> for $t {
            const FRAME_SLOT_SIZE: usize = core::mem::size_of::<$t>();

            #[inline]
            unsafe fn read_from_frame(
                _pool: &'a RootPool,
                frame_ptr: *const u8,
                offset: usize,
            ) -> Self {
                let p = unsafe { frame_ptr.add(offset) as *const $t };
                unsafe {
                    if core::mem::align_of::<$t>() <= MAX_ALIGN {
                        core::ptr::read(p)
                    } else {
                        core::ptr::read_unaligned(p)
                    }
                }
            }

            #[inline]
            unsafe fn write_to_frame(self, frame_ptr: *mut u8, offset: usize) {
                let p = unsafe { frame_ptr.add(offset) as *mut $t };
                unsafe {
                    if core::mem::align_of::<$t>() <= MAX_ALIGN {
                        core::ptr::write(p, self);
                    } else {
                        core::ptr::write_unaligned(p, self);
                    }
                }
            }
        }
    };
}

impl_vm_value_for_integer!(u8);
impl_vm_value_for_integer!(u16);
impl_vm_value_for_integer!(u32);
impl_vm_value_for_integer!(u64);
impl_vm_value_for_integer!(u128);
impl_vm_value_for_integer!(i8);
impl_vm_value_for_integer!(i16);
impl_vm_value_for_integer!(i32);
impl_vm_value_for_integer!(i64);
impl_vm_value_for_integer!(i128);
impl_vm_value_for_integer!(U256);
impl_vm_value_for_integer!(I256);

/// The empty payload of a fieldless enum variant: zero-width, reads and writes
/// nothing.
impl<'a> VMValue<'a> for () {
    const FRAME_SLOT_SIZE: usize = 0;

    #[inline]
    unsafe fn read_from_frame(_pool: &'a RootPool, _frame_ptr: *const u8, _offset: usize) -> Self {}

    #[inline]
    unsafe fn write_to_frame(self, _frame_ptr: *mut u8, _offset: usize) {}
}

impl<'a> VMValue<'a> for bool {
    const FRAME_SLOT_SIZE: usize = 1;

    #[inline]
    unsafe fn read_from_frame(_pool: &'a RootPool, frame_ptr: *const u8, offset: usize) -> Self {
        unsafe { core::ptr::read_unaligned(frame_ptr.add(offset)) != 0 }
    }

    #[inline]
    unsafe fn write_to_frame(self, frame_ptr: *mut u8, offset: usize) {
        unsafe {
            core::ptr::write_unaligned(frame_ptr.add(offset), self as u8);
        }
    }
}

impl<'a> VMValue<'a> for AccountAddress {
    const FRAME_SLOT_SIZE: usize = AccountAddress::LENGTH;

    #[inline]
    unsafe fn read_from_frame(_pool: &'a RootPool, frame_ptr: *const u8, offset: usize) -> Self {
        unsafe { core::ptr::read_unaligned(frame_ptr.add(offset) as *const AccountAddress) }
    }

    #[inline]
    unsafe fn write_to_frame(self, frame_ptr: *mut u8, offset: usize) {
        unsafe {
            core::ptr::write_unaligned(frame_ptr.add(offset) as *mut AccountAddress, self);
        }
    }
}

/// A table's storage handle.
#[repr(transparent)]
#[derive(Clone, Copy, Eq, PartialEq, Hash)]
pub struct TableHandle(AccountAddress);

impl TableHandle {
    pub fn new(address: AccountAddress) -> Self {
        Self(address)
    }

    /// The handle's address.
    pub fn address(&self) -> AccountAddress {
        self.0
    }
}

/// A `TableHandle` has the same representation as the `address` it wraps.
impl<'a> VMValue<'a> for TableHandle {
    const FRAME_SLOT_SIZE: usize = <AccountAddress as VMValue<'a>>::FRAME_SLOT_SIZE;

    unsafe fn read_from_frame(pool: &'a RootPool, frame_ptr: *const u8, offset: usize) -> Self {
        Self(unsafe { AccountAddress::read_from_frame(pool, frame_ptr, offset) })
    }

    unsafe fn write_to_frame(self, frame_ptr: *mut u8, offset: usize) {
        unsafe { self.0.write_to_frame(frame_ptr, offset) }
    }
}

impl Ref<'_, TableHandle> {
    /// Borrows the referenced table handle.
    pub fn get(&self) -> &TableHandle {
        // SAFETY: `TableHandle` is `repr(transparent)` over the address bytes the
        // reference points at, so the referent reinterprets as `&TableHandle`.
        unsafe { &*(self.ptr() as *const TableHandle) }
    }
}

/// Marker for a type that is not statically known.
///
/// This can be used to build composite types in generic native functions — e.g. the
/// `&mut T` arguments of a generic native are read as `Ref<Opaque>`, with the size
/// info being determinted based on the type argument.
///
/// It is an empty (uninhabited) enum: a type with no valid instantiations -- exactly
/// what a marker type should be.
pub enum Opaque {}

/// Represents a typed Move reference.
///
/// Valid throughout the lifetime of the native call -- safe to use even across GC calls.
pub struct Ref<'a, T> {
    handle: ReferenceHandle<'a>,
    _marker: PhantomData<T>,
}

impl<'a, T> Ref<'a, T> {
    /// Wraps a rooted reference handle.
    pub fn from_handle(handle: ReferenceHandle<'a>) -> Self {
        Self {
            handle,
            _marker: PhantomData,
        }
    }

    /// Raw pointer to the referenced value.
    ///
    /// Stale if GC runs -- it is the caller's responsibility to only use it in a transient
    /// manner.
    #[inline]
    pub fn ptr(&self) -> *mut u8 {
        self.handle.ptr()
    }
}

impl<'a, V> Ref<'a, Vector<'a, V>> {
    /// Borrow the `vector<V>` behind this reference.
    ///
    /// The returned value is a handle to the vector -- valid throughout the lifetime of the native call.
    ///
    /// Note that it is still the caller's responsibility to ensure logical consistency in case the
    /// original slot is overwritten by another vector value -- this vector handle will still be
    /// pointing to the old instance.
    #[inline]
    pub fn borrow(&self) -> Vector<'a, V> {
        // SAFETY: `self` references valid a `vector<V>` value.
        let vec_ptr = unsafe { read_ptr(self.handle.ptr(), 0usize) };

        // It is still safer to root the vector, even if an external owner may exist.
        //
        // Once rooted, a vector will be valid throughout the lifetime of the native call,
        // even if its original slot is overwritten by another vector value.
        //
        // SAFETY: `vec_ptr` is the heap object pointer held by a valid `vector<V>`.
        Vector::from_handle(unsafe { self.handle.pool().root_object(vec_ptr) })
    }
}

impl<'a, T> VMValue<'a> for Ref<'a, T> {
    const FRAME_SLOT_SIZE: usize = 16;

    #[inline]
    unsafe fn read_from_frame(pool: &'a RootPool, frame_ptr: *const u8, offset: usize) -> Self {
        // SAFETY: the slot holds a 16-byte fat reference `(base, offset)`.
        let (base, byte_offset) = unsafe { read_fat_ptr(frame_ptr, offset) };
        Ref {
            // SAFETY: `(base, offset)` is a valid Move reference read from the frame.
            handle: unsafe { pool.root_reference(base, byte_offset) },
            _marker: PhantomData,
        }
    }

    #[inline]
    unsafe fn write_to_frame(self, frame_ptr: *mut u8, offset: usize) {
        let (base, byte_offset) = self.handle.fat();
        unsafe { write_fat_ptr(frame_ptr, offset, base, byte_offset) };
    }
}

/// A handle to a Move vector value.
///
/// Valid throughout the lifetime of the native call -- safe to use even across GC calls.
pub struct Vector<'a, V> {
    handle: ObjectHandle<'a>,
    _marker: PhantomData<V>,
}

impl<'a, V> Vector<'a, V> {
    /// Wrap a rooted object handle. VM-internal (the context impl builds a
    /// `Vector` this way).
    ///
    /// Natives should not have a reason to call this -- if they want a vector,
    /// they can obtain it through composing [`VMValue`], a reference or a fresh
    /// allocation through the context.
    pub fn from_handle(handle: ObjectHandle<'a>) -> Self {
        Self {
            handle,
            _marker: PhantomData,
        }
    }

    /// Current heap pointer of the vector object. (Null when unallocated/empty.)
    ///
    /// Stale if GC runs -- it is the caller's responsibility to only use it in a transient
    /// manner.
    ///
    /// Note: currently private, but could be made public if needed. In that case, external
    /// callers need to follow the same rule above.
    #[inline]
    fn ptr(&self) -> *mut u8 {
        self.handle.ptr()
    }

    /// The number of elements in the vector.
    #[inline]
    pub fn len(&self) -> u64 {
        let p = self.ptr();
        if p.is_null() {
            0
        } else {
            // SAFETY: a non-null vector object holds its length at the start.
            unsafe { read_u64(p, VEC_LENGTH_OFFSET) }
        }
    }

    /// Whether the vector has no elements.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Reinterprets the element type of a vector.
    ///
    /// # Safety
    ///
    /// The caller must ensure the object's actual element layout matches
    /// the target layout.
    #[inline]
    pub(crate) unsafe fn transmute_unchecked<U>(self) -> Vector<'a, U> {
        Vector::from_handle(self.handle)
    }

    // TODO(completeness): Other vector APIs, added on-demand.
}

impl<'a, V: VMValue<'a>> Vector<'a, V> {
    /// Reads element `i` by value, using `V`'s in-frame representation. Any heap
    /// pointer the element carries is rooted for the rest of the native call, so
    /// the result survives GC. Returns an invalid-operation error if `i` is out
    /// of bounds.
    #[inline]
    pub fn get_element(&self, i: u64) -> VMResult<V> {
        let len = self.len();
        if i >= len {
            return Err(native_vector_index_out_of_bounds(i, len));
        }
        // SAFETY: element `i` (bounds-checked above) lives at `VEC_DATA_OFFSET +
        // i * FRAME_SLOT_SIZE` in the vector object; `read_from_frame` reads it
        // and roots any heap pointer it carries.
        Ok(unsafe {
            V::read_from_frame(
                self.handle.pool(),
                self.ptr(),
                VEC_DATA_OFFSET + i as usize * <V as VMValue<'a>>::FRAME_SLOT_SIZE,
            )
        })
    }
}

impl Vector<'_, u8> {
    /// Returns a reference to the bytes stored in the vector, borrowed directly from the heap.
    ///
    /// # Safety
    ///
    /// The slice is invalidated if the next heap allocation in this native call
    /// triggers a GC, which may relocate the bytes. The caller must NOT hold it
    /// across allocations.
    #[inline]
    pub unsafe fn as_bytes(&self) -> &[u8] {
        let p = self.ptr();
        if p.is_null() {
            return &[];
        }
        // SAFETY: `self` points to a valid `vector<u8>` on the VM's heap.
        unsafe {
            let len = read_u64(p, VEC_LENGTH_OFFSET) as usize;
            std::slice::from_raw_parts(p.add(VEC_DATA_OFFSET), len)
        }
    }
}

impl<'a, V> VMValue<'a> for Vector<'a, V> {
    const FRAME_SLOT_SIZE: usize = 8;

    #[inline]
    unsafe fn read_from_frame(pool: &'a RootPool, frame_ptr: *const u8, offset: usize) -> Self {
        // SAFETY: the slot holds the vector's 8-byte heap pointer.
        let ptr = unsafe { read_ptr(frame_ptr, offset) };
        // SAFETY: `ptr` is the vector's heap object pointer.
        Vector::from_handle(unsafe { pool.root_object(ptr) })
    }

    #[inline]
    unsafe fn write_to_frame(self, frame_ptr: *mut u8, offset: usize) {
        // Write the vector's current heap pointer into the destination slot.
        unsafe { write_ptr(frame_ptr, offset, self.ptr()) };
    }
}

/// An owned, freshly heap-allocated Move value.
///
/// Unlike a `Ref`, it owns its heap object rather than borrowing one.
/// Valid throughout the native call and GC-safe.
#[must_use = "a boxed value should be consumed"]
pub struct Boxed<'a, T> {
    handle: ObjectHandle<'a>,
    _marker: PhantomData<T>,
}

impl<'a, T> Boxed<'a, T> {
    /// Wraps a rooted heap object. VM-internal (the context impl boxes a value
    /// this way).
    pub fn from_handle(handle: ObjectHandle<'a>) -> Self {
        Self {
            handle,
            _marker: PhantomData,
        }
    }

    /// Current heap pointer of the boxed object.
    ///
    /// Stale if GC runs -- only for transient use.
    #[inline]
    pub fn ptr(&self) -> *mut u8 {
        self.handle.ptr()
    }
}

/// A boxed value's in-frame representation is the 8-byte pointer to its object,
/// letting a native return one with `set_return`.
impl<'a, T> VMValue<'a> for Boxed<'a, T> {
    const FRAME_SLOT_SIZE: usize = 8;

    #[inline]
    unsafe fn read_from_frame(pool: &'a RootPool, frame_ptr: *const u8, offset: usize) -> Self {
        // SAFETY: the slot holds the object's 8-byte heap pointer.
        let ptr = unsafe { read_ptr(frame_ptr, offset) };
        // SAFETY: `ptr` is the object's heap pointer.
        Boxed::from_handle(unsafe { pool.root_object(ptr) })
    }

    #[inline]
    unsafe fn write_to_frame(self, frame_ptr: *mut u8, offset: usize) {
        unsafe { write_ptr(frame_ptr, offset, self.ptr()) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // `#[repr(align(N))]` needs a literal, so the buffers below use `u64`
    // backing to get `MAX_ALIGN`.
    //
    // TODO(cleanup, testing): allocate through `MemoryRegion`, which is
    // `MAX_ALIGN`-aligned by construction; it lives in `mono-move-runtime`,
    // which this crate cannot depend on.
    const _: () = assert!(
        core::mem::align_of::<u64>() >= MAX_ALIGN,
        "u64 no longer covers MAX_ALIGN"
    );

    /// A `Ref` must preserve its value even after the frame slot it was read
    /// from is overwritten — e.g. by a `set_return` that reuses the argument
    /// region.
    #[test]
    fn ref_preserves_value_after_source_slot_overwritten() {
        let pool = RootPool::new();

        // A real allocation: `Ref::ptr` offsets `base`, which needs provenance.
        let backing = vec![0u64; 0x400];
        let base = backing.as_ptr() as *mut u8;
        // SAFETY: 0x1000 is within `backing`.
        let unrelated = unsafe { base.add(0x1000) };

        // Slot 0 holds the reference; slot 1 takes the write-back. The frame is
        // `MAX_ALIGN`-aligned because the fat-pointer halves are written as
        // aligned 8-byte stores.
        let mut frame = [0u64; 4];
        let frame_ptr: *mut u8 = frame.as_mut_ptr().cast();
        let offset = 24_u64;
        unsafe { write_fat_ptr(frame_ptr, 0usize, base, offset) };

        let reference: Ref<Opaque> = unsafe { Ref::read_from_frame(&pool, frame_ptr, 0) };

        // Overwrite slot 0 with unrelated bytes, as a later `set_return` would.
        unsafe { write_fat_ptr(frame_ptr, 0usize, unrelated, 7) };

        // Reading the pointer still resolves the original referent.
        assert_eq!(reference.ptr(), unsafe { base.add(offset as usize) });

        // Writing the reference into slot 1 writes the original fat pointer.
        unsafe { reference.write_to_frame(frame_ptr, 16) };
        assert_eq!(unsafe { read_fat_ptr(frame_ptr, 16usize) }, (base, offset));
    }

    /// Lays out a `vector<u8>` heap object -- `[len: u64][bytes...]` -- in an
    /// 8-byte-aligned buffer. The object's data pointer is the buffer's start:
    /// the length is at `VEC_LENGTH_OFFSET`, the bytes at `VEC_DATA_OFFSET`.
    fn byte_vec_object_for_test(bytes: &[u8]) -> Vec<u64> {
        let mut buf = vec![0u64; 1 + bytes.len().div_ceil(8)];
        buf[0] = bytes.len() as u64;
        // SAFETY: `buf` has room for `bytes.len()` bytes after the length word.
        let data = unsafe {
            std::slice::from_raw_parts_mut(buf.as_mut_ptr().add(1) as *mut u8, bytes.len())
        };
        data.copy_from_slice(bytes);
        buf
    }

    /// `Vector::<Vector<u8>>::get_element` reads each inner byte vector back by
    /// value, including an empty inner vector.
    #[test]
    fn nested_byte_vector_get_reads_elements() {
        let pool = RootPool::new();

        // Three inner byte vectors, kept alive for the reads below.
        let inners = [
            byte_vec_object_for_test(b"hello"),
            byte_vec_object_for_test(b""),
            byte_vec_object_for_test(b"world!!"),
        ];

        // Outer object: `[len: u64][inner_ptr0][inner_ptr1][inner_ptr2]`.
        let mut outer = vec![0u64; 1 + inners.len()];
        outer[0] = inners.len() as u64;
        let outer_ptr: *mut u8 = outer.as_mut_ptr().cast();
        for (i, inner) in inners.iter().enumerate() {
            // SAFETY: slot `i` is within `outer`. The pointers are stored with
            // `write_ptr` rather than as `u64`, which would discard provenance
            // and leave `get` reading through an address it cannot dereference.
            unsafe { write_ptr(outer_ptr, VEC_DATA_OFFSET + i * 8, inner.as_ptr().cast()) };
        }

        // SAFETY: `outer` is a valid `vector<vector<u8>>` object and the inner
        // buffers outlive the reads.
        let vec: Vector<Vector<u8>> = Vector::from_handle(unsafe { pool.root_object(outer_ptr) });

        assert_eq!(vec.len(), 3);
        // Bind each inner handle so its bytes outlive the borrow.
        let e0 = vec.get_element(0).unwrap();
        let e1 = vec.get_element(1).unwrap();
        let e2 = vec.get_element(2).unwrap();
        assert_eq!(unsafe { e0.as_bytes() }, b"hello");
        assert_eq!(unsafe { e1.as_bytes() }, b"");
        assert_eq!(unsafe { e2.as_bytes() }, b"world!!");
    }

    /// `get_element` fails with an invalid-operation error on an out-of-bounds
    /// index rather than reading past the vector object.
    #[test]
    fn get_element_out_of_bounds_errors() {
        let pool = RootPool::new();

        // Outer object holding two inner byte vectors.
        let inners = [
            byte_vec_object_for_test(b"a"),
            byte_vec_object_for_test(b"b"),
        ];
        let mut outer = vec![0u64; 1 + inners.len()];
        outer[0] = inners.len() as u64;
        for (i, inner) in inners.iter().enumerate() {
            outer[1 + i] = inner.as_ptr() as u64;
        }

        // SAFETY: `outer` is a valid `vector<vector<u8>>` object.
        let vec: Vector<Vector<u8>> =
            Vector::from_handle(unsafe { pool.root_object(outer.as_ptr() as *mut u8) });

        assert_eq!(vec.len(), 2);
        assert!(vec.get_element(0).is_ok());
        // Index == len and beyond both fail.
        match vec.get_element(2) {
            Ok(_) => panic!("out-of-bounds index must not read an element"),
            Err(err) => assert_eq!(err.kind(), crate::ExecutionErrorKind::InvalidOperation),
        }
        assert!(vec.get_element(u64::MAX).is_err());
    }

    /// `get_element` on an empty vector errors for every index.
    #[test]
    fn get_element_empty_vector_errors() {
        let pool = RootPool::new();
        // A null-backed vector reads as empty.
        let vec: Vector<u64> =
            Vector::from_handle(unsafe { pool.root_object(std::ptr::null_mut()) });
        assert_eq!(vec.len(), 0);
        assert!(vec.get_element(0).is_err());
    }
}
