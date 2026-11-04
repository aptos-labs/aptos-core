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
//!
//! The types carry the VM-level memory checks (see `mono_move_mem::checks`):
//! a reference's offset range, an object pointer's shape, an object header's
//! self-consistency, field and element addresses against the payload, and a
//! frame's pointer and metadata. Each is attached with `mem_check!` and
//! compiles to nothing in the production tier.

use crate::{
    ClosureFuncRef, DescriptorId, FrameOffset, Function, CLOSURE_CAPTURED_DATA_PTR_OFFSET,
    CLOSURE_FUNC_REF_OFFSET, CLOSURE_MASK_OFFSET, ENUM_TAG_OFFSET, FRAME_METADATA_SIZE,
    FUNC_REF_PAYLOAD_OFFSET, FUNC_REF_TAG_OFFSET, FUNC_REF_TAG_RESOLVED, FUNC_REF_TAG_UNRESOLVED,
    MAX_ALIGN, OBJECT_HEADER_SIZE, VEC_DATA_OFFSET, VEC_LENGTH_OFFSET,
};
pub use mono_move_alloc::MemoryRegion;
use mono_move_mem::{
    checks::{MemFault, MemFaultKind},
    mem_check,
};
pub use mono_move_mem::{MemAddr, Scalar};

/// Marker written into the `descriptor_id` header word of an object the GC
/// has moved; the object's first data word then holds the forwarding
/// pointer. A header-format fact: outside a collection no live object
/// carries it.
pub const FORWARDED_MARKER: u32 = u32::MAX;

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
        // Hardened: `base + offset` does not wrap the address space.
        mem_check!(
            hardened,
            (self.base.as_usize() as u64)
                .checked_add(self.offset)
                .is_some(),
            MemFault::new(
                MemFaultKind::FatRefRange,
                self.base.as_usize(),
                0,
                self.offset as usize
            )
        );
        // SAFETY: forwarded to the caller.
        unsafe { self.base.add(self.offset as usize) }
    }

    /// A reference `delta` bytes further into the same object.
    #[inline(always)]
    pub fn with_added_offset(self, delta: u64) -> FatRef {
        // Hardened: the new offset neither overflows nor exceeds `u32::MAX`
        // (the verifier bounds static offsets to `u32`, and no object is
        // larger, so a wider offset cannot name a byte of its object).
        mem_check!(
            hardened,
            self.offset
                .checked_add(delta)
                .is_some_and(|o| o <= u32::MAX as u64),
            MemFault::new(
                MemFaultKind::FatRefRange,
                self.base.as_usize(),
                delta as usize,
                self.offset as usize
            )
        );
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
        // Hardened: every frame the call protocol places is `MAX_ALIGN`-aligned.
        mem_check!(
            hardened,
            fp.is_aligned_to(MAX_ALIGN),
            MemFault::new(MemFaultKind::BadFramePointer, fp.as_usize(), 0, MAX_ALIGN)
        );
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
        // Exhaustive: a non-root frame's caller sits below it (frames grow
        // upward), so the saved fp is strictly less than this frame's fp.
        mem_check!(
            exhaustive,
            meta.saved_func_ptr().is_null() || meta.saved_fp().as_usize() < self.0.as_usize(),
            MemFault::new(
                MemFaultKind::BadFrameMeta,
                meta.saved_fp().as_usize(),
                0,
                self.0.as_usize()
            )
        );
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
        let fp = unsafe { self.0.add(META_SAVED_FP_OFFSET).load_ptr() };
        // Hardened: the call protocol saves either null (root frame) or an
        // aligned frame pointer; null is trivially aligned.
        mem_check!(
            hardened,
            fp.is_aligned_to(MAX_ALIGN),
            MemFault::new(MemFaultKind::BadFrameMeta, fp.as_usize(), 0, MAX_ALIGN)
        );
        fp
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
        // Hardened: an object's data pointer is non-null and, because the
        // allocator reserves `OBJECT_HEADER_SIZE == MAX_ALIGN` bytes below
        // it, `MAX_ALIGN`-aligned.
        mem_check!(
            hardened,
            !data.is_null() && data.is_aligned_to(MAX_ALIGN),
            MemFault::new(
                MemFaultKind::BadObjectPointer,
                data.as_usize(),
                0,
                MAX_ALIGN
            )
        );
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
        let header = self.header_or_forwarded();
        // Hardened: outside a collection no live object is forwarded.
        mem_check!(
            hardened,
            header.descriptor != FORWARDED_MARKER,
            MemFault::new(
                MemFaultKind::BadHeader,
                self.0.as_usize(),
                header.size as usize,
                header.descriptor as usize
            )
        );
        header
    }

    /// The object's header, which may carry the GC's [`FORWARDED_MARKER`].
    /// For the collector, which legitimately meets forwarded objects while
    /// it runs; everything else uses [`Obj::header`].
    #[inline(always)]
    pub fn header_or_forwarded(self) -> ObjHeader {
        // SAFETY: `Obj::new` guarantees an allocator-written header: both
        // words are initialized and aligned.
        let header = unsafe {
            ObjHeader {
                descriptor: self.descriptor_slot().load::<u32>(),
                size: self.size_slot().load::<u32>(),
            }
        };
        // Hardened: the allocator writes the `MAX_ALIGN`-rounded total size,
        // which covers at least the header itself.
        mem_check!(
            hardened,
            header.size as usize >= OBJECT_HEADER_SIZE
                && (header.size as usize).is_multiple_of(MAX_ALIGN),
            MemFault::new(
                MemFaultKind::BadHeader,
                self.0.as_usize(),
                header.size as usize,
                header.descriptor as usize
            )
        );
        header
    }

    /// The payload size the header records: the bytes from the data start to
    /// the end of the object. For the checks below; a header load.
    #[inline(always)]
    fn payload_size(self) -> usize {
        self.header().size as usize - OBJECT_HEADER_SIZE
    }

    /// The field at `bytes` from the data start.
    ///
    /// # Safety
    ///
    /// The field must lie within the object.
    #[inline(always)]
    pub unsafe fn field(self, bytes: usize) -> MemAddr {
        // Hardened: the address is within, or one past the end of, the
        // payload.
        mem_check!(
            hardened,
            bytes <= self.payload_size(),
            MemFault::new(MemFaultKind::FieldOutOfPayload, self.0.as_usize(), 0, bytes)
        );
        // SAFETY: forwarded to the caller.
        unsafe { self.0.add(bytes) }
    }

    /// The field of `width` bytes at `off` from the data start; as
    /// [`Obj::field`], for a site that copies a known number of bytes.
    ///
    /// # Safety
    ///
    /// The `width` bytes at `off` must lie within the object.
    #[inline(always)]
    pub unsafe fn field_range(self, off: usize, width: usize) -> MemAddr {
        // Hardened: the whole range is within the payload.
        mem_check!(
            hardened,
            off.checked_add(width)
                .is_some_and(|end| end <= self.payload_size()),
            MemFault::new(
                MemFaultKind::FieldOutOfPayload,
                self.0.as_usize(),
                width,
                off
            )
        );
        // SAFETY: forwarded to the caller.
        unsafe { self.0.add(off) }
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
        unsafe { self.field(VEC_DATA_OFFSET) }
    }

    /// Hardened: `count` elements of `elem_size` bytes starting at `idx` lie
    /// within the payload. In `u64` so the check itself cannot overflow.
    #[inline(always)]
    fn check_vec_range(self, idx: u64, count: u64, elem_size: u32) {
        mem_check!(
            hardened,
            idx.checked_add(count)
                .and_then(|end| end.checked_mul(u64::from(elem_size)))
                .and_then(|bytes| bytes.checked_add(VEC_DATA_OFFSET as u64))
                .is_some_and(|end| end <= self.payload_size() as u64),
            MemFault::new(
                MemFaultKind::VectorOutOfPayload,
                self.0.as_usize(),
                elem_size as usize,
                idx as usize
            )
        );
    }

    /// The `idx`-th element of a vector with `elem_size`-byte elements; the
    /// address form of [`vec_elem_offset`].
    ///
    /// # Safety
    ///
    /// The object must be such a vector and `idx` within its allocation.
    #[inline(always)]
    pub unsafe fn vec_elem(self, idx: u64, elem_size: u32) -> MemAddr {
        self.check_vec_range(idx, 1, elem_size);
        // Two pointer adds rather than `add(vec_elem_offset(..))`: a pointer
        // add has no overflow check, so this is one `lea` in release.
        // SAFETY: forwarded to the caller.
        unsafe {
            self.0
                .add(VEC_DATA_OFFSET)
                .add(idx as usize * elem_size as usize)
        }
    }

    /// The start of the `count` elements at `idx` of a vector with
    /// `elem_size`-byte elements. Unlike [`Obj::vec_elem`], a `count` of
    /// zero may name the one-past-the-end position of a full allocation,
    /// as the empty end of a range copy does.
    ///
    /// # Safety
    ///
    /// The object must be such a vector and the `count` elements at `idx`
    /// within its allocation.
    #[inline(always)]
    pub unsafe fn vec_range(self, idx: u64, count: u64, elem_size: u32) -> MemAddr {
        self.check_vec_range(idx, count, elem_size);
        // SAFETY: forwarded to the caller.
        unsafe {
            self.0
                .add(VEC_DATA_OFFSET)
                .add(idx as usize * elem_size as usize)
        }
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
        // Hardened: `len` elements of at least one byte each fit in the
        // payload. A weak bound (the element size is unknown here), but a
        // free one.
        mem_check!(
            hardened,
            (VEC_DATA_OFFSET as u64)
                .checked_add(len)
                .is_some_and(|end| end <= self.payload_size() as u64),
            MemFault::new(
                MemFaultKind::VectorOutOfPayload,
                self.0.as_usize(),
                0,
                len as usize
            )
        );
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
    /// No memory check here: the VM validates the tag and a null payload
    /// itself (`exec_call_closure` reports an invariant violation).
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

#[cfg(test)]
mod tests {
    use super::*;
    use mono_move_mem::checks::{take_last_fault, EXHAUSTIVE, HARDENED};

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

        /// An object of total `size` bytes laid out at the start of the
        /// buffer: the header in the first 8 bytes, the data after it.
        fn obj(&mut self, size: u32) -> Obj {
            assert!(OBJECT_HEADER_SIZE + size as usize <= N);
            // SAFETY: the buffer holds the header bytes and `size` more.
            unsafe { Obj::write_header(self.addr().add(OBJECT_HEADER_SIZE), DescriptorId(1), size) }
        }
    }

    #[test]
    fn header_round_trip() {
        let mut buf = Buf::<40>::zeroed();
        // Payload 24: a vector of two 8-byte elements.
        let obj = buf.obj(32);
        assert_eq!(obj.header(), ObjHeader {
            descriptor: 1,
            size: 32
        });
        assert_eq!(obj.header_or_forwarded(), obj.header());
        unsafe {
            obj.set_vec_len(3);
            assert_eq!(obj.vec_len(), 3);
            assert_eq!(obj.vec_elem(1, 8), obj.vec_data().add(8));
            assert_eq!(obj.field(16), obj.addr().add(16));
            assert_eq!(obj.field_range(8, 8), obj.field(8));
            assert_eq!(obj.vec_range(1, 0, 8), obj.vec_elem(1, 8));
        }
    }

    #[test]
    fn frame_meta_round_trip() {
        let mut buf = Buf::<64>::zeroed();
        let base = buf.addr();
        unsafe {
            let fp = base.add(FRAME_METADATA_SIZE);
            FrameMeta::write_at(base, 7, base, MemAddr::NULL);
            let frame = Frame::new(fp);
            assert_eq!(frame.fp(), fp);
            assert_eq!(frame.meta().saved_pc_word(), 7);
            assert_eq!(frame.meta().saved_fp(), base);
            assert!(frame.meta().saved_func_ptr().is_null());
            assert!(frame.caller().is_none());
        }
    }

    // ----- hardened-tier negative tests (no-ops in the production tier) -----

    /// Runs `f` and returns the fault it raised, or `None` if checks are
    /// not compiled in.
    fn expect_fault(f: impl FnOnce()) -> Option<MemFaultKind> {
        if !HARDENED {
            return None;
        }
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        assert!(r.is_err(), "expected a memory fault");
        Some(take_last_fault().expect("fault recorded").kind)
    }

    fn assert_fault(expected: MemFaultKind, f: impl FnOnce()) {
        if let Some(kind) = expect_fault(f) {
            assert_eq!(kind, expected);
        }
    }

    #[test]
    fn hardened_bad_header_faults() {
        let mut buf = Buf::<32>::zeroed();
        // Size zero.
        let obj = buf.obj(0);
        assert_fault(MemFaultKind::BadHeader, || {
            obj.header();
        });
        // Size not a multiple of `MAX_ALIGN`.
        let obj = buf.obj(12);
        assert_fault(MemFaultKind::BadHeader, || {
            obj.header();
        });
        // Forwarded outside a collection: `header` faults, the GC accessor
        // does not.
        let obj = buf.obj(16);
        unsafe { obj.set_descriptor(FORWARDED_MARKER) };
        assert_fault(MemFaultKind::BadHeader, || {
            obj.header();
        });
        assert_eq!(obj.header_or_forwarded().descriptor, FORWARDED_MARKER);
    }

    #[test]
    fn hardened_vector_out_of_payload_faults() {
        let mut buf = Buf::<32>::zeroed();
        // Payload 16: the length word and one 8-byte element.
        let obj = buf.obj(24);
        unsafe {
            obj.vec_elem(0, 8);
            obj.vec_range(1, 0, 8);
            obj.set_vec_len(8);
        }
        assert_fault(MemFaultKind::VectorOutOfPayload, || unsafe {
            obj.vec_elem(1, 8);
        });
        assert_fault(MemFaultKind::VectorOutOfPayload, || unsafe {
            obj.vec_elem(0, 9);
        });
        assert_fault(MemFaultKind::VectorOutOfPayload, || unsafe {
            obj.vec_range(0, 2, 8);
        });
        assert_fault(MemFaultKind::VectorOutOfPayload, || unsafe {
            obj.vec_elem(u64::MAX, 8);
        });
        assert_fault(MemFaultKind::VectorOutOfPayload, || unsafe {
            obj.set_vec_len(9);
        });
        assert_fault(MemFaultKind::VectorOutOfPayload, || unsafe {
            obj.set_vec_len(u64::MAX);
        });
    }

    #[test]
    fn hardened_field_out_of_payload_faults() {
        let mut buf = Buf::<32>::zeroed();
        // Payload 16.
        let obj = buf.obj(24);
        unsafe {
            obj.field(16);
            obj.field_range(8, 8);
            obj.field_range(16, 0);
        }
        assert_fault(MemFaultKind::FieldOutOfPayload, || unsafe {
            obj.field(17);
        });
        assert_fault(MemFaultKind::FieldOutOfPayload, || unsafe {
            obj.field_range(8, 9);
        });
        assert_fault(MemFaultKind::FieldOutOfPayload, || unsafe {
            obj.field_range(usize::MAX, 1);
        });
        // A vector's data start inherits the check: payload 0 has no room.
        let obj = buf.obj(8);
        assert_fault(MemFaultKind::FieldOutOfPayload, || unsafe {
            obj.vec_data();
        });
    }

    #[test]
    fn hardened_bad_object_pointer_faults() {
        let mut buf = Buf::<32>::zeroed();
        let base = buf.addr();
        assert_fault(MemFaultKind::BadObjectPointer, || unsafe {
            Obj::new(MemAddr::NULL);
        });
        assert_fault(MemFaultKind::BadObjectPointer, || unsafe {
            Obj::new(base.add(OBJECT_HEADER_SIZE + 1));
        });
        assert_fault(MemFaultKind::BadObjectPointer, || unsafe {
            Obj::write_header(base.add(OBJECT_HEADER_SIZE + 4), DescriptorId(1), 8);
        });
    }

    #[test]
    fn hardened_fat_ref_range_faults() {
        let mut buf = Buf::<32>::zeroed();
        let base = buf.addr();
        // Offset arithmetic overflows.
        assert_fault(MemFaultKind::FatRefRange, || {
            FatRef::new(base, u64::MAX).with_added_offset(1);
        });
        // Offset leaves the `u32` range.
        assert_fault(MemFaultKind::FatRefRange, || {
            FatRef::new(base, u64::from(u32::MAX)).with_added_offset(1);
        });
        FatRef::new(base, u64::from(u32::MAX) - 1).with_added_offset(1);
        // `base + offset` wraps the address space; the address is only
        // formed, never dereferenced.
        let high = MemAddr::from_ptr(std::ptr::without_provenance_mut(usize::MAX - 7));
        assert_fault(MemFaultKind::FatRefRange, || unsafe {
            FatRef::new(high, 16).target();
        });
    }

    #[test]
    fn hardened_bad_frame_pointer_faults() {
        let mut buf = Buf::<64>::zeroed();
        let base = buf.addr();
        assert_fault(MemFaultKind::BadFramePointer, || unsafe {
            Frame::new(base.add(FRAME_METADATA_SIZE + 4));
        });
    }

    #[test]
    fn hardened_bad_frame_meta_faults() {
        let mut buf = Buf::<64>::zeroed();
        let base = buf.addr();
        unsafe {
            // A misaligned saved fp cannot have been written by the call
            // protocol.
            FrameMeta::write_at(base, 0, base.add(4), MemAddr::NULL);
            let frame = Frame::new(base.add(FRAME_METADATA_SIZE));
            assert_fault(MemFaultKind::BadFrameMeta, || {
                frame.meta().saved_fp();
            });
        }
    }

    #[test]
    fn exhaustive_caller_above_frame_faults() {
        if !EXHAUSTIVE {
            return;
        }
        let mut buf = Buf::<64>::zeroed();
        let base = buf.addr();
        unsafe {
            let fp = base.add(FRAME_METADATA_SIZE);
            // A non-root frame (non-null saved function word; the check fires
            // before the word is turned into a `Function`) whose saved fp is
            // not below it.
            FrameMeta::write_at(base, 0, fp, base);
            let frame = Frame::new(fp);
            assert_fault(MemFaultKind::BadFrameMeta, || {
                frame.caller();
            });
            FrameMeta::write_at(base, 0, fp.add(8), base);
            assert_fault(MemFaultKind::BadFrameMeta, || {
                frame.caller();
            });
        }
    }
}
