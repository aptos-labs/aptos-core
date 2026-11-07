// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The VM's view of memory on top of [`MemAddr`]: a Move reference
//! ([`FatRef`]), a call frame ([`Frame`], [`FrameMeta`]) and a heap object
//! ([`Obj`], [`ObjHeader`]).
//!
//! An object pointer addresses the first byte of the object's data; the
//! `[descriptor_id: u32 | size: u32]` header occupies the 8 bytes just below
//! it (the allocator reserves `OBJECT_HEADER_SIZE` bytes there, which keeps
//! the data `MAX_ALIGN`-aligned). [`Obj::header`], [`Obj::write_header`] and
//! [`Obj::set_descriptor`] are the only places that reach below a data
//! pointer.

use crate::{
    ClosureFuncRef, DescriptorId, FrameOffset, Function, CLOSURE_CAPTURED_DATA_PTR_OFFSET,
    CLOSURE_FUNC_REF_OFFSET, CLOSURE_MASK_OFFSET, ENUM_TAG_OFFSET, FRAME_METADATA_SIZE,
    FUNC_REF_PAYLOAD_OFFSET, FUNC_REF_TAG_OFFSET, FUNC_REF_TAG_RESOLVED, FUNC_REF_TAG_UNRESOLVED,
    OBJECT_HEADER_SIZE, VEC_DATA_OFFSET, VEC_LENGTH_OFFSET,
};
pub use mono_move_alloc::MemoryRegion;
pub use mono_move_mem::{MemAddr, Scalar};

// ---------------------------------------------------------------------------
// Frame metadata layout
// ---------------------------------------------------------------------------

/// Byte offset of `saved_pc` within frame metadata.
pub const META_SAVED_PC_OFFSET: usize = 0;
/// Byte offset of `saved_fp` within frame metadata.
pub const META_SAVED_FP_OFFSET: usize = 8;
/// Byte offset of `saved_func_ptr` within frame metadata.
pub const META_SAVED_FUNC_PTR_OFFSET: usize = 16;

const _: () = assert!(META_SAVED_FUNC_PTR_OFFSET + 8 == FRAME_METADATA_SIZE);

// ---------------------------------------------------------------------------
// FatRef: the 16-byte representation of a Move reference
// ---------------------------------------------------------------------------

/// Byte offset of the `offset` half of a fat pointer; `base` is first.
pub const FAT_PTR_OFFSET_HALF: usize = 8;

/// A Move reference: a base pointer to an object or stack slot, plus a byte
/// offset into it. [`FatRef::target`] is the only place the VM computes
/// `base + offset`.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct FatRef {
    pub base: MemAddr,
    pub offset: u64,
}

impl FatRef {
    #[inline(always)]
    pub const fn new(base: MemAddr, offset: u64) -> FatRef {
        FatRef { base, offset }
    }

    /// # Safety
    ///
    /// `slot` must be a readable, initialized, pointer-aligned 16-byte slot.
    #[inline(always)]
    pub unsafe fn load_from(slot: MemAddr) -> FatRef {
        // SAFETY: forwarded to the caller; both halves lie within the slot.
        unsafe {
            FatRef {
                base: slot.load_ptr(),
                offset: slot.add(FAT_PTR_OFFSET_HALF).load::<u64>(),
            }
        }
    }

    /// # Safety
    ///
    /// `slot` must be a writable, pointer-aligned 16-byte slot.
    #[inline(always)]
    pub unsafe fn store_to(self, slot: MemAddr) {
        // SAFETY: forwarded to the caller; both halves lie within the slot.
        unsafe {
            slot.store_ptr(self.base);
            slot.add(FAT_PTR_OFFSET_HALF).store::<u64>(self.offset);
        }
    }

    /// The address the reference points at.
    ///
    /// # Safety
    ///
    /// The reference must be live: `base + offset` lies within the object or
    /// slot `base` points at.
    #[inline(always)]
    pub unsafe fn target(self) -> MemAddr {
        // SAFETY: forwarded to the caller.
        unsafe { self.base.add(self.offset as usize) }
    }

    /// A reference `delta` bytes further into the same object.
    #[inline(always)]
    pub fn with_added_offset(self, delta: u64) -> FatRef {
        FatRef {
            base: self.base,
            offset: self.offset + delta,
        }
    }
}

// ---------------------------------------------------------------------------
// Frame: a live call frame
// ---------------------------------------------------------------------------

/// A live call frame. Slots are addressed as `fp + offset`; the metadata
/// block the call protocol wrote sits just below `fp`.
///
/// Constructing one asserts the frame exists; forming a slot address asserts
/// the offset belongs to the function executing in it; what construction
/// covers (the metadata) is safe to read.
#[repr(transparent)]
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Frame(MemAddr);

impl Frame {
    /// # Safety
    ///
    /// `fp` must be the aligned frame pointer of a live frame: its extent lies
    /// within the stack and the call protocol has written its metadata.
    #[inline(always)]
    pub unsafe fn new(fp: MemAddr) -> Frame {
        debug_assert!(fp.is_aligned_to(crate::MAX_ALIGN));
        Frame(fp)
    }

    /// The frame pointer.
    #[inline(always)]
    pub fn fp(self) -> MemAddr {
        self.0
    }

    /// The slot at `off`.
    ///
    /// # Safety
    ///
    /// `off` must come from the verified code of the function executing in
    /// this frame.
    #[inline(always)]
    pub unsafe fn slot(self, off: FrameOffset) -> MemAddr {
        // SAFETY: forwarded to the caller.
        unsafe { self.0.add(usize::from(off)) }
    }

    /// The target of the reference held in the slot at `off`.
    ///
    /// # Safety
    ///
    /// As [`Frame::slot`], plus the slot must hold a live reference (see
    /// [`FatRef::target`]).
    #[inline(always)]
    pub unsafe fn ref_target(self, off: FrameOffset) -> MemAddr {
        // SAFETY: forwarded to the caller.
        unsafe { FatRef::load_from(self.slot(off)).target() }
    }

    /// `fp + bytes`, for offsets that are not [`FrameOffset`]s.
    ///
    /// # Safety
    ///
    /// The result must lie within, or one past the end of, the frame.
    #[inline(always)]
    pub unsafe fn at(self, bytes: usize) -> MemAddr {
        // SAFETY: forwarded to the caller.
        unsafe { self.0.add(bytes) }
    }

    /// The metadata block below this frame.
    #[inline(always)]
    pub fn meta(self) -> FrameMeta {
        // SAFETY: `Frame::new` guarantees the metadata block below `fp` exists
        // and is initialized.
        FrameMeta(unsafe { self.0.sub(FRAME_METADATA_SIZE) })
    }

    /// The caller's frame and function, or `None` for the root frame.
    ///
    /// # Safety
    ///
    /// As for [`FrameMeta::saved_caller`].
    #[inline(always)]
    pub unsafe fn caller<'a>(self) -> Option<(Frame, &'a Function)> {
        let meta = self.meta();
        // SAFETY: forwarded to the caller.
        let func = unsafe { meta.saved_caller()? };
        // SAFETY: a non-null saved function means this is not the root frame,
        // so the saved fp is the caller's live, aligned frame pointer written
        // by the call protocol.
        Some((unsafe { Frame::new(meta.saved_fp()) }, func))
    }
}

/// A frame's metadata block: the caller's pc word, frame pointer, and
/// function pointer. Read via [`Frame::meta`]; written by the call protocol
/// via [`FrameMeta::write_at`].
#[repr(transparent)]
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct FrameMeta(MemAddr);

impl FrameMeta {
    /// The saved pc word; the interpreter packs a flag into its top bit.
    #[inline(always)]
    pub fn saved_pc_word(self) -> u64 {
        // SAFETY: `Frame::meta` guarantees an initialized, aligned block.
        unsafe { self.0.add(META_SAVED_PC_OFFSET).load::<u64>() }
    }

    /// The caller's frame pointer.
    #[inline(always)]
    pub fn saved_fp(self) -> MemAddr {
        // SAFETY: `Frame::meta` guarantees an initialized, aligned block.
        unsafe { self.0.add(META_SAVED_FP_OFFSET).load_ptr() }
    }

    /// The caller's function pointer, or null for the root frame.
    #[inline(always)]
    pub fn saved_func_ptr(self) -> MemAddr {
        // SAFETY: `Frame::meta` guarantees an initialized, aligned block.
        unsafe { self.0.add(META_SAVED_FUNC_PTR_OFFSET).load_ptr() }
    }

    /// The caller's function, or `None` for the root frame. This is the one
    /// place the saved word is turned back into a [`Function`]; the word is
    /// the address the call protocol stored from the caller's function pointer.
    ///
    /// # Safety
    ///
    /// The saved word must be null or the address of a `Function` that the
    /// execution guard keeps alive (frame metadata integrity).
    #[inline(always)]
    pub unsafe fn saved_caller<'a>(self) -> Option<&'a Function> {
        let word = self.saved_func_ptr();
        if word.is_null() {
            None
        } else {
            // SAFETY: forwarded to the caller: non-null means a live `Function`.
            Some(unsafe { word.as_ref::<Function>() })
        }
    }

    /// Writes the metadata block of the frame about to be entered.
    ///
    /// # Safety
    ///
    /// `at` must be the pointer-aligned, writable block at
    /// `callee_fp - FRAME_METADATA_SIZE`.
    #[inline(always)]
    pub unsafe fn write_at(
        at: MemAddr,
        saved_pc_word: u64,
        saved_fp: MemAddr,
        saved_func_ptr: MemAddr,
    ) {
        // SAFETY: forwarded to the caller; all three words lie within the block.
        unsafe {
            at.add(META_SAVED_PC_OFFSET).store::<u64>(saved_pc_word);
            at.add(META_SAVED_FP_OFFSET).store_ptr(saved_fp);
            at.add(META_SAVED_FUNC_PTR_OFFSET).store_ptr(saved_func_ptr);
        }
    }
}

// ---------------------------------------------------------------------------
// Obj: a live heap object
// ---------------------------------------------------------------------------

/// `descriptor_id` sits at `data - 8`, `size` at `data - 4`.
const HEADER_DESCRIPTOR_NEG_OFFSET: usize = 8;
const HEADER_SIZE_NEG_OFFSET: usize = 4;

/// A live heap object, identified by its data pointer.
///
/// Constructing one asserts the object exists with an allocator-written
/// header; forming a field or element address asserts the offset is within
/// the object; reading the header is safe.
#[repr(transparent)]
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Obj(MemAddr);

/// An object's header, loaded by value.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct ObjHeader {
    /// Descriptor id, or the GC's forwarded marker during a collection.
    pub descriptor: u32,
    /// Total size: header plus `MAX_ALIGN`-rounded payload.
    pub size: u32,
}

impl Obj {
    /// # Safety
    ///
    /// `data` must be the data pointer of a live heap object.
    #[inline(always)]
    pub unsafe fn new(data: MemAddr) -> Obj {
        debug_assert!(!data.is_null());
        Obj(data)
    }

    /// The data pointer.
    #[inline(always)]
    pub fn addr(self) -> MemAddr {
        self.0
    }

    /// The header's descriptor word.
    #[inline(always)]
    fn descriptor_slot(self) -> MemAddr {
        // SAFETY: `Obj::new` guarantees an allocator-written header at `-8`.
        unsafe { self.0.sub(HEADER_DESCRIPTOR_NEG_OFFSET) }
    }

    /// The header's size word.
    #[inline(always)]
    fn size_slot(self) -> MemAddr {
        // SAFETY: `Obj::new` guarantees an allocator-written header at `-8`.
        unsafe { self.0.sub(HEADER_SIZE_NEG_OFFSET) }
    }

    /// The object's header.
    #[inline(always)]
    pub fn header(self) -> ObjHeader {
        // SAFETY: `Obj::new` guarantees an allocator-written header: both
        // words are initialized and aligned.
        unsafe {
            ObjHeader {
                descriptor: self.descriptor_slot().load::<u32>(),
                size: self.size_slot().load::<u32>(),
            }
        }
    }

    /// The field at `bytes` from the data start.
    ///
    /// # Safety
    ///
    /// The field must lie within the object.
    #[inline(always)]
    pub unsafe fn field(self, bytes: usize) -> MemAddr {
        // SAFETY: forwarded to the caller.
        unsafe { self.0.add(bytes) }
    }

    /// The start of a vector's element data.
    ///
    /// # Safety
    ///
    /// The object must be a vector.
    #[inline(always)]
    pub unsafe fn vec_data(self) -> MemAddr {
        // SAFETY: forwarded to the caller: a vector's data starts at
        // `VEC_DATA_OFFSET`, within the object.
        unsafe { self.0.add(VEC_DATA_OFFSET) }
    }

    /// The `idx`-th element of a vector with `elem_size`-byte elements; the
    /// address form of [`vec_elem_offset`].
    ///
    /// # Safety
    ///
    /// The object must be such a vector and `idx` within its allocation.
    #[inline(always)]
    pub unsafe fn vec_elem(self, idx: u64, elem_size: u32) -> MemAddr {
        // SAFETY: forwarded to the caller.
        unsafe { self.vec_data().add(idx as usize * elem_size as usize) }
    }

    /// The number of `elem_size`-byte elements a vector's allocation holds:
    /// the bytes after the header and length word, divided by the element
    /// size. Meaningful only for a vector of such elements.
    #[inline(always)]
    pub fn vec_capacity(self, elem_size: u32) -> u64 {
        ((self.header().size as usize - OBJECT_HEADER_SIZE - VEC_DATA_OFFSET) / elem_size as usize)
            as u64
    }

    /// A vector's length word.
    ///
    /// # Safety
    ///
    /// The object must be a vector.
    #[inline(always)]
    pub unsafe fn vec_len(self) -> u64 {
        // SAFETY: forwarded to the caller: a vector's length word is at offset 0.
        unsafe { self.0.add(VEC_LENGTH_OFFSET).load::<u64>() }
    }

    /// Sets a vector's length word.
    ///
    /// # Safety
    ///
    /// The object must be a vector.
    #[inline(always)]
    pub unsafe fn set_vec_len(self, len: u64) {
        // SAFETY: forwarded to the caller.
        unsafe { self.0.add(VEC_LENGTH_OFFSET).store::<u64>(len) }
    }

    /// An enum object's variant tag.
    ///
    /// # Safety
    ///
    /// The object must be an enum.
    #[inline(always)]
    pub unsafe fn enum_tag(self) -> u64 {
        // SAFETY: forwarded to the caller.
        unsafe { self.0.add(ENUM_TAG_OFFSET).load::<u64>() }
    }

    /// Sets an enum object's variant tag.
    ///
    /// # Safety
    ///
    /// The object must be an enum.
    #[inline(always)]
    pub unsafe fn set_enum_tag(self, tag: u64) {
        // SAFETY: forwarded to the caller.
        unsafe { self.0.add(ENUM_TAG_OFFSET).store::<u64>(tag) }
    }

    // ----- Closure layout -----

    /// A closure's function reference as `(tag, payload)`: the tag is
    /// `FUNC_REF_TAG_RESOLVED` or `FUNC_REF_TAG_UNRESOLVED` and the payload
    /// is the address of the `Function` or interned `FunctionRef`.
    ///
    /// # Safety
    ///
    /// The object must be a closure.
    #[inline(always)]
    pub unsafe fn closure_func_ref(self) -> (u8, MemAddr) {
        // SAFETY: forwarded to the caller; both words are within the closure
        // layout.
        unsafe {
            (
                self.field(CLOSURE_FUNC_REF_OFFSET + FUNC_REF_TAG_OFFSET)
                    .load::<u8>(),
                self.field(CLOSURE_FUNC_REF_OFFSET + FUNC_REF_PAYLOAD_OFFSET)
                    .load_ptr(),
            )
        }
    }

    /// Writes a closure's function reference; see [`Obj::closure_func_ref`].
    ///
    /// # Safety
    ///
    /// The object must be a closure.
    #[inline(always)]
    pub unsafe fn set_closure_func_ref(self, tag: u8, payload: MemAddr) {
        // SAFETY: forwarded to the caller; both words are within the closure
        // layout.
        unsafe {
            self.field(CLOSURE_FUNC_REF_OFFSET + FUNC_REF_TAG_OFFSET)
                .store::<u8>(tag);
            self.field(CLOSURE_FUNC_REF_OFFSET + FUNC_REF_PAYLOAD_OFFSET)
                .store_ptr(payload);
        }
    }

    /// A closure's capture mask: bit `i` set means parameter `i` is captured.
    ///
    /// # Safety
    ///
    /// The object must be a closure.
    #[inline(always)]
    pub unsafe fn closure_mask(self) -> u64 {
        // SAFETY: forwarded to the caller; the word is within the closure layout.
        unsafe { self.field(CLOSURE_MASK_OFFSET).load::<u64>() }
    }

    /// A closure's captured-data object pointer; null before `PackClosure`
    /// has written it or when nothing is captured.
    ///
    /// # Safety
    ///
    /// The object must be a closure.
    #[inline(always)]
    pub unsafe fn closure_captured_data(self) -> MemAddr {
        // SAFETY: forwarded to the caller; the word is within the closure layout.
        unsafe { self.field(CLOSURE_CAPTURED_DATA_PTR_OFFSET).load_ptr() }
    }

    /// Sets a closure's captured-data object pointer.
    ///
    /// # Safety
    ///
    /// The object must be a closure.
    #[inline(always)]
    pub unsafe fn set_closure_captured_data(self, captured_data: MemAddr) {
        // SAFETY: forwarded to the caller; the word is within the closure layout.
        unsafe {
            self.field(CLOSURE_CAPTURED_DATA_PTR_OFFSET)
                .store_ptr(captured_data)
        }
    }

    /// Writes a closure's function reference and capture mask.
    ///
    /// # Safety
    ///
    /// The object must be a closure.
    #[inline]
    pub unsafe fn write_closure_func_ref_and_mask(self, func_ref: &ClosureFuncRef, mask: u64) {
        let (tag, payload) = match func_ref {
            ClosureFuncRef::Resolved(func_ptr) => (
                FUNC_REF_TAG_RESOLVED,
                MemAddr::from_typed_non_null(func_ptr.as_non_null()),
            ),
            ClosureFuncRef::Unresolved(func_ref) => (
                FUNC_REF_TAG_UNRESOLVED,
                MemAddr::from_typed_ptr(func_ref.as_raw_ptr()),
            ),
        };
        // SAFETY: forwarded to the caller; the mask word is within the closure
        // layout.
        unsafe {
            self.set_closure_func_ref(tag, payload);
            self.field(CLOSURE_MASK_OFFSET).store::<u64>(mask);
        }
    }

    // ----- Allocator / GC only -----

    /// Writes the header of a freshly reserved object and returns it.
    ///
    /// # Safety
    ///
    /// `data` must be the data start of a reservation that includes the
    /// header bytes below it.
    #[inline(always)]
    pub unsafe fn write_header(data: MemAddr, descriptor_id: DescriptorId, size: u32) -> Obj {
        // SAFETY: forwarded to the caller: the reservation includes the header
        // bytes, which is what `Obj::new` needs here.
        unsafe {
            let obj = Obj::new(data);
            obj.descriptor_slot().store::<u32>(descriptor_id.as_u32());
            obj.size_slot().store::<u32>(size);
            obj
        }
    }

    /// Overwrites the header's descriptor word. For the allocator and the
    /// GC only: user code never changes an object's descriptor.
    ///
    /// # Safety
    ///
    /// The caller must own the object for the rest of the operation that
    /// needs the new descriptor (e.g. a GC forwarding marker).
    #[inline(always)]
    pub unsafe fn set_descriptor(self, descriptor: u32) {
        // SAFETY: `Obj::new` guarantees an allocator-written header: the word
        // is aligned and within the reservation.
        unsafe { self.descriptor_slot().store::<u32>(descriptor) }
    }
}

/// Byte offset of the `idx`-th element of a vector with `elem_size`-byte
/// elements from the object's data start; the offset form of
/// [`Obj::vec_elem`].
#[inline(always)]
pub const fn vec_elem_offset(idx: u64, elem_size: u32) -> usize {
    VEC_DATA_OFFSET + idx as usize * elem_size as usize
}

/// A vector's length, where null is the empty vector.
///
/// # Safety
///
/// `ptr` must be null or a vector.
#[inline(always)]
pub unsafe fn vec_len_or_zero(ptr: MemAddr) -> u64 {
    if ptr.is_null() {
        0
    } else {
        // SAFETY: the caller guarantees a vector.
        unsafe { Obj::new(ptr).vec_len() }
    }
}

/// A `vector<u8>`'s bytes, where null is the empty vector.
///
/// # Safety
///
/// `ptr` must be null or a `vector<u8>` whose `len` bytes are initialized and
/// not written through any other path for `'a`.
#[inline(always)]
pub unsafe fn vec_bytes_or_empty<'a>(ptr: MemAddr) -> &'a [u8] {
    if ptr.is_null() {
        &[]
    } else {
        // SAFETY: the caller guarantees a `vector<u8>` with `len` initialized,
        // pointer-free bytes from the data start.
        unsafe {
            let vec = Obj::new(ptr);
            vec.vec_data().as_bytes(vec.vec_len() as usize)
        }
    }
}
