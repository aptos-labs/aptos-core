// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Bytecode instruction set.

#[derive(Debug)]
pub enum Instruction {
    /// Store an immediate u64 value at `dst_fp_offset` from the current frame pointer.
    StoreU64 { dst_fp_offset: u32, val: u64 },

    /// `dst = src - val` (u64, checked).
    SubU64Const {
        src_fp_offset: u32,
        val: u64,
        dst_fp_offset: u32,
    },

    /// `dst = src1 + src2` (u64, checked).
    AddU64 {
        src_fp_offset_1: u32,
        src_fp_offset_2: u32,
        dst_fp_offset: u32,
    },

    /// Copy 8 bytes from `src_fp_offset` to `dst_fp_offset`.
    Mov8 {
        src_fp_offset: u32,
        dst_fp_offset: u32,
    },

    /// Copy `size` bytes from `src_fp_offset` to `dst_fp_offset`.
    Mov {
        src_fp_offset: u32,
        dst_fp_offset: u32,
        size: u32,
    },

    /// Call function `func_id`. Frame metadata is saved at
    /// current fp + data_size, and callee fp = current fp + data_size
    /// + FRAME_METADATA_SIZE.
    CallFunc { func_id: usize },

    /// Jump to `dst_pc` if the u64 at `src_fp_offset` is **not** zero.
    JumpIfNotZero { src_fp_offset: u32, dst_pc: u32 },

    /// Jump to `dst_pc` if the u64 at `src_fp_offset` is **>=** `val`.
    JumpIfGreaterEqualU64Const {
        src_fp_offset: u32,
        dst_pc: u32,
        val: u64,
    },

    /// `dst = src + val` (u64, checked).
    AddU64Const {
        src_fp_offset: u32,
        val: u64,
        dst_fp_offset: u32,
    },

    /// `dst = src >> val` (u64, logical right shift).
    ShrU64Const {
        src_fp_offset: u32,
        val: u64,
        dst_fp_offset: u32,
    },

    /// Jump to `dst_pc` if u64 at `lhs_fp_offset` < u64 at `rhs_fp_offset`.
    JumpIfLessU64 {
        lhs_fp_offset: u32,
        rhs_fp_offset: u32,
        dst_pc: u32,
    },

    /// Unconditional jump.
    Jump { dst_pc: u32 },

    /// `dst = lhs % rhs` (u64 remainder). Panics on division by zero.
    RemU64 {
        lhs_fp_offset: u32,
        rhs_fp_offset: u32,
        dst_fp_offset: u32,
    },

    /// Advance the interpreter's RNG and write a random u64 to `dst_fp_offset`.
    StoreRandomU64 { dst_fp_offset: u32 },

    /// Return from the current function call.
    Return,

    /// Unconditionally trigger a garbage collection cycle.
    /// Requires a stack map entry at this PC. Useful for testing.
    ForceGC,

    // ----- Vector instructions -----
    /// Allocate a new empty vector with the given initial capacity.
    /// `descriptor_id = 0` means trivial elements (no refs); >= 1 indexes
    /// the object descriptor table. Writes heap pointer to `dst_fp_offset`.
    /// MAY TRIGGER GC.
    VecNew {
        descriptor_id: u16,
        elem_size: u32,
        initial_capacity: u64,
        dst_fp_offset: u32,
    },

    /// Write the length (u64) of the vector to `dst_fp_offset`.
    VecLen {
        vec_fp_offset: u32,
        dst_fp_offset: u32,
    },

    /// Append an element. Copies `elem_size` bytes from `elem_fp_offset`
    /// into the vector. If capacity is exceeded, reallocates (bump) and
    /// updates `vec_fp_offset` in place. MAY TRIGGER GC.
    VecPushBack {
        vec_fp_offset: u32,
        elem_fp_offset: u32,
        elem_size: u32,
    },

    /// Pop last element. Copies `elem_size` bytes to `dst_fp_offset`.
    /// Aborts if empty.
    VecPopBack {
        vec_fp_offset: u32,
        dst_fp_offset: u32,
        elem_size: u32,
    },

    /// Read vector[idx]. Copies `elem_size` bytes to `dst_fp_offset`.
    /// Aborts if out of bounds.
    VecLoadElem {
        vec_fp_offset: u32,
        idx_fp_offset: u32,
        dst_fp_offset: u32,
        elem_size: u32,
    },

    /// Write vector[idx]. Copies `elem_size` bytes from `src_fp_offset`.
    /// Aborts if out of bounds.
    VecStoreElem {
        vec_fp_offset: u32,
        idx_fp_offset: u32,
        src_fp_offset: u32,
        elem_size: u32,
    },

    // ----- Reference (fat pointer) instructions -----

    /// Borrow a vector element, producing a fat pointer `(base, offset)`.
    /// Writes 16 bytes at `[dst_fp_offset, dst_fp_offset+16)`:
    ///   - base   = the vector's heap pointer
    ///   - offset = VEC_DATA_OFFSET + idx * elem_size
    ///
    /// Aborts if index is out of bounds.
    VecBorrow {
        vec_fp_offset: u32,
        idx_fp_offset: u32,
        elem_size: u32,
        dst_fp_offset: u32,
    },

    /// Borrow a stack-local slot, producing a fat pointer `(base, offset)`.
    /// Writes 16 bytes at `[dst_fp_offset, dst_fp_offset+16)`:
    ///   - base   = fp + local_fp_offset (a stack address)
    ///   - offset = 0
    BorrowLocal {
        local_fp_offset: u32,
        dst_fp_offset: u32,
    },

    /// Read through a fat pointer. Copies `size` bytes from the
    /// referenced location `(base + offset)` to `dst_fp_offset`.
    ReadRef {
        ref_fp_offset: u32,
        dst_fp_offset: u32,
        size: u32,
    },

    /// Write through a fat pointer. Copies `size` bytes from
    /// `src_fp_offset` to the referenced location `(base + offset)`.
    WriteRef {
        ref_fp_offset: u32,
        src_fp_offset: u32,
        size: u32,
    },

    // ----- Struct instructions (heap-allocated) -----

    /// Allocate a new zeroed struct on the heap. Size is determined by the
    /// `Struct` descriptor at `descriptor_id`. Writes the heap pointer to
    /// `dst_fp_offset`. **MAY TRIGGER GC.**
    StructNew {
        descriptor_id: u16,
        dst_fp_offset: u32,
    },

    /// Read a field from a heap struct. Copies `size` bytes from
    /// `*(struct_ptr + STRUCT_DATA_OFFSET + field_offset)` to `fp + dst_fp_offset`.
    StructLoadField {
        struct_fp_offset: u32,
        field_offset: u32,
        dst_fp_offset: u32,
        size: u32,
    },

    /// Write a field into a heap struct. Copies `size` bytes from
    /// `fp + src_fp_offset` to `*(struct_ptr + STRUCT_DATA_OFFSET + field_offset)`.
    StructStoreField {
        struct_fp_offset: u32,
        field_offset: u32,
        src_fp_offset: u32,
        size: u32,
    },

    /// Borrow a field of a heap struct, producing a fat pointer
    /// `(struct_ptr, STRUCT_DATA_OFFSET + field_offset)`.
    /// Writes 16 bytes at `dst_fp_offset`.
    StructBorrow {
        struct_fp_offset: u32,
        field_offset: u32,
        dst_fp_offset: u32,
    },
}
