// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Runtime instruction set.
//!
//! # Design decisions to revisit
//!
//! - **Addressing modes**: most operands are fp-relative offsets today.
//!   Secondary modes: immediate (`*Const`), pointer + static-offset (`ObjLoad`),
//!   pointer + dynamic-offset (`VecLoadElem`).
//!     - Right now, offsets are `u32`. Do we want something more packed?
//!     - Do we need other addressing modes?
//!     - Which instructions should support which addressing modes?
//!
//! - **Size specialization**: 8-byte variants (`Mov8`, `ObjLoad8`, `ObjStore8`)
//!   fast-path the common primitive/pointer size. May want `Mov16` (fat
//!   pointers) and others.
//!
//! - **Branch design**: fused compare-and-branch (current) vs separate
//!   `Cmp` + `BranchTrue`/`BranchFalse`. Fused is standard for bytecode VMs.
//!
//! - **Immediate representation**: instructions like `StoreConst8` carry a
//!   `u64`, but the same instruction is used for other 8-byte types (addresses,
//!   bools, pointers). Should immediates be `u64` or `[u8; 8]`? `u64` is
//!   convenient but conflates the bit pattern with an integer type.
//!
//! - **Endianness**: the VM currently assumes native byte order. If
//!   instructions or memory layouts need to be portable across architectures,
//!   we'll be in trouble.
//!
//! - **Encoding**: Rust enum is convenient for prototyping but may not be optimal.
//!   Revisit for production.

#[derive(Debug)]
pub enum Instruction {
    //======================================================================
    // Data movement
    //======================================================================
    // Move data between frame slots or store constants.
    // `Mov8` fast-paths 8-byte values; `Mov` handles arbitrary sizes.
    //
    // May want:
    // - `Mov16` (fat pointers) + other sizes,
    // - `StoreConst` to handle arbitrary sizes.
    //======================================================================
    /// Store an immediate u64 (8 bytes) at `dst_fp_offset` from the current frame pointer.
    StoreConst8 { dst_fp_offset: u32, val: u64 },

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

    //======================================================================
    // Arithmetic & bitwise
    //======================================================================
    // Currently u64-only. Each op has a reg-reg form and/or an immediate
    // form; we add variants as the compiler needs them.
    //
    // May want:
    // - Mul, Div, Sub (reg-reg), Negate, bitwise And/Or/Xor/Not, Shl,
    // - u8, u16, u32 variants (mask on u64?), u128/u256 (multi-word),
    // - signed integer support.
    //======================================================================
    /// `dst = src1 + src2` (u64, checked).
    AddU64 {
        src_fp_offset_1: u32,
        src_fp_offset_2: u32,
        dst_fp_offset: u32,
    },

    /// `dst = src + val` (u64, checked).
    AddU64Const {
        src_fp_offset: u32,
        val: u64,
        dst_fp_offset: u32,
    },

    /// `dst = src - val` (u64, checked).
    SubU64Const {
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

    /// `dst = lhs % rhs` (u64 remainder). Panics on division by zero.
    RemU64 {
        lhs_fp_offset: u32,
        rhs_fp_offset: u32,
        dst_fp_offset: u32,
    },

    //======================================================================
    // Control flow
    //======================================================================
    // Fused compare-and-branch (no separate cmp + flags).
    //
    // May want:
    // - Abort,
    // - more conditions: ==, !=, >, <=, and const variants,
    // - something for enum dispatch (jump table)?
    //======================================================================
    /// Call function `func_id`. Frame metadata is saved at
    /// current fp + data_size, and callee fp = current fp + data_size
    /// + FRAME_METADATA_SIZE.
    CallFunc { func_id: usize },

    /// Return from the current function call.
    Return,

    /// Unconditional jump.
    Jump { dst_pc: u32 },

    /// Jump to `dst_pc` if the u64 at `src_fp_offset` is **not** zero.
    JumpIfNotZero { src_fp_offset: u32, dst_pc: u32 },

    /// Jump to `dst_pc` if the u64 at `src_fp_offset` is **>=** `val`.
    JumpIfGreaterEqualU64Const {
        src_fp_offset: u32,
        dst_pc: u32,
        val: u64,
    },

    /// Jump to `dst_pc` if u64 at `lhs_fp_offset` < u64 at `rhs_fp_offset`.
    JumpIfLessU64 {
        lhs_fp_offset: u32,
        rhs_fp_offset: u32,
        dst_pc: u32,
    },

    //======================================================================
    // Vector operations
    //======================================================================
    // Heap-allocated: [header | length | capacity | elements...].
    // `elem_size` is baked into each instruction (statically known).
    //
    // May want:
    // - VecSwap (native in Move's vector module),
    // - VecMoveRange (bulk move between vectors) — tricky: this is
    //   really a memcpy between two heap-allocated regions, but we
    //   currently have no vector-to-vector addressing mode.
    //======================================================================
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

    //======================================================================
    // Reference (fat pointer) operations
    //======================================================================
    // References are fat pointers: (base: *mut u8, offset: u64).
    // Fat pointers keep the base visible to GC; thin pointers would be
    // cheaper but GC couldn't identify the owning object.
    //
    // May want:
    // - Specializations (e.g. ReadRef8/WriteRef8)
    //======================================================================
    /// Borrow a stack-local slot, producing a fat pointer `(base, offset)`.
    /// Writes 16 bytes at `[dst_fp_offset, dst_fp_offset+16)`:
    ///   - base   = fp + local_fp_offset (a stack address)
    ///   - offset = 0
    BorrowLocal {
        local_fp_offset: u32,
        dst_fp_offset: u32,
    },

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

    /// Borrow a location within a heap object, producing a fat pointer
    /// `(obj_ptr, offset)`. Writes 16 bytes at `[dst_fp_offset, dst_fp_offset+16)`:
    ///   - base   = the object's heap pointer
    ///   - offset = offset from the object's start
    ObjBorrow {
        obj_fp_offset: u32,
        offset: u32,
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

    //======================================================================
    // Heap object operations (structs and enums)
    //======================================================================
    // Structs and enums are both heap objects. The interpreter treats them
    // uniformly as ptr+offset load/store — the compiler helpers (below)
    // bake in the right offsets for each.
    //
    // `ObjLoad8`/`ObjStore8` specialize for 8-byte fields.
    //
    // May want:
    // - fused Pack/Unpack,
    // - ObjLoad16/ObjStore16,
    //======================================================================
    /// Allocate a new heap object. Size is determined by the `Struct` or
    /// `Enum` descriptor at `descriptor_id`. Writes the heap pointer to
    /// `dst_fp_offset`. **MAY TRIGGER GC.**
    ///
    /// Note: the allocated memory is not zeroed or initialized — the caller
    /// must store into every field before reading. This is potentially
    /// dangerous; revisit whether ObjNew should zero-initialize, or use a
    /// fused Pack instruction that allocates and initializes in one step.
    ObjNew {
        descriptor_id: u16,
        dst_fp_offset: u32,
    },

    /// Read 8 bytes from a heap object at `obj_ptr + offset` into `dst_fp_offset`.
    ObjLoad8 {
        obj_fp_offset: u32,
        offset: u32,
        dst_fp_offset: u32,
    },

    /// Read `size` bytes from a heap object at `obj_ptr + offset` into `dst_fp_offset`.
    ObjLoad {
        obj_fp_offset: u32,
        offset: u32,
        dst_fp_offset: u32,
        size: u32,
    },

    /// Write 8 bytes from `src_fp_offset` into a heap object at `obj_ptr + offset`.
    ObjStore8 {
        obj_fp_offset: u32,
        offset: u32,
        src_fp_offset: u32,
    },

    /// Write an immediate u64 into a heap object at `obj_ptr + offset`.
    ObjStoreConst8 {
        obj_fp_offset: u32,
        offset: u32,
        val: u64,
    },

    /// Write `size` bytes from `src_fp_offset` into a heap object at `obj_ptr + offset`.
    ObjStore {
        obj_fp_offset: u32,
        offset: u32,
        src_fp_offset: u32,
        size: u32,
    },

    //======================================================================
    // Debugging
    //======================================================================
    /// Advance the interpreter's RNG and write a random u64 to `dst_fp_offset`.
    StoreRandomU64 { dst_fp_offset: u32 },

    /// Unconditionally trigger a garbage collection cycle.
    /// Requires a stack map entry at this PC. Useful for testing.
    ForceGC,
    //======================================================================
    // Missing instructions
    //======================================================================
    // - **Comparison**: Eq, Neq, Lt, Gt, Le, Ge (standalone, not fused
    //   with branch — for when the result is used as a value).
    // - **Casting**: truncation and widening between integer types,
    //   including signed casts.
    // - **Boolean**: logical Not, And, Or (distinct from bitwise).
    // - **Global storage**: MoveTo, MoveFrom, BorrowGlobal, Exists.
    // - **Closures / function values**
    // - **Gas metering**: explicit charge points.
    // - **Runtime instrumentation**: tracing, profiling, coverage hooks.
    //======================================================================
}

// ---------------------------------------------------------------------------
// Constructor helpers for the re-compiler
// ---------------------------------------------------------------------------

/// Size of the object header: [descriptor_id: u32 | size_in_bytes: u32].
const OBJECT_HEADER_SIZE: usize = 8;

/// Offset where struct field data begins (same as OBJECT_HEADER_SIZE).
const STRUCT_DATA_OFFSET: usize = OBJECT_HEADER_SIZE; // 8

/// Offset of the variant tag (u64) within an enum object.
const ENUM_TAG_OFFSET: usize = OBJECT_HEADER_SIZE; // 8

/// Offset where enum variant field data begins (after header + tag).
const ENUM_DATA_OFFSET: usize = OBJECT_HEADER_SIZE + 8; // 16

impl Instruction {
    // ----- Struct helpers (offsets relative to STRUCT_DATA_OFFSET) -----

    pub fn struct_load8(struct_fp_offset: u32, field_offset: u32, dst_fp_offset: u32) -> Self {
        Instruction::ObjLoad8 {
            obj_fp_offset: struct_fp_offset,
            offset: STRUCT_DATA_OFFSET as u32 + field_offset,
            dst_fp_offset,
        }
    }

    pub fn struct_load(
        struct_fp_offset: u32,
        field_offset: u32,
        dst_fp_offset: u32,
        size: u32,
    ) -> Self {
        Instruction::ObjLoad {
            obj_fp_offset: struct_fp_offset,
            offset: STRUCT_DATA_OFFSET as u32 + field_offset,
            dst_fp_offset,
            size,
        }
    }

    pub fn struct_store8(struct_fp_offset: u32, field_offset: u32, src_fp_offset: u32) -> Self {
        Instruction::ObjStore8 {
            obj_fp_offset: struct_fp_offset,
            offset: STRUCT_DATA_OFFSET as u32 + field_offset,
            src_fp_offset,
        }
    }

    pub fn struct_store(
        struct_fp_offset: u32,
        field_offset: u32,
        src_fp_offset: u32,
        size: u32,
    ) -> Self {
        Instruction::ObjStore {
            obj_fp_offset: struct_fp_offset,
            offset: STRUCT_DATA_OFFSET as u32 + field_offset,
            src_fp_offset,
            size,
        }
    }

    pub fn struct_borrow(struct_fp_offset: u32, field_offset: u32, dst_fp_offset: u32) -> Self {
        Instruction::ObjBorrow {
            obj_fp_offset: struct_fp_offset,
            offset: STRUCT_DATA_OFFSET as u32 + field_offset,
            dst_fp_offset,
        }
    }

    // ----- Enum helpers (offsets relative to ENUM_DATA_OFFSET) -----

    pub fn enum_get_tag(enum_fp_offset: u32, dst_fp_offset: u32) -> Self {
        Instruction::ObjLoad8 {
            obj_fp_offset: enum_fp_offset,
            offset: ENUM_TAG_OFFSET as u32,
            dst_fp_offset,
        }
    }

    pub fn enum_set_tag(enum_fp_offset: u32, variant: u16) -> Self {
        Instruction::ObjStoreConst8 {
            obj_fp_offset: enum_fp_offset,
            offset: ENUM_TAG_OFFSET as u32,
            val: variant as u64,
        }
    }

    pub fn enum_load8(enum_fp_offset: u32, field_offset: u32, dst_fp_offset: u32) -> Self {
        Instruction::ObjLoad8 {
            obj_fp_offset: enum_fp_offset,
            offset: ENUM_DATA_OFFSET as u32 + field_offset,
            dst_fp_offset,
        }
    }

    pub fn enum_load(
        enum_fp_offset: u32,
        field_offset: u32,
        dst_fp_offset: u32,
        size: u32,
    ) -> Self {
        Instruction::ObjLoad {
            obj_fp_offset: enum_fp_offset,
            offset: ENUM_DATA_OFFSET as u32 + field_offset,
            dst_fp_offset,
            size,
        }
    }

    pub fn enum_store8(enum_fp_offset: u32, field_offset: u32, src_fp_offset: u32) -> Self {
        Instruction::ObjStore8 {
            obj_fp_offset: enum_fp_offset,
            offset: ENUM_DATA_OFFSET as u32 + field_offset,
            src_fp_offset,
        }
    }

    pub fn enum_store(
        enum_fp_offset: u32,
        field_offset: u32,
        src_fp_offset: u32,
        size: u32,
    ) -> Self {
        Instruction::ObjStore {
            obj_fp_offset: enum_fp_offset,
            offset: ENUM_DATA_OFFSET as u32 + field_offset,
            src_fp_offset,
            size,
        }
    }

    pub fn enum_borrow(enum_fp_offset: u32, field_offset: u32, dst_fp_offset: u32) -> Self {
        Instruction::ObjBorrow {
            obj_fp_offset: enum_fp_offset,
            offset: ENUM_DATA_OFFSET as u32 + field_offset,
            dst_fp_offset,
        }
    }
}
