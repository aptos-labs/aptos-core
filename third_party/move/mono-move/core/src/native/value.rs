// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Read / write of native arg and return values through a frame slot.

use crate::{
    memory::{vec_bytes_or_empty, vec_len_or_zero, FatRef, MemAddr},
    native::native_vector_index_out_of_bounds,
    root_pool::{ObjectHandle, ReferenceHandle, RootPool},
    VMResult,
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

    /// Read a value out of the frame slot at `slot`.
    ///
    /// If the value requires heap allocation, its pointer needs to be rooted in the
    /// [`RootPool`] so it can survive GC. A handle is then returned, which
    /// can be used for safe access throughout the lifetime of the native call.
    ///
    /// # Safety
    ///
    /// `slot` must be within the current frame's arg / return region (or a
    /// vector element of this type), with [`Self::FRAME_SLOT_SIZE`]
    /// initialized bytes.
    unsafe fn read_from_frame(pool: &'a RootPool, slot: MemAddr) -> Self;

    /// Write a value into the frame slot at `slot`.
    ///
    /// # Safety
    ///
    /// `slot` must be within the current frame's arg / return region, with
    /// [`Self::FRAME_SLOT_SIZE`] writable bytes.
    unsafe fn write_to_frame(self, slot: MemAddr);
}

/// Implements [`VMValue`] for an integer type.
///
/// Uses aligned or unaligned access based on the VM's
/// [`MAX_ALIGN`](crate::MAX_ALIGN), via [`MemAddr::load`] / [`MemAddr::store`].
macro_rules! impl_vm_value_for_integer {
    ($t:ty) => {
        impl<'a> VMValue<'a> for $t {
            const FRAME_SLOT_SIZE: usize = core::mem::size_of::<$t>();

            #[inline]
            unsafe fn read_from_frame(_pool: &'a RootPool, slot: MemAddr) -> Self {
                // SAFETY: the caller keeps `slot` within the frame's arg /
                // return region, where the slot holds an initialized `$t`.
                unsafe { slot.load::<$t>() }
            }

            #[inline]
            unsafe fn write_to_frame(self, slot: MemAddr) {
                // SAFETY: the caller keeps `slot` within the frame's arg /
                // return region.
                unsafe { slot.store(self) }
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
    unsafe fn read_from_frame(_pool: &'a RootPool, _slot: MemAddr) -> Self {}

    #[inline]
    unsafe fn write_to_frame(self, _slot: MemAddr) {}
}

impl<'a> VMValue<'a> for bool {
    const FRAME_SLOT_SIZE: usize = 1;

    #[inline]
    unsafe fn read_from_frame(_pool: &'a RootPool, slot: MemAddr) -> Self {
        // SAFETY: the caller keeps `slot` within the frame's arg / return
        // region, where the slot holds an initialized boolean byte.
        unsafe { slot.load_bool() }
    }

    #[inline]
    unsafe fn write_to_frame(self, slot: MemAddr) {
        // SAFETY: the caller keeps `slot` within the frame's arg / return region.
        unsafe { slot.store_bool(self) }
    }
}

impl<'a> VMValue<'a> for AccountAddress {
    const FRAME_SLOT_SIZE: usize = AccountAddress::LENGTH;

    #[inline]
    unsafe fn read_from_frame(_pool: &'a RootPool, slot: MemAddr) -> Self {
        // SAFETY: the caller keeps `slot` within the frame's arg / return
        // region, where the slot holds an initialized `address`; the load is a
        // byte copy, so slot alignment does not matter.
        unsafe { slot.load::<AccountAddress>() }
    }

    #[inline]
    unsafe fn write_to_frame(self, slot: MemAddr) {
        // SAFETY: the caller keeps `slot` within the frame's arg / return region.
        unsafe { slot.store(self) }
    }
}

/// A table's storage handle.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
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

    unsafe fn read_from_frame(pool: &'a RootPool, slot: MemAddr) -> Self {
        // SAFETY: forwarded to the caller; same slot shape as `address`.
        Self(unsafe { AccountAddress::read_from_frame(pool, slot) })
    }

    unsafe fn write_to_frame(self, slot: MemAddr) {
        // SAFETY: forwarded to the caller; same slot shape as `address`.
        unsafe { self.0.write_to_frame(slot) }
    }
}

impl Ref<'_, TableHandle> {
    /// Reads the referenced table handle out by value.
    pub fn get(&self) -> TableHandle {
        // SAFETY: `TableHandle` is `repr(transparent)` over the address bytes the
        // live reference points at; the load is a byte copy, so slot alignment
        // does not matter.
        TableHandle(unsafe { self.addr().load::<AccountAddress>() })
    }
}

impl Ref<'_, AccountAddress> {
    /// Reads the referenced address out by value.
    pub fn get(&self) -> AccountAddress {
        // SAFETY: the reference points at a live `address`; the load is a byte
        // copy, so slot alignment does not matter.
        unsafe { self.addr().load::<AccountAddress>() }
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

    /// Address of the referenced value.
    ///
    /// Stale if GC runs -- it is the caller's responsibility to only use it in a transient
    /// manner.
    #[inline]
    pub fn addr(&self) -> MemAddr {
        self.handle.addr()
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
        // SAFETY: `self` references a valid `vector<V>` value, whose slot is
        // the vector's 8-byte heap pointer.
        let vec_ptr = unsafe { self.addr().load_ptr() };

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
    unsafe fn read_from_frame(pool: &'a RootPool, slot: MemAddr) -> Self {
        // SAFETY: the caller keeps `slot` within the frame's arg / return
        // region, where the slot holds a 16-byte fat reference `(base, offset)`.
        let fat = unsafe { FatRef::load_from(slot) };
        Ref {
            // SAFETY: `fat` is a valid Move reference read from the frame.
            handle: unsafe { pool.root_reference(fat) },
            _marker: PhantomData,
        }
    }

    #[inline]
    unsafe fn write_to_frame(self, slot: MemAddr) {
        // SAFETY: the caller keeps `slot` within the frame's arg / return region.
        unsafe { self.handle.fat().store_to(slot) };
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

    /// Current heap address of the vector object. (Null when unallocated/empty.)
    ///
    /// Stale if GC runs -- it is the caller's responsibility to only use it in a transient
    /// manner.
    #[inline]
    pub fn addr(&self) -> MemAddr {
        self.handle.addr()
    }

    /// The number of elements in the vector.
    #[inline]
    pub fn len(&self) -> u64 {
        // SAFETY: the handle roots a vector object (null when empty).
        unsafe { vec_len_or_zero(self.addr()) }
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
        // SAFETY: element `i` (bounds-checked above, so the vector is non-null)
        // lives at `VEC_DATA_OFFSET + i * FRAME_SLOT_SIZE` in the vector object;
        // `read_from_frame` reads it and roots any heap pointer it carries.
        Ok(unsafe {
            V::read_from_frame(
                self.handle.pool(),
                self.handle
                    .obj()
                    .vec_elem(i, <V as VMValue<'a>>::FRAME_SLOT_SIZE as u32),
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
        // SAFETY: the handle holds null or a live `vector<u8>` on the VM's
        // heap with `len` initialized bytes, and the caller does not hold the
        // slice across an allocation.
        unsafe { vec_bytes_or_empty(self.addr()) }
    }
}

impl<'a, V> VMValue<'a> for Vector<'a, V> {
    const FRAME_SLOT_SIZE: usize = 8;

    #[inline]
    unsafe fn read_from_frame(pool: &'a RootPool, slot: MemAddr) -> Self {
        // SAFETY: the caller keeps `slot` within the frame's arg / return
        // region, where the slot holds the vector's 8-byte heap pointer.
        let ptr = unsafe { slot.load_ptr() };
        // SAFETY: `ptr` is the vector's heap object pointer.
        Vector::from_handle(unsafe { pool.root_object(ptr) })
    }

    #[inline]
    unsafe fn write_to_frame(self, slot: MemAddr) {
        // Write the vector's current heap pointer into the destination slot.
        // SAFETY: the caller keeps `slot` within the frame's arg / return region.
        unsafe { slot.store_ptr(self.addr()) };
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

    /// Current heap address of the boxed object.
    ///
    /// Stale if GC runs -- only for transient use.
    #[inline]
    pub fn addr(&self) -> MemAddr {
        self.handle.addr()
    }
}

/// A boxed value's in-frame representation is the 8-byte pointer to its object,
/// letting a native return one with `set_return`.
impl<'a, T> VMValue<'a> for Boxed<'a, T> {
    const FRAME_SLOT_SIZE: usize = 8;

    #[inline]
    unsafe fn read_from_frame(pool: &'a RootPool, slot: MemAddr) -> Self {
        // SAFETY: the caller keeps `slot` within the frame's arg / return
        // region, where the slot holds the object's 8-byte heap pointer.
        let ptr = unsafe { slot.load_ptr() };
        // SAFETY: `ptr` is the object's heap pointer.
        Boxed::from_handle(unsafe { pool.root_object(ptr) })
    }

    #[inline]
    unsafe fn write_to_frame(self, slot: MemAddr) {
        // SAFETY: the caller keeps `slot` within the frame's arg / return region.
        unsafe { slot.store_ptr(self.addr()) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{align::MAX_ALIGN, memory::Obj, DescriptorId, OBJECT_HEADER_SIZE};

    // `#[repr(align(N))]` needs a literal, so the buffers below use `u64`
    // backing to get `MAX_ALIGN`.
    //
    // TODO(cleanup, testing): allocate through `MemoryRegion`, which is
    // `MAX_ALIGN`-aligned by construction.
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

        // A real allocation: `Ref::addr` offsets `base`, which needs provenance.
        let backing = vec![0u64; 0x400];
        let base = MemAddr::from_ref(backing.as_slice());
        // SAFETY: 0x1000 is within `backing` (0x2000 bytes).
        let unrelated = unsafe { base.add(0x1000) };

        // Slot 0 holds the reference; slot 1 takes the write-back. The frame is
        // `MAX_ALIGN`-aligned because the fat-pointer halves are written as
        // aligned 8-byte stores.
        let mut frame = [0u64; 4];
        let frame_addr = MemAddr::from_mut(&mut frame);
        let offset = 24_u64;
        // SAFETY: slot 0 is the aligned first 16 bytes of `frame`.
        unsafe { FatRef::new(base, offset).store_to(frame_addr) };

        let reference: Ref<Opaque> = unsafe { Ref::read_from_frame(&pool, frame_addr) };

        // Overwrite slot 0 with unrelated bytes, as a later `set_return` would.
        // SAFETY: as above.
        unsafe { FatRef::new(unrelated, 7).store_to(frame_addr) };

        // Reading the address still resolves the original referent.
        assert_eq!(reference.addr(), unsafe { base.add(offset as usize) });

        // Writing the reference into slot 1 writes the original fat pointer.
        // SAFETY: slot 1 is the aligned second 16 bytes of `frame`.
        unsafe { reference.write_to_frame(frame_addr.add(16)) };
        // SAFETY: slot 1 is the aligned second 16 bytes of `frame`, just written.
        assert_eq!(
            unsafe { FatRef::load_from(frame_addr.add(16)) },
            FatRef::new(base, offset)
        );
    }

    /// The data pointer of an object laid out by the helpers below: the word
    /// after the header at the buffer's start.
    fn test_obj_data(buf: &mut [u64]) -> MemAddr {
        // SAFETY: the buffer holds at least the header word.
        unsafe { MemAddr::from_mut(buf).add(OBJECT_HEADER_SIZE) }
    }

    /// Writes an object header covering the whole buffer and returns the
    /// object.
    fn test_obj(buf: &mut [u64]) -> Obj {
        let size = (buf.len() * 8) as u32;
        // SAFETY: the buffer's first word is the header reservation.
        unsafe { Obj::write_header(test_obj_data(buf), DescriptorId(1), size) }
    }

    /// Lays out a `vector<u8>` heap object -- `[header][len: u64][bytes...]`
    /// -- in an 8-byte-aligned buffer. The object's data pointer is
    /// `test_obj_data`: the length is at `VEC_LENGTH_OFFSET`, the bytes at
    /// `VEC_DATA_OFFSET`.
    fn byte_vec_object_for_test(bytes: &[u8]) -> Vec<u64> {
        let mut buf = vec![0u64; 2 + bytes.len().div_ceil(8)];
        buf[1] = bytes.len() as u64;
        // SAFETY: `buf` has room for `bytes.len()` initialized (zeroed) bytes
        // after the header and length words, and nothing else accesses them
        // while the slice is live.
        let data = unsafe { test_obj(&mut buf).vec_data().as_bytes_mut(bytes.len()) };
        data.copy_from_slice(bytes);
        buf
    }

    /// Builds a `vector<vector<u8>>` object with layout
    /// `[header][len: u64][inner_ptr...]`. The inner buffers must outlive all
    /// uses of the returned object.
    fn nested_byte_vec_object_for_test(inners: &mut [Vec<u64>]) -> Vec<u64> {
        let mut outer = vec![0u64; 2 + inners.len()];
        outer[1] = inners.len() as u64;
        let outer_obj = test_obj(&mut outer);
        for (slot, inner) in inners.iter_mut().enumerate() {
            // SAFETY: `outer` has an aligned pointer slot for each inner vector.
            // `store_ptr` preserves each inner buffer's pointer provenance.
            unsafe {
                outer_obj
                    .vec_elem(slot as u64, 8)
                    .store_ptr(test_obj_data(inner.as_mut_slice()))
            };
        }
        outer
    }

    /// `Vector::<Vector<u8>>::get_element` reads each inner byte vector back by
    /// value, including an empty inner vector.
    #[test]
    fn nested_byte_vector_get_reads_elements() {
        let pool = RootPool::new();

        // Three inner byte vectors, kept alive for the reads below.
        let mut inners = [
            byte_vec_object_for_test(b"hello"),
            byte_vec_object_for_test(b""),
            byte_vec_object_for_test(b"world!!"),
        ];

        let mut outer = nested_byte_vec_object_for_test(&mut inners);
        let outer_addr = test_obj_data(outer.as_mut_slice());

        // SAFETY: `outer` is a valid `vector<vector<u8>>` object and the inner
        // buffers outlive the reads.
        let vec: Vector<Vector<u8>> = Vector::from_handle(unsafe { pool.root_object(outer_addr) });

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
        let mut inners = [
            byte_vec_object_for_test(b"a"),
            byte_vec_object_for_test(b"b"),
        ];
        let mut outer = nested_byte_vec_object_for_test(&mut inners);
        let outer_addr = test_obj_data(outer.as_mut_slice());

        // SAFETY: `outer` is a valid `vector<vector<u8>>` object.
        let vec: Vector<Vector<u8>> = Vector::from_handle(unsafe { pool.root_object(outer_addr) });

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
        let vec: Vector<u64> = Vector::from_handle(unsafe { pool.root_object(MemAddr::NULL) });
        assert_eq!(vec.len(), 0);
        assert!(vec.get_element(0).is_err());
    }
}
