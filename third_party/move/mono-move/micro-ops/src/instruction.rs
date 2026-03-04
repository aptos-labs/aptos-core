// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! # Runtime Instruction Set (Micro-ops)
//!
//! Defines the low-level instruction set that the MonoMove interpreter
//! executes. These micro-ops are emitted by the runtime compiler
//! (monomorphizer) from Move bytecode. They operate on a flat frame-pointer-
//! relative memory model — no operand stack, no type metadata at runtime.
//!
//! ## Design overview
//!
//! - **Highly specialized**: we are not aiming for a minimal set of micro-ops.
//!   Specialized variants (e.g. `HeapMoveFrom8` for the common 8-byte case)
//!   are preferred when they enable faster dispatch or more efficient interpreter code.
//!
//! - **Fixed-size micro-ops**: each [`MicroOp`] variant should fit in a fixed
//!   number of bytes so side-tables (e.g. source maps, gas tables) can be
//!   indexed by program counter without indirection. Current size is 24 bytes;
//!   we should aim to bring this down to 16.
//!
//! - **Variable-size frame slots**: frame slots are variable-sized. Each
//!   micro-op makes the width explicit — either baked into the opcode name
//!   (e.g. `Move8` = 8 bytes) or as an explicit `size` field.
//!
//! - **Inline (flat) structs**: most structs are stored inline in the frame,
//!   not heap-allocated. The existing data movement ops (`Move`, `Move8`)
//!   already handle inline structs. The `Heap*` ops are only for structs
//!   that must live on the heap (e.g. too large to inline).
//!
//! - **Calling convention**: the VM uses a single flat linear buffer as its
//!   call stack. Each frame contains locals/args followed by a 24-byte
//!   metadata section `(saved_pc, saved_fp, func_id)`.
//!
//!   ```text
//!                 caller frame                           callee frame
//!     ┌──────────────────────────────────┐   ┌──────────────────────────────┐
//!     │                        │ saved  ││   │                              │
//!     │  caller locals         │  pc    ││   │  args  │  callee locals      │
//!     │                        │  fp    ││   │                              │
//!     │                        │func_id ││   │                              │
//!     └──────────────────────────────────┘   └──────────────────────────────┘
//!                              ▲             ▲
//!                         metadata (24B)     fp
//!   ```
//!
//!   **Call**: caller writes metadata at end of its frame, places args at
//!   the start of the callee frame, then sets `fp` to the callee frame.
//!   **Return**: callee writes return values at the start of its own frame
//!   (potentially overwriting args/locals), then restores `pc`/`fp` from
//!   the metadata at `fp - 24`.
//!
//! ## Design decisions to revisit
//!
//! - **Addressing modes**: most operands are fp-relative offsets today.
//!   Secondary modes: immediate (`*Imm`), pointer + static-offset
//!   (`HeapMoveFrom`), pointer + dynamic-offset (`VecLoadElem`).
//!     - Right now, offsets are `u32` wrapped in [`FrameOffset`]. Do we want
//!       something more packed?
//!     - Do we need other addressing modes?
//!     - Which instructions should support which addressing modes?
//!
//! - **Size specialization**: 8-byte variants (`Move8`, `HeapMoveFrom8`,
//!   `HeapMoveTo8`) fast-path the common primitive/pointer size. May want
//!   `Move16` (fat pointers) and others.
//!
//! - **Branch design**: fused compare-and-branch (current) vs separate
//!   `Cmp` + `BranchTrue`/`BranchFalse`. Fused is standard for bytecode VMs.
//!
//! - **Immediate representation**: instructions like `StoreImm8` carry a
//!   `u64`, but the same instruction is used for other 8-byte types (addresses,
//!   bools, pointers). Should immediates be `u64` or `[u8; 8]`? `u64` is
//!   convenient but conflates the bit pattern with an integer type.
//!
//! - **Endianness**: the VM currently assumes native byte order. If
//!   instructions or memory layouts need to be portable across architectures,
//!   we'll be in trouble.
//!
//! - **Encoding**: Rust enum is convenient for prototyping but may not be
//!   optimal. Revisit for production.
//!
//! ## Naming conventions
//!
//! Micro-op names follow the pattern `{Op}{Modifier}{Type}{Size}`:
//!
//! - **Op**: the operation (`Store`, `Move`, `Add`, `HeapMoveFrom`, `Vec`, …)
//! - **Modifier**: addressing or source mode (`Imm` = immediate)
//! - **Type**: data type when relevant (`U64`, …)
//! - **Size**: byte width when specialized (`8` = 8 bytes)
//!
//! Examples: `StoreImm8`, `AddU64Imm`, `HeapMoveFrom8`, `VecLoadElem`.
//!
//! Operand ordering: destination (`dst`) or branch target (`target`) comes
//! first, followed by sources and immediates.
//!
//! ## Object descriptor table
//!
//! Every heap object has a header `[descriptor_id: u32 | size_in_bytes: u32]`.
//! `descriptor_id` indexes into a table of [`ObjectDescriptor`] entries that
//! tell the GC how to trace internal pointers. Three variants:
//!
//! - **Trivial** — no internal heap pointers; GC just copies the blob.
//! - **Struct** `{ size, ptr_offsets }` — fixed-size payload; `ptr_offsets`
//!   lists byte offsets within the payload that hold heap pointers.
//! - **Enum** `{ size, variants }` — like Struct but with per-variant
//!   ptr_offsets, keyed by the tag value.
//! - **Vector** `{ elem_size, elem_ptr_offsets }` — variable-length array;
//!   `elem_ptr_offsets` lists byte offsets *within each element* that hold
//!   heap pointers.

/// A typed wrapper around a `u32` frame-pointer-relative byte offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameOffset(pub u32);

/// A typed wrapper around a `u32` program-counter offset (instruction index).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodeOffset(pub u32);

#[derive(Debug)]
pub enum MicroOp {
    //======================================================================
    // Data movement
    //======================================================================
    // Move data between frame slots or store constants.
    // `Move8` fast-paths 8-byte values; `Move` handles arbitrary sizes.
    //
    // May want:
    // - `Move16` (fat pointers) + other sizes,
    // - `StoreImm` to handle arbitrary sizes,
    // - bulk data movement.
    //======================================================================
    /// Store an immediate u64 (8 bytes) at `dst` in the current frame.
    StoreImm8 { dst: FrameOffset, imm: u64 },

    /// Copy 8 bytes from `src` to `dst`.
    Move8 { dst: FrameOffset, src: FrameOffset },

    /// Copy `size` bytes from `src` to `dst`.
    Move {
        dst: FrameOffset,
        src: FrameOffset,
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
    /// `dst = lhs + rhs` (u64, checked).
    AddU64 {
        dst: FrameOffset,
        lhs: FrameOffset,
        rhs: FrameOffset,
    },

    /// `dst = src + imm` (u64, checked).
    AddU64Imm {
        dst: FrameOffset,
        src: FrameOffset,
        imm: u64,
    },

    /// `dst = src - imm` (u64, checked).
    SubU64Imm {
        dst: FrameOffset,
        src: FrameOffset,
        imm: u64,
    },

    /// `dst = imm - src` (u64, checked). Reverse immediate subtract.
    RSubU64Imm {
        dst: FrameOffset,
        src: FrameOffset,
        imm: u64,
    },

    /// `dst = src >> imm` (u64, logical right shift).
    ShrU64Imm {
        dst: FrameOffset,
        src: FrameOffset,
        imm: u64,
    },

    /// `dst = lhs % rhs` (u64 modulo). Panics on division by zero.
    ModU64 {
        dst: FrameOffset,
        lhs: FrameOffset,
        rhs: FrameOffset,
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
    /// Call function `func_id`. The caller has already placed arguments
    /// at the start of the callee's frame and written the 24-byte metadata
    /// `(pc, fp, func_id)` at `current_fp + data_size`. Sets `fp` to
    /// `current_fp + data_size + 24`.
    CallFunc { func_id: u32 },

    /// Return from the current function call. The callee has written
    /// return values at the start of its frame. Restores `pc` and `fp`
    /// from the metadata at `fp - 24`.
    Return,

    /// Unconditional jump.
    Jump { target: CodeOffset },

    /// Jump to `target` if the u64 at `src` is **not** zero.
    JumpIfNotZero {
        target: CodeOffset,
        src: FrameOffset,
    },

    /// Jump to `target` if the u64 at `src` is **>=** `imm`.
    JumpIfGreaterEqualU64Imm {
        target: CodeOffset,
        src: FrameOffset,
        imm: u64,
    },

    /// Jump to `target` if u64 at `lhs` < u64 at `rhs`.
    JumpIfLessU64 {
        target: CodeOffset,
        lhs: FrameOffset,
        rhs: FrameOffset,
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
    //   currently have no vector-to-vector addressing mode,
    // - specializations for common element sizes (e.g. 1-byte for
    //   byte strings, 8-byte for primitives).
    //======================================================================
    /// Allocate a new empty vector with the given initial capacity.
    /// `descriptor_id = 0` means trivial elements (no refs); >= 1 indexes
    /// the object descriptor table. Writes heap pointer to `dst`.
    /// MAY TRIGGER GC.
    VecNew {
        dst: FrameOffset,
        descriptor_id: u16,
        elem_size: u32,
        initial_capacity: u64,
    },

    /// Write the length (u64) of the vector to `dst`.
    VecLen {
        dst: FrameOffset,
        heap_ptr: FrameOffset,
    },

    /// Append an element. Copies `elem_size` bytes from `elem`
    /// into the vector. If capacity is exceeded, reallocates (bump) and
    /// updates `heap_ptr` in place. MAY TRIGGER GC.
    VecPushBack {
        heap_ptr: FrameOffset,
        elem: FrameOffset,
        elem_size: u32,
    },

    /// Pop last element. Copies `elem_size` bytes to `dst`.
    /// Aborts if empty.
    VecPopBack {
        dst: FrameOffset,
        heap_ptr: FrameOffset,
        elem_size: u32,
    },

    /// Read vector[idx]. Copies `elem_size` bytes to `dst`.
    /// Aborts if out of bounds.
    VecLoadElem {
        dst: FrameOffset,
        heap_ptr: FrameOffset,
        idx: FrameOffset,
        elem_size: u32,
    },

    /// Write vector[idx]. Copies `elem_size` bytes from `src`.
    /// Aborts if out of bounds.
    VecStoreElem {
        heap_ptr: FrameOffset,
        idx: FrameOffset,
        src: FrameOffset,
        elem_size: u32,
    },

    //======================================================================
    // Reference (fat pointer) operations
    //======================================================================
    // References are fat pointers: (base: *mut u8, offset: u64).
    // Fat pointers keep the base visible to GC; thin pointers would be
    // cheaper but GC couldn't identify the owning object.
    //
    // Even local references use fat pointers (with offset = 0), because
    // the same frame slot may hold either a local reference or a
    // reference into a heap object (e.g. a vector element), so we need a
    // uniform representation.
    //
    // May want:
    // - Specializations (e.g. ReadRef8/WriteRef8)
    //======================================================================
    /// Borrow a stack-local slot, producing a fat pointer `(base, offset)`.
    /// Writes 16 bytes at `[dst, dst+16)`:
    ///   - base   = fp + local (a stack address)
    ///   - offset = 0
    ///
    /// The resulting pointer slot is marked as containing a pointer. During
    /// GC, the collector checks whether a pointer falls within the heap
    /// address range — stack-local references like this one are ignored.
    StackBorrow {
        dst: FrameOffset,
        local: FrameOffset,
    },

    /// Borrow a vector element, producing a fat pointer `(base, offset)`.
    /// Writes 16 bytes at `[dst, dst+16)`:
    ///   - base   = the vector's heap pointer
    ///   - offset = VEC_DATA_OFFSET + idx * elem_size
    ///
    /// Aborts if index is out of bounds.
    VecBorrow {
        dst: FrameOffset,
        heap_ptr: FrameOffset,
        idx: FrameOffset,
        elem_size: u32,
    },

    /// Borrow a location within a heap object, producing a fat pointer
    /// `(heap_ptr, offset)`. Writes 16 bytes at `[dst, dst+16)`:
    ///   - base   = the object's heap pointer
    ///   - offset = offset from the object's start
    ///
    /// Move semantics guarantee the offset is within bounds.
    HeapBorrow {
        dst: FrameOffset,
        heap_ptr: FrameOffset,
        offset: u32,
    },

    /// Read through a fat pointer. Copies `size` bytes from the
    /// referenced location `(base + offset)` to `dst`.
    ReadRef {
        dst: FrameOffset,
        ref_ptr: FrameOffset,
        size: u32,
    },

    /// Write through a fat pointer. Copies `size` bytes from
    /// `src` to the referenced location `(base + offset)`.
    WriteRef {
        ref_ptr: FrameOffset,
        src: FrameOffset,
        size: u32,
    },

    //======================================================================
    // Heap object operations (structs and enums)
    //======================================================================
    // These ops are for structs/enums that live on the heap. Most
    // structs are inline in the frame and use the data movement ops
    // instead; enums are always heap-allocated for now. The interpreter
    // treats heap structs and enums uniformly as ptr+offset load/store —
    // the compiler helpers (below) bake in the right offsets for each.
    //
    // `HeapMoveFrom8`/`HeapMoveTo8` specialize for 8-byte fields.
    //
    // May want:
    // - fused Pack/Unpack (allocate + initialize or destructure in one
    //   step; also addresses the bulk-move problem for call setup),
    // - More specializations for common sizes (HeapMoveFrom/To).
    //======================================================================
    /// Allocate a new heap object. Size is determined by the `Struct` or
    /// `Enum` descriptor at `descriptor_id`. Writes the heap pointer to
    /// `dst`. **MAY TRIGGER GC.**
    ///
    /// The allocated memory is zero-initialized. Revisit whether a fused
    /// Pack instruction (allocate + initialize in one step) would be
    /// preferable.
    HeapNew {
        dst: FrameOffset,
        descriptor_id: u16,
    },

    /// Copy 8 bytes from a heap object at `heap_ptr + offset` into `dst`.
    HeapMoveFrom8 {
        dst: FrameOffset,
        heap_ptr: FrameOffset,
        offset: u32,
    },

    /// Copy `size` bytes from a heap object at `heap_ptr + offset` into `dst`.
    HeapMoveFrom {
        dst: FrameOffset,
        heap_ptr: FrameOffset,
        offset: u32,
        size: u32,
    },

    /// Copy 8 bytes from `src` into a heap object at `heap_ptr + offset`.
    HeapMoveTo8 {
        heap_ptr: FrameOffset,
        offset: u32,
        src: FrameOffset,
    },

    /// Write an immediate u64 into a heap object at `heap_ptr + offset`.
    HeapMoveToImm8 {
        heap_ptr: FrameOffset,
        offset: u32,
        imm: u64,
    },

    /// Copy `size` bytes from `src` into a heap object at `heap_ptr + offset`.
    HeapMoveTo {
        heap_ptr: FrameOffset,
        offset: u32,
        src: FrameOffset,
        size: u32,
    },

    //======================================================================
    // Debugging
    //======================================================================
    /// Advance the interpreter's RNG and write a random u64 to `dst`.
    /// Provides easy access to randomness during execution, enabling
    /// stress tests based on random sequences of operations (e.g. GC
    /// correctness, heap layout robustness).
    StoreRandomU64 { dst: FrameOffset },

    /// Unconditionally trigger a garbage collection cycle.
    /// Useful for testing GC correctness.
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

impl MicroOp {
    // ----- Struct helpers (offsets relative to STRUCT_DATA_OFFSET) -----

    pub fn struct_load8(heap_ptr: FrameOffset, field_offset: u32, dst: FrameOffset) -> Self {
        MicroOp::HeapMoveFrom8 {
            dst,
            heap_ptr,
            offset: STRUCT_DATA_OFFSET as u32 + field_offset,
        }
    }

    pub fn struct_load(
        heap_ptr: FrameOffset,
        field_offset: u32,
        dst: FrameOffset,
        size: u32,
    ) -> Self {
        MicroOp::HeapMoveFrom {
            dst,
            heap_ptr,
            offset: STRUCT_DATA_OFFSET as u32 + field_offset,
            size,
        }
    }

    pub fn struct_store8(heap_ptr: FrameOffset, field_offset: u32, src: FrameOffset) -> Self {
        MicroOp::HeapMoveTo8 {
            heap_ptr,
            offset: STRUCT_DATA_OFFSET as u32 + field_offset,
            src,
        }
    }

    pub fn struct_store(
        heap_ptr: FrameOffset,
        field_offset: u32,
        src: FrameOffset,
        size: u32,
    ) -> Self {
        MicroOp::HeapMoveTo {
            heap_ptr,
            offset: STRUCT_DATA_OFFSET as u32 + field_offset,
            src,
            size,
        }
    }

    pub fn struct_borrow(heap_ptr: FrameOffset, field_offset: u32, dst: FrameOffset) -> Self {
        MicroOp::HeapBorrow {
            dst,
            heap_ptr,
            offset: STRUCT_DATA_OFFSET as u32 + field_offset,
        }
    }

    // ----- Enum helpers (offsets relative to ENUM_DATA_OFFSET) -----

    pub fn enum_get_tag(heap_ptr: FrameOffset, dst: FrameOffset) -> Self {
        MicroOp::HeapMoveFrom8 {
            dst,
            heap_ptr,
            offset: ENUM_TAG_OFFSET as u32,
        }
    }

    pub fn enum_set_tag(heap_ptr: FrameOffset, variant: u16) -> Self {
        MicroOp::HeapMoveToImm8 {
            heap_ptr,
            offset: ENUM_TAG_OFFSET as u32,
            imm: variant as u64,
        }
    }

    pub fn enum_load8(heap_ptr: FrameOffset, field_offset: u32, dst: FrameOffset) -> Self {
        MicroOp::HeapMoveFrom8 {
            dst,
            heap_ptr,
            offset: ENUM_DATA_OFFSET as u32 + field_offset,
        }
    }

    pub fn enum_load(
        heap_ptr: FrameOffset,
        field_offset: u32,
        dst: FrameOffset,
        size: u32,
    ) -> Self {
        MicroOp::HeapMoveFrom {
            dst,
            heap_ptr,
            offset: ENUM_DATA_OFFSET as u32 + field_offset,
            size,
        }
    }

    pub fn enum_store8(heap_ptr: FrameOffset, field_offset: u32, src: FrameOffset) -> Self {
        MicroOp::HeapMoveTo8 {
            heap_ptr,
            offset: ENUM_DATA_OFFSET as u32 + field_offset,
            src,
        }
    }

    pub fn enum_store(
        heap_ptr: FrameOffset,
        field_offset: u32,
        src: FrameOffset,
        size: u32,
    ) -> Self {
        MicroOp::HeapMoveTo {
            heap_ptr,
            offset: ENUM_DATA_OFFSET as u32 + field_offset,
            src,
            size,
        }
    }

    pub fn enum_borrow(heap_ptr: FrameOffset, field_offset: u32, dst: FrameOffset) -> Self {
        MicroOp::HeapBorrow {
            dst,
            heap_ptr,
            offset: ENUM_DATA_OFFSET as u32 + field_offset,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn micro_op_size() {
        // Current size is 24 bytes due to large variants (e.g. VecNew,
        // JumpIfGreaterEqualU64Imm). We should aim to bring this down to 16.
        assert_eq!(std::mem::size_of::<MicroOp>(), 24);
    }
}
