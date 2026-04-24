// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! # Runtime Instruction Set (Micro-ops)
//!
//! Defines the low-level instruction set that the MonoMove interpreter
//! executes. These micro-ops are emitted by the runtime compiler
//! (monomorphizer). They operate on a flat frame-pointer-
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
//!   that must live on the heap (e.g. too large to inline or enums).
//!
//! - **Calling convention**: the VM uses a single flat linear buffer as its
//!   call stack. Each frame contains slots followed by a
//!   [`FRAME_METADATA_SIZE`]-byte metadata section `(saved_pc, saved_fp,
//!   saved_func_ptr)`.
//!
//!   ```text
//!                 caller frame                           callee frame
//!     ┌──────────────────────────────────┐   ┌──────────────────────────────┐
//!     │                        │ saved  ││   │                              │
//!     │  caller slots          │  pc    ││   │  params     │  other slots   │
//!     │                        │  fp    ││   │                              │
//!     │                        │func_ptr││   │                              │
//!     └──────────────────────────────────┘   └──────────────────────────────┘
//!                              ▲             ▲
//!                         metadata (24B)     fp
//!   ```
//!
//!   **Call**: the compiler emits explicit micro-ops to place arguments
//!   into the callee's parameter region. The `CallFunc`/`CallIndirect`/`CallDirect`
//!   instruction itself implicitly writes the metadata `(pc, fp,
//!   func_ptr)` at the end of the caller frame and sets `fp` to the
//!   callee frame.
//!   **Return**: the compiler emits explicit micro-ops to write return
//!   values at the start of the callee's frame (potentially overwriting
//!   parameter slots). The `Return` instruction itself implicitly restores
//!   `pc`/`fp` from the metadata at `fp - FRAME_METADATA_SIZE`.
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
//! ## Heap object descriptor table
//!
//! Frame slots that refer to heap objects store a raw pointer to the heap
//! allocation. Every heap object has a header
//! `[descriptor_id: u32 | size_in_bytes: u32]`.
//! `descriptor_id` indexes into a table of `ObjectDescriptor` entries
//! that tell the GC how to trace internal pointers. Three variants:
//!
//! - **Trivial** — no internal heap pointers; GC just copies the blob.
//! - **Struct** `{ size, ptr_offsets }` — fixed-size payload; `ptr_offsets`
//!   lists byte offsets within the payload that hold heap pointers.
//! - **Enum** `{ size, variants }` — like Struct but with per-variant
//!   ptr_offsets, keyed by the tag value.
//! - **Vector** `{ elem_size, elem_ptr_offsets }` — variable-length array;
//!   `elem_ptr_offsets` lists byte offsets *within each element* that hold
//!   heap pointers.

use crate::{ExecutableId, Function};
use mono_move_alloc::{ExecutableArenaPtr, GlobalArenaPtr};
use std::fmt;

// Submodules for instruction.
mod gas;
pub use gas::MicroOpGasSchedule;

/// A typed wrapper around a `u32` frame-pointer-relative byte offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameOffset(pub u32);

impl From<FrameOffset> for usize {
    #[inline(always)]
    fn from(o: FrameOffset) -> usize {
        o.0 as usize
    }
}

impl std::ops::Add<u32> for FrameOffset {
    type Output = usize;

    #[inline(always)]
    fn add(self, rhs: u32) -> usize {
        self.0 as usize + rhs as usize
    }
}

/// A typed wrapper around a `u32` program-counter offset (instruction index).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CodeOffset(pub u32);

impl From<CodeOffset> for usize {
    #[inline(always)]
    fn from(o: CodeOffset) -> usize {
        o.0 as usize
    }
}

/// Typed index into the program's object descriptor table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DescriptorId(pub u32);

impl DescriptorId {
    #[inline(always)]
    pub fn as_usize(self) -> usize {
        self.0 as usize
    }

    #[inline(always)]
    pub fn as_u32(self) -> u32 {
        self.0
    }
}

impl std::fmt::Display for DescriptorId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A sized view into a frame slot: its byte offset and width.
//
// TODO: reconcile with `SlotInfo` (same shape, different provenance).
// Also: add alignment information once the layout admits non-8-byte
// fields, otherwise callers that pack `SizedSlot`s back-to-back (e.g.
// captured data) will produce mis-aligned fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SizedSlot {
    pub offset: FrameOffset,
    pub size: u32,
}

/// Reference to the target function of a closure.
///
/// Conceptually an enum. The concrete in-memory representation of this enum
/// (tag size, padding, payload layout) inside closure heap objects is given
/// by [`CLOSURE_FUNC_REF_SIZE`] and associated constants.
#[derive(Clone, Debug)]
pub enum ClosureFuncRef {
    /// Local function, already monomorphized and materialized.
    Resolved(ExecutableArenaPtr<Function>),
    // `Unresolved { ... }` — cross-module form, TBD. Resolved lazily at call
    // time. Its payload will mirror whatever representation cross-module
    // calls end up using.
}

/// Operand data for [`MicroOp::PackClosure`].
///
/// Boxed so the micro-op enum stays small despite the variable-length
/// `captured` list.
#[derive(Clone, Debug)]
pub struct PackClosureOp {
    /// Frame slot that receives the heap pointer to the closure object.
    pub dst: FrameOffset,
    /// Target function.
    pub func_ref: ClosureFuncRef,
    /// Bitmask of which function parameters are captured vs provided.
    pub mask: u64,
    /// Descriptor for the allocated closure object.
    pub closure_descriptor_id: DescriptorId,
    /// Descriptor for the allocated `ClosureCapturedData` (Materialized) object.
    pub captured_data_descriptor_id: DescriptorId,
    /// Sources (in caller's frame) of the captured values, in the order that
    /// `mask.is_captured(i)` is true — i.e. ascending `i` through the
    /// function's parameter list.
    pub captured: Vec<SizedSlot>,
}

/// Operand data for [`MicroOp::CallClosure`].
#[derive(Clone, Debug)]
pub struct CallClosureOp {
    /// Frame slot holding the closure heap pointer.
    pub closure_src: FrameOffset,
    /// Sources (in caller's frame) of the provided (non-captured) arguments,
    /// in the order that `mask.is_captured(i)` is false — i.e. ascending `i`
    /// through the function's parameter list.
    pub provided_args: Vec<SizedSlot>,
}

#[derive(Clone, Debug)]
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

    /// `dst = lhs ^ rhs` (u64, bitwise XOR).
    XorU64 {
        dst: FrameOffset,
        lhs: FrameOffset,
        rhs: FrameOffset,
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
    // Boolean branches: booleans could be represented as small integers
    // (0 = false, non-zero = true) and handled with a dedicated
    // `JumpIfTrue` / `JumpIfNotZeroU8` variant. Open questions: how to
    // handle "dirty" bools (values other than 0 or 1).
    //
    // May want:
    // - Abort,
    // - more conditions: ==, !=, >, <=, and const variants,
    // - something for enum dispatch (jump table)?
    //======================================================================
    /// Call function `func_id`. The compiler has already emitted micro-ops
    /// to place arguments into the callee's parameter region. This instruction
    /// implicitly writes the metadata `(pc, fp, func_ptr)` at
    /// `current_fp + param_and_local_sizes_sum` and sets `fp` to
    /// `current_fp + param_and_local_sizes_sum + FRAME_METADATA_SIZE`.
    CallFunc { func_id: u32 },

    /// Call a function by module identity and name. Same calling convention
    /// as `CallFunc`.
    CallIndirect {
        executable_id: GlobalArenaPtr<ExecutableId>,
        func_name: GlobalArenaPtr<str>,
    },

    /// Call a function via direct pointer. Same calling convention as
    /// `CallFunc`.
    CallDirect { ptr: ExecutableArenaPtr<Function> },

    /// Return from the current function call. The compiler has already
    /// emitted micro-ops to write return values at the start of the
    /// callee's frame. This instruction implicitly restores `pc` and `fp`
    /// from the metadata at `fp - FRAME_METADATA_SIZE`.
    Return,

    /// Unconditional jump.
    Jump { target: CodeOffset },

    /// Jump to `target` if the u64 at `src` is **not** zero.
    JumpNotZeroU64 {
        target: CodeOffset,
        src: FrameOffset,
    },

    /// Jump to `target` if the u64 at `src` is **>=** `imm`.
    JumpGreaterEqualU64Imm {
        target: CodeOffset,
        src: FrameOffset,
        imm: u64,
    },

    /// Jump to `target` if the u64 at `src` is **<** `imm`.
    JumpLessU64Imm {
        target: CodeOffset,
        src: FrameOffset,
        imm: u64,
    },

    /// Jump to `target` if the u64 at `src` is **>** `imm`.
    JumpGreaterU64Imm {
        target: CodeOffset,
        src: FrameOffset,
        imm: u64,
    },

    /// Jump to `target` if the u64 at `src` is **<=** `imm`.
    JumpLessEqualU64Imm {
        target: CodeOffset,
        src: FrameOffset,
        imm: u64,
    },

    /// Jump to `target` if u64 at `lhs` < u64 at `rhs`.
    JumpLessU64 {
        target: CodeOffset,
        lhs: FrameOffset,
        rhs: FrameOffset,
    },

    /// Jump to `target` if u64 at `lhs` >= u64 at `rhs`.
    JumpGreaterEqualU64 {
        target: CodeOffset,
        lhs: FrameOffset,
        rhs: FrameOffset,
    },

    /// Jump to `target` if u64 at `lhs` != u64 at `rhs`.
    JumpNotEqualU64 {
        target: CodeOffset,
        lhs: FrameOffset,
        rhs: FrameOffset,
    },

    //======================================================================
    // Vector operations
    //======================================================================
    // Heap-allocated: [header | length | elements...].
    // Capacity is derived from the header's size field.
    // A null pointer represents an empty vector (no allocation).
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
    /// Initialize an empty vector by writing a null pointer to `dst`.
    /// No heap allocation occurs; the first `VecPushBack` allocates lazily
    /// using the `descriptor_id` and `elem_size` it carries.
    VecNew { dst: FrameOffset },

    /// Write the length (u64) of the vector to `dst`.
    /// `vec_ref` is a 16-byte fat pointer `(base, offset)` whose target
    /// holds the vector's heap pointer.
    VecLen {
        dst: FrameOffset,
        vec_ref: FrameOffset,
    },

    /// Append an element. Copies `elem_size` bytes from `elem`
    /// into the vector. If the vector is null (empty), allocates a new
    /// buffer using `descriptor_id`. If capacity is exceeded, reallocates
    /// (bump) and writes the new pointer back through `vec_ref`.
    /// `vec_ref` is a 16-byte fat pointer whose target holds the vector's
    /// heap pointer. MAY TRIGGER GC.
    VecPushBack {
        vec_ref: FrameOffset,
        elem: FrameOffset,
        elem_size: u32,
        descriptor_id: DescriptorId,
    },

    /// Pop last element. Copies `elem_size` bytes to `dst`.
    /// `vec_ref` is a 16-byte fat pointer whose target holds the vector's
    /// heap pointer. Aborts if empty.
    VecPopBack {
        dst: FrameOffset,
        vec_ref: FrameOffset,
        elem_size: u32,
    },

    /// Read vector[idx]. Copies `elem_size` bytes to `dst`.
    /// `vec_ref` is a 16-byte fat pointer whose target holds the vector's
    /// heap pointer. Aborts if out of bounds.
    VecLoadElem {
        dst: FrameOffset,
        vec_ref: FrameOffset,
        idx: FrameOffset,
        elem_size: u32,
    },

    /// Write vector[idx]. Copies `elem_size` bytes from `src`.
    /// `vec_ref` is a 16-byte fat pointer whose target holds the vector's
    /// heap pointer. Aborts if out of bounds.
    VecStoreElem {
        vec_ref: FrameOffset,
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
    /// Borrow a frame slot, producing a fat pointer `(base, offset)`.
    /// Writes 16 bytes at `[dst, dst+16)`:
    ///   - base   = fp + local (a stack address)
    ///   - offset = 0
    ///
    /// The resulting pointer slot is marked as containing a pointer. During
    /// GC, the collector checks whether a pointer falls within the heap
    /// address range — stack-local references like this one are ignored.
    SlotBorrow {
        dst: FrameOffset,
        local: FrameOffset,
    },

    /// Borrow a vector element, producing a fat pointer `(base, offset)`.
    /// `vec_ref` is a 16-byte fat pointer whose target holds the vector's
    /// heap pointer. Writes 16 bytes at `[dst, dst+16)`:
    ///   - base   = the vector's heap pointer
    ///   - offset = VEC_DATA_OFFSET + idx * elem_size
    ///
    /// Aborts if index is out of bounds.
    VecBorrow {
        dst: FrameOffset,
        vec_ref: FrameOffset,
        idx: FrameOffset,
        elem_size: u32,
    },

    /// Borrow a location within a heap object, producing a fat pointer.
    /// `obj_ref` is a 16-byte fat pointer `(base, ref_offset)` whose
    /// target holds the heap object pointer. Writes 16 bytes at
    /// `[dst, dst+16)`:
    ///   - base   = the object's heap pointer (read through `obj_ref`)
    ///   - offset = `offset` (byte offset from the object's start)
    ///
    /// Move semantics guarantee the offset is within bounds.
    HeapBorrow {
        dst: FrameOffset,
        obj_ref: FrameOffset,
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
        descriptor_id: DescriptorId,
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
    // Gas metering
    //======================================================================
    // Inserted by the instrumentation pass; never emitted directly by user code.
    //======================================================================
    /// Charge a pre-computed static gas cost for the current basic block.
    /// The interpreter must call the gas meter and abort on exhaustion.
    Charge { cost: u64 },

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
    // - **Runtime instrumentation**: tracing, profiling, coverage hooks.
    //======================================================================

    //======================================================================
    // Closures / function values
    //======================================================================
    // Two fused super-instructions. Both carry variable-length data and are
    // boxed so the base `MicroOp` enum stays small. The closure runtime
    // representation is a two-level heap object: a closure object that
    // points to a separate `ClosureCapturedData` object.
    //======================================================================
    /// Pack a closure. Allocates the closure object and a fresh
    /// `ClosureCapturedData` (Materialized) object, copies captured values
    /// into the latter, and writes the closure heap pointer to `dst`.
    /// **MAY TRIGGER GC** (two allocations).
    PackClosure(Box<PackClosureOp>),

    /// Call a closure. Reads the closure at `closure_src`, interleaves its
    /// captured values with the provided arguments into the callee's
    /// parameter region (using the mask and the callee's `param_sizes`),
    /// then performs the standard call protocol.
    ///
    /// For v0 this only supports `ClosureFuncRef::Resolved` closures whose
    /// captured data is `Materialized`. A raw-captured-data path (for
    /// closures loaded from storage) is future work.
    CallClosure(Box<CallClosureOp>),
}

impl fmt::Display for MicroOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MicroOp::StoreImm8 { dst, imm } => {
                write!(f, "StoreImm8 [{}] <- #{}", dst.0, imm)
            },
            MicroOp::Move8 { dst, src } => {
                write!(f, "Move8 [{}] <- [{}]", dst.0, src.0)
            },
            MicroOp::Move { dst, src, size } => {
                write!(f, "Move({}) [{}] <- [{}]", size, dst.0, src.0)
            },
            MicroOp::AddU64 { dst, lhs, rhs } => {
                write!(f, "AddU64 [{}] <- [{}] + [{}]", dst.0, lhs.0, rhs.0)
            },
            MicroOp::AddU64Imm { dst, src, imm } => {
                write!(f, "AddU64Imm [{}] <- [{}] + #{}", dst.0, src.0, imm)
            },
            MicroOp::SubU64Imm { dst, src, imm } => {
                write!(f, "SubU64Imm [{}] <- [{}] - #{}", dst.0, src.0, imm)
            },
            MicroOp::RSubU64Imm { dst, src, imm } => {
                write!(f, "RSubU64Imm [{}] <- #{} - [{}]", dst.0, imm, src.0)
            },
            MicroOp::ShrU64Imm { dst, src, imm } => {
                write!(f, "ShrU64Imm [{}] <- [{}] >> #{}", dst.0, src.0, imm)
            },
            MicroOp::ModU64 { dst, lhs, rhs } => {
                write!(f, "ModU64 [{}] <- [{}] % [{}]", dst.0, lhs.0, rhs.0)
            },
            MicroOp::CallFunc { func_id } => {
                write!(f, "CallFunc #{}", func_id)
            },
            MicroOp::CallIndirect { .. } => {
                write!(f, "CallIndirect")
            },
            MicroOp::CallDirect { .. } => {
                write!(f, "CallDirect")
            },
            MicroOp::Return => {
                write!(f, "Return")
            },
            MicroOp::Jump { target } => {
                write!(f, "Jump @{}", target.0)
            },
            MicroOp::JumpNotZeroU64 { target, src } => {
                write!(f, "JumpNotZeroU64 @{} [{}]", target.0, src.0)
            },
            MicroOp::JumpGreaterEqualU64Imm { target, src, imm } => {
                write!(
                    f,
                    "JumpGreaterEqualU64Imm @{} [{}] >= #{}",
                    target.0, src.0, imm
                )
            },
            MicroOp::JumpGreaterU64Imm { target, src, imm } => {
                write!(f, "JumpGreaterU64Imm @{} [{}] > #{}", target.0, src.0, imm)
            },
            MicroOp::JumpLessEqualU64Imm { target, src, imm } => {
                write!(
                    f,
                    "JumpLessEqualU64Imm @{} [{}] <= #{}",
                    target.0, src.0, imm
                )
            },
            MicroOp::JumpLessU64 { target, lhs, rhs } => {
                write!(f, "JumpLessU64 @{} [{}] < [{}]", target.0, lhs.0, rhs.0)
            },
            MicroOp::VecNew { dst } => {
                write!(f, "VecNew [{}]", dst.0)
            },
            MicroOp::VecLen { dst, vec_ref } => {
                write!(f, "VecLen [{}] <- vec_len([{}])", dst.0, vec_ref.0)
            },
            MicroOp::VecPushBack {
                vec_ref,
                elem,
                elem_size,
                descriptor_id,
            } => {
                write!(
                    f,
                    "VecPushBack [{}].push([{}], size={}, desc={})",
                    vec_ref.0, elem.0, elem_size, descriptor_id
                )
            },
            MicroOp::VecPopBack {
                dst,
                vec_ref,
                elem_size,
            } => {
                write!(
                    f,
                    "VecPopBack [{}] <- [{}].pop(size={})",
                    dst.0, vec_ref.0, elem_size
                )
            },
            MicroOp::VecLoadElem {
                dst,
                vec_ref,
                idx,
                elem_size,
            } => {
                write!(
                    f,
                    "VecLoadElem [{}] <- [{}][[{}]] (size={})",
                    dst.0, vec_ref.0, idx.0, elem_size
                )
            },
            MicroOp::VecStoreElem {
                vec_ref,
                idx,
                src,
                elem_size,
            } => {
                write!(
                    f,
                    "VecStoreElem [{}][[{}]] <- [{}] (size={})",
                    vec_ref.0, idx.0, src.0, elem_size
                )
            },
            MicroOp::SlotBorrow { dst, local } => {
                write!(f, "SlotBorrow [{}] <- &[{}]", dst.0, local.0)
            },
            MicroOp::VecBorrow {
                dst,
                vec_ref,
                idx,
                elem_size,
            } => {
                write!(
                    f,
                    "VecBorrow [{}] <- &[{}][[{}]] (elem_size={})",
                    dst.0, vec_ref.0, idx.0, elem_size
                )
            },
            MicroOp::HeapBorrow {
                dst,
                obj_ref,
                offset,
            } => {
                write!(f, "HeapBorrow [{}] <- &[{}]+{}", dst.0, obj_ref.0, offset)
            },
            MicroOp::ReadRef { dst, ref_ptr, size } => {
                write!(f, "ReadRef [{}] <- *[{}] (size={})", dst.0, ref_ptr.0, size)
            },
            MicroOp::WriteRef { ref_ptr, src, size } => {
                write!(
                    f,
                    "WriteRef *[{}] <- [{}] (size={})",
                    ref_ptr.0, src.0, size
                )
            },
            MicroOp::HeapNew { dst, descriptor_id } => {
                write!(f, "HeapNew [{}] desc={}", dst.0, descriptor_id)
            },
            MicroOp::HeapMoveFrom8 {
                dst,
                heap_ptr,
                offset,
            } => {
                write!(
                    f,
                    "HeapMoveFrom8 [{}] <- [{}]+{}",
                    dst.0, heap_ptr.0, offset
                )
            },
            MicroOp::HeapMoveFrom {
                dst,
                heap_ptr,
                offset,
                size,
            } => {
                write!(
                    f,
                    "HeapMoveFrom [{}] <- [{}]+{} (size={})",
                    dst.0, heap_ptr.0, offset, size
                )
            },
            MicroOp::HeapMoveTo8 {
                heap_ptr,
                offset,
                src,
            } => {
                write!(f, "HeapMoveTo8 [{}]+{} <- [{}]", heap_ptr.0, offset, src.0)
            },
            MicroOp::HeapMoveToImm8 {
                heap_ptr,
                offset,
                imm,
            } => {
                write!(f, "HeapMoveToImm8 [{}]+{} <- #{}", heap_ptr.0, offset, imm)
            },
            MicroOp::HeapMoveTo {
                heap_ptr,
                offset,
                src,
                size,
            } => {
                write!(
                    f,
                    "HeapMoveTo [{}]+{} <- [{}] (size={})",
                    heap_ptr.0, offset, src.0, size
                )
            },
            MicroOp::StoreRandomU64 { dst } => {
                write!(f, "StoreRandomU64 [{}]", dst.0)
            },
            MicroOp::XorU64 { dst, lhs, rhs } => {
                write!(f, "XorU64 [{}] <- [{}] ^ [{}]", dst.0, lhs.0, rhs.0)
            },
            MicroOp::JumpLessU64Imm { target, src, imm } => {
                write!(f, "JumpLessU64Imm @{} [{}] < #{}", target.0, src.0, imm)
            },
            MicroOp::JumpGreaterEqualU64 { target, lhs, rhs } => {
                write!(
                    f,
                    "JumpGreaterEqualU64 @{} [{}] >= [{}]",
                    target.0, lhs.0, rhs.0
                )
            },
            MicroOp::JumpNotEqualU64 { target, lhs, rhs } => {
                write!(
                    f,
                    "JumpNotEqualU64 @{} [{}] != [{}]",
                    target.0, lhs.0, rhs.0
                )
            },
            MicroOp::ForceGC => {
                write!(f, "ForceGC")
            },
            MicroOp::Charge { cost } => {
                write!(f, "Charge #{}", cost)
            },
            MicroOp::PackClosure(op) => {
                write!(
                    f,
                    "PackClosure [{}] <- func_ref={:?}, mask={:b}, closure_desc={}, captured_desc={}, captured=[",
                    op.dst.0, op.func_ref, op.mask, op.closure_descriptor_id, op.captured_data_descriptor_id
                )?;
                for (i, slot) in op.captured.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "[{}]({})", slot.offset.0, slot.size)?;
                }
                write!(f, "]")
            },
            MicroOp::CallClosure(op) => {
                write!(
                    f,
                    "CallClosure closure=[{}], provided_args=[",
                    op.closure_src.0
                )?;
                for (i, slot) in op.provided_args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "[{}]({})", slot.offset.0, slot.size)?;
                }
                write!(f, "]")
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Constructor helpers for the re-compiler
// ---------------------------------------------------------------------------

/// Size of the per-frame metadata section: `(saved_pc, saved_fp, func_id)`.
pub const FRAME_METADATA_SIZE: usize = 24;

/// Size of the object header: [descriptor_id: u32 | size_in_bytes: u32].
pub const OBJECT_HEADER_SIZE: usize = 8;

/// Offset where struct field data begins (same as OBJECT_HEADER_SIZE).
pub const STRUCT_DATA_OFFSET: usize = OBJECT_HEADER_SIZE; // 8

/// Offset of the variant tag (u64) within an enum object.
pub const ENUM_TAG_OFFSET: usize = OBJECT_HEADER_SIZE; // 8

/// Offset where enum variant field data begins (after header + tag).
pub const ENUM_DATA_OFFSET: usize = OBJECT_HEADER_SIZE + 8; // 16

// ---------------------------------------------------------------------------
// Closure object layout
// ---------------------------------------------------------------------------
//
// Closure object (heap-allocated, fixed size):
//
//   [header(8)] [func_ref(16)] [mask(8)] [captured_data_ptr(8)]  = 40 bytes
//
// `func_ref` is an inline `ClosureFuncRef` enum. Reserved as 16 bytes to
// leave room for a future `Unresolved` variant; v0 uses only `Resolved`:
//
//   within func_ref:
//     offset 0:  tag (u8)   — FUNC_REF_TAG_RESOLVED = 0
//     offset 1:  padding (7 bytes)
//     offset 8:  payload (8-byte pointer)

/// Reserved size for an inline `ClosureFuncRef` inside a closure heap object.
/// Sized to accommodate a future `Unresolved` variant without on-disk layout
/// changes.
pub const CLOSURE_FUNC_REF_SIZE: usize = 16;

/// Offset of the `func_ref` field within a closure heap object (after header).
pub const CLOSURE_FUNC_REF_OFFSET: usize = OBJECT_HEADER_SIZE; // 8

/// Offset of the `mask` field within a closure heap object.
pub const CLOSURE_MASK_OFFSET: usize = CLOSURE_FUNC_REF_OFFSET + CLOSURE_FUNC_REF_SIZE; // 24

/// Offset of the `captured_data_ptr` field within a closure heap object.
/// The GC traces this slot (heap pointer to the `ClosureCapturedData` object).
pub const CLOSURE_CAPTURED_DATA_PTR_OFFSET: usize = CLOSURE_MASK_OFFSET + 8; // 32

/// Total size of a closure heap object (header + payload).
pub const CLOSURE_OBJECT_SIZE: usize = CLOSURE_CAPTURED_DATA_PTR_OFFSET + 8; // 40

// Offsets within the `func_ref` region.
/// Byte offset of the tag within `func_ref`.
pub const FUNC_REF_TAG_OFFSET: usize = 0;
/// Byte offset of the payload within `func_ref`.
pub const FUNC_REF_PAYLOAD_OFFSET: usize = 8;

/// `ClosureFuncRef::Resolved` tag value.
pub const FUNC_REF_TAG_RESOLVED: u8 = 0;
// Future: `FUNC_REF_TAG_UNRESOLVED: u8 = 1`

// ---------------------------------------------------------------------------
// ClosureCapturedData object layout (Materialized)
// ---------------------------------------------------------------------------
//
//   [header(8)] [tag(1)] [padding(7)] [captured values packed in param order]
//
// Captured values are packed tightly, in the order of their parameter
// positions (i.e. ascending `i` where `mask.is_captured(i)` is true).
// Total count is implied by `mask.captured_count()`. Individual sizes are
// read from the target function's `param_sizes` at call time.

/// Byte offset of the tag (u8) within a `ClosureCapturedData` heap object.
pub const CAPTURED_DATA_TAG_OFFSET: usize = OBJECT_HEADER_SIZE; // 8

/// Byte offset where captured values begin (after header + tag + padding).
pub const CAPTURED_DATA_VALUES_OFFSET: usize = OBJECT_HEADER_SIZE + 8; // 16

/// `ClosureCapturedData::Materialized` tag value.
pub const CAPTURED_DATA_TAG_MATERIALIZED: u8 = 0;
// Future: `CAPTURED_DATA_TAG_RAW: u8 = 1`

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

    pub fn struct_borrow(obj_ref: FrameOffset, field_offset: u32, dst: FrameOffset) -> Self {
        MicroOp::HeapBorrow {
            dst,
            obj_ref,
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

    pub fn enum_borrow(obj_ref: FrameOffset, field_offset: u32, dst: FrameOffset) -> Self {
        MicroOp::HeapBorrow {
            dst,
            obj_ref,
            offset: ENUM_DATA_OFFSET as u32 + field_offset,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn micro_op_size() {
        // Size is 32 bytes due to CallIndirect which carries two
        // GlobalArenaPtr fields (8 + 16 bytes). TODO: bring this down.
        assert_eq!(std::mem::size_of::<MicroOp>(), 32);
    }
}
