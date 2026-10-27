// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Static well-formedness checker for lowered [`Function`] bodies.
//!
//! The loader runs [`check_well_formedness`] on every lowered function before it
//! is cached, so a rejected lowering is never executed. Any error it reports is
//! an invariant violation: the bytecode already passed the Move bytecode
//! verifier, so a lowering that fails here is a bug in the specializer
//! pipeline. Test harnesses that build `Function`s by hand call
//! [`assert_well_formed`].
//!
//! Each check below has an identifier (`F1`, `P3`, `O1`, ...) that the
//! implementation cites where the check is evaluated, as `[F1]` or
//! `[P1-P4, R1-R4]`; a test checks that every check is cited and every citation
//! is specified. All checks run and every violation is reported; none depends
//! on another having passed.
//!
//! The checker is unmetered, so its cost must stay linear, or at worst
//! `O(n * log(n))`. Lookups must be implemented efficiently, e.g. with binary
//! search over sorted lists or with map data structures.
//!
//! # Notation
//!
//! For a function `F`:
//!
//! | Symbol                | Meaning                                                                                |
//! |-----------------------|----------------------------------------------------------------------------------------|
//! | `code`                | `F.code.ops()`; `N = len(code)`; `code[pc]` is the op at `pc`                          |
//! | `origins`             | `F.code.origins()`, one bytecode offset per op, or empty                               |
//! | `S`                   | `F.param_and_local_sizes_sum`                                                          |
//! | `M`                   | `FRAME_METADATA_SIZE` (24)                                                             |
//! | `E`                   | `F.extended_frame_size`                                                                |
//! | `P`                   | `F.param_region_size`                                                                  |
//! | `A`                   | `MAX_ALIGN` (8)                                                                        |
//! | `ptr`                 | `PTR_SLOT`: width and alignment `(w, a)` of a heap-pointer slot, currently `(8, 8)`    |
//! | `Data`                | `[0, S)`: parameters and locals                                                        |
//! | `Meta`                | `[S, S + M)`: saved pc, fp, and function pointer, written only by call and return      |
//! | `Callee`              | `[S + M, E)`: the callee's argument and return region; the callee's fp is `fp + S + M` |
//! | `params`, `param_tys` | `F.param_slots` and the matching types                                                 |
//! | `rets`, `ret_tys`     | `F.return_slots` and the matching types                                                |
//! | `base`                | `F.frame_layout.heap_ptr_offsets`: pointer slots the GC scans at every pc              |
//! | `sps`                 | `F.safe_point_layouts.entries()`: `(code_offset, heap_ptr_offsets)` pairs              |
//! | `desc(id)`            | the descriptor for `id`, or none                                                       |
//! | `shape(ty)`           | `(size, align)` of `ty`, intrinsic or from a published layout, or none                 |
//! | `layout(ty)`          | the published `ValueLayout` for `ty`, or none                                          |
//! | `const_ty(idx)`       | the type of constant `idx` in `F`'s module, or none                                    |
//!
//! A **slot** is `(o, w, a)`: offset `o` from the frame pointer, width `w`,
//! alignment `a`; `end = o + w`. A slot is **well-formed** iff `w > 0`, `a` is a
//! power of two with `a <= A`, and `o mod a = 0`.
//!
//! An **access** `(o, w, a)` is **valid** iff `o + w <= E`, `[o, o + w)` does not
//! intersect `Meta`, and `o mod a = 0`. Accesses into `Callee` are valid: that is
//! how arguments and return values are passed.
//!
//! A slot list is **ascending and disjoint** iff `end(s_i) <= o_{i+1}` for every
//! consecutive pair.
//!
//! The **canonical signature layout** starts at cursor zero. For each type,
//! resolve `shape(ty)`, align the cursor to that alignment, place a slot with
//! that offset/size/alignment, and advance by its size. Sizes must be positive,
//! alignments valid, and offsets and ends must fit in `u32`. There is no final
//! padding. The same checked calculation packs capture sizes/alignments.
//!
//! # Checks
//!
//! ## Function shape
//!
//! | Id | Property                       | Condition                                                               | Rationale                                                                 |
//! |----|--------------------------------|-------------------------------------------------------------------------|---------------------------------------------------------------------------|
//! | F1 | has code                       | `N >= 1`                                                                |                                                                           |
//! | F2 | cannot run off the end         | `code[N - 1].is_terminator()`: `Return`, `Abort`, `AbortMsg`, or `Jump` | every other op falls through to `pc + 1`; a call returns to `call_pc + 1` |
//! | F3 | frame holds its metadata       | `S + M <= E`                                                            |                                                                           |
//! | F4 | parameters fit the data region | `P <= S`                                                                |                                                                           |
//! | F5 | metadata is aligned            | `S mod A = 0`                                                           | `Meta` is written with aligned `u64` stores                               |
//! | F6 | callee fp is aligned           | `(S + M) mod A = 0`                                                     | a callee's frame pointer is `fp + S + M`                                  |
//! | F7 | origins match code             | `len(origins) = 0` or `len(origins) = N`                                | a partial table would attribute errors to the wrong bytecode offsets      |
//!
//! ## Parameter and return slots
//!
//! | Id | Property                                | Condition                                                                        | Rationale                                                                      |
//! |----|-----------------------------------------|----------------------------------------------------------------------------------|--------------------------------------------------------------------------------|
//! | P1 | one slot per parameter                  | `len(params) = len(param_tys)`                                                   |                                                                                |
//! | P2 | parameter slots are well-formed         | every slot in `params` is well-formed                                            |                                                                                |
//! | P3 | parameters lie in the parameter region  | every slot in `params` has `end <= P`                                            | callers write each parameter at its slot, then the callee zeroes `[P, E)`      |
//! | P4 | parameter slots match the signature ABI | every slot has the offset, size, and alignment of the canonical signature layout | implies ascending, disjoint slots; callers calculate this layout independently |
//! | P5 | parameter region has exact extent       | `P` equals the canonical parameter layout size, or zero for no parameters        | no final alignment rounding                                                    |
//! | R1 | one slot per return value               | `len(rets) = len(ret_tys)`                                                       |                                                                                |
//! | R2 | return slots are well-formed            | every slot in `rets` is well-formed                                              |                                                                                |
//! | R3 | return values lie in the data region    | every slot in `rets` has `end <= S`                                              |                                                                                |
//! | R4 | return slots match the signature ABI    | every slot has the offset, size, and alignment of the canonical signature layout | parameters and returns reuse storage and are checked separately                |
//!
//! ## GC layouts
//!
//! | Id | Property                                      | Condition                                                                                                     | Rationale                                                         |
//! |----|-----------------------------------------------|---------------------------------------------------------------------------------------------------------------|-------------------------------------------------------------------|
//! | G1 | base pointer slots are real slots             | every `o` in `base` is a valid access `(o, ptr.w, ptr.a)`                                                     |                                                                   |
//! | G2 | base layout is sorted                         | `base` is strictly increasing                                                                                 |                                                                   |
//! | G3 | base roots have entry initialization coverage | for each `b` in `base`: if `b < P`, `[b, b + ptr.w)` fits within one parameter slot; otherwise `F.zero_frame` | argument copies do not initialize padding between parameter slots |
//! | G4 | safe points are sorted                        | the `code_offset`s of `sps` are strictly increasing                                                           |                                                                   |
//! | G5 | safe points sit at allocating ops             | every `code_offset < N` and `code[code_offset].is_allocating()`                                               |                                                                   |
//! | G6 | safe-point pointer slots are real slots       | every entry's `heap_ptr_offsets` satisfies G1 and G2                                                          |                                                                   |
//! | G7 | safe points do not repeat the base layout     | no offset is in both an entry's `heap_ptr_offsets` and `base`                                                 |                                                                   |
//!
//! ## Operand accesses
//!
//! For every `pc` and every `(o, kind, access)` reported by `code[pc].for_each_frame_operand` (see `instruction::operands`):
//!
//! | Id | Property                                                        | Condition                                                                                                                           | Rationale                                                            |
//! |----|-----------------------------------------------------------------|-------------------------------------------------------------------------------------------------------------------------------------|----------------------------------------------------------------------|
//! | O1 | every operand access is in frame, off the metadata, and aligned | the access `(o, w, a)` is valid; width/alignment come from the fixed kind or the published layout for `Value(ty)` / `Constant(idx)` |                                                                      |
//! | O2 | compared values have a published layout                         | the `ty` of all four value comparison ops has `layout(ty)`                                                                          | reference comparisons also interpret their pointee using this layout |
//! | O3 | comparison is on values, not references                         | the `ty` of `ValueCmp`, `ValueRefCmp`, `JumpValueCmp`, and `JumpValueRefCmp` is not a reference type                                |                                                                      |
//! | O4 | constants exist                                                 | for `Constant(idx)`, `const_ty(idx)` exists                                                                                         |                                                                      |
//! | O5 | constants have a layout                                         | for `Constant(idx)`, `layout(const_ty(idx))` exists                                                                                 |                                                                      |
//! | O6 | known non-pointer bytes do not alias GC roots                   | every known non-pointer operand range is disjoint from every root range in `base` and the safe-point layout at `pc`                 | applies to both reads and writes; see ranges below                   |
//!
//! Known non-pointer ranges, relative to the operand start:
//!
//! - `Bool`, `Byte`, `U64`, `Int`, `Address`: the entire operand;
//! - `FatPtr`, or a typed operand with `LayoutKind::Ref`: `[8, 16)`;
//! - `Value`/`Constant` with `NO_POINTERS_NO_PADDING`: the entire operand.
//!
//! Raw `Bytes` and other aggregate layouts add no non-pointer ranges.
//!
//! ## Instruction-local invariants
//!
//! | Id | Property                                  | Condition                                                                                        | Rationale                                                                               |
//! |----|-------------------------------------------|--------------------------------------------------------------------------------------------------|-----------------------------------------------------------------------------------------|
//! | I1 | no unchecked division by zero             | `DivU64Imm`, `ModU64Imm`: `imm != 0`                                                             | lowering emits the checked op when the immediate may be zero, so this is a lowering bug |
//! | I2 | no unchecked over-shift                   | `ShlU64Imm`, `ShrU64Imm`: `imm < 64`                                                             | lowering emits the checked op when the amount may be out of range                       |
//! | I3 | bitwise ops are unsigned                  | `IntBitAnd`, `IntBitOr`, `IntBitXor`: `rhs` is unsigned                                          | the interpreter rejects it at runtime as an invariant violation                         |
//! | I4 | shifts are unsigned                       | `IntShl`, `IntShr`: `ty` is unsigned                                                             | the interpreter rejects it at runtime as an invariant violation                         |
//! | I5 | negation is signed                        | `IntNegate`: `ty` is signed                                                                      | the interpreter rejects it at runtime as an invariant violation                         |
//! | Z1 | copies move at least one byte             | `size > 0`, or `elem_size > 0`, for the ops listed below                                         |                                                                                         |
//! | Z2 | heap and reference windows do not wrap    | `offset + size` (or `offset + ptr.w`) fits in `u32` for the ops listed below                     | any variant tag may be selected at runtime                                              |
//! | Z3 | deep-copy slot offsets do not wrap        | `DeepCopyHeapPtrs`: `base + off` fits in `u32` for every `off`                                   |                                                                                         |
//! | Z4 | enum fields skip the tag                  | every `Some(offset)` in enum field read/write/borrow tables has `offset >= ENUM_DATA_OFFSET` (8) | offsets include the tag                                                                 |
//! | J1 | jumps stay in the function                | every jump `target < N`                                                                          |                                                                                         |
//! | D1 | logical frame destinations do not overlap | all nonempty operand ranges with access `Write` or `ReadWrite` are pairwise disjoint             | a read/write operand contributes one destination; source/destination overlap is allowed |
//! | B1 | borrowed locals are in the data region    | `SlotBorrow`: `local < S`                                                                        | the op carries no width, so only the base can be checked                                |
//!
//! - Z1 `size`: `Move`, `ReadRef`, `WriteRef`, `ReadRefOffset`, `WriteRefOffset`,
//!   `HeapReadOffset`, `HeapWriteOffset`, `HeapMoveFrom`, `HeapMoveTo`,
//!   `EnumReadVariantFieldByTag`, `EnumWriteVariantFieldByTag`. Z1 `elem_size`:
//!   `VecPushBack`, `VecPopBack`, `VecLoadElem`, `VecStoreElem`, `VecSwap`,
//!   `VecBorrow`, `VecPack`, `VecUnpack`.
//! - Z2 `offset + size`: `HeapMoveFrom`, `HeapMoveTo`, `HeapReadOffset`,
//!   `HeapWriteOffset`, `ReadRefOffset`, `WriteRefOffset`, and every `Some(offset)`
//!   in the tables of `EnumReadVariantFieldByTag` and `EnumWriteVariantFieldByTag`.
//!   Z2 `offset + ptr.w`: `HeapMoveFrom8`, `HeapMoveTo8`, `HeapMoveToImm8`.
//! - D1 uses the operand schema for every op, including `VecUnpack.dsts` and
//!   the read/write pointer slots of `DeepCopyHeapPtrs`. Heap destinations
//!   reached through runtime pointers and internal GC stores are not included.
//!
//! ## Descriptors
//!
//! | Id | Property                                    | Condition                                                                                                                                                    | Rationale                                                |
//! |----|---------------------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------|----------------------------------------------------------|
//! | K1 | HeapNew allocates a struct or enum          | `desc(descriptor_id)` exists and is `Struct` or `Enum`                                                                                                       |                                                          |
//! | K2 | EnumNew names a real variant                | `desc(descriptor_id)` exists, is `Enum`, and `variant < len(variant_pointer_offsets)`                                                                        |                                                          |
//! | K3 | vector descriptors match the element stride | `VecPushBack`, `VecPack`: `desc(descriptor_id)` exists and is `Trivial`, or `Vector` with non-empty `elem_pointer_offsets` and `elem_size` equal to the op's | the GC strides a vector by the descriptor's element size |
//!
//! ## Calls
//!
//! For `CallNative` with ABI `abi`, and `CallDirect` to `callee`:
//!
//! | Id | Property                                     | Condition                                                                                                                                 | Rationale                                                    |
//! |----|----------------------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------|--------------------------------------------------------------|
//! | C1 | native frame fits                            | `S + M + abi.total_frame_size <= E`                                                                                                       |                                                              |
//! | C2 | native pointer slots are real argument slots | every `o` in `abi.heap_ptr_offsets` has `o + ptr.w <= abi.total_frame_size`, `o mod ptr.a = 0`, and lies inside an argument slot of `abi` | the GC reads them with aligned `read_ptr` at the native's fp |
//! | C3 | native descriptors exist                     | every `id` in `abi.required_descriptors` has `desc(id)`                                                                                   |                                                              |
//! | C4 | callee fits the callee region                | `callee.P <= E - (S + M)` and every slot in `callee.rets` has `end <= E - (S + M)`                                                        | arguments and results pass through `Callee`                  |
//!
//! C4 relies on the callee having passed its own signature checks before
//! publication. The final return slot bounds all returns, so C4 is O(1).
//!
//! ## Closures
//!
//! For `PackClosure` with `mask`, `cdd` (`captured_data_descriptor_id`), `values_size`, `captured`, and for `CallClosure` with `provided_args`:
//!
//! | Id  | Property                                       | Condition                                                                                                                                             | Rationale                                                                          |
//! |-----|------------------------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------|------------------------------------------------------------------------------------|
//! | L1  | captured data exists iff something is captured | `cdd` is `Some` iff `captured` is non-empty                                                                                                           |                                                                                    |
//! | L2  | captured-data descriptor is the right kind     | if `cdd = Some(id)`, `desc(id)` exists and is `Trivial` or `CapturedData`                                                                             |                                                                                    |
//! | L3  | captured-data pointers lie inside the values   | if `desc(id)` is `CapturedData { pointer_offsets }`, every `off` has `off + ptr.w <= values_size`                                                     | these descriptors are shared across closures of different sizes                    |
//! | L4  | captured slots are well-formed                 | every slot in `captured` is well-formed                                                                                                               | its alignment drives `align_up`, undefined for 0 or non-powers of two              |
//! | L5  | captured count matches the mask                | `len(captured) = popcount(mask)`                                                                                                                      |                                                                                    |
//! | L6  | mask fits the callee                           | if `func_ref = Resolved(callee)`: `len(callee.params) <= u64::BITS` and `mask >> len(callee.params) = 0`                                              |                                                                                    |
//! | L7  | captured values match the callee's parameters  | if `func_ref = Resolved(callee)`: for the `k`-th set bit `i` of `mask`, `captured[k].w = callee.params[i].w` and `captured[k].a = callee.params[i].a` | a captured value is written with its own `(w, a)` and read back at the parameter's |
//! | L8  | values size matches the captured layout        | the aligned packed size of `captured` fits in `u32` and equals `values_size`, with no final padding                                                   | checked arithmetic; source frame offsets do not affect the packed layout           |
//! | L9  | provided arguments are well-formed             | every slot in `provided_args` is well-formed                                                                                                          |                                                                                    |
//! | L10 | provided arguments lie in caller data          | every slot in `provided_args` has `end <= S`                                                                                                          | interleaved callee writes must not clobber later sources                           |
//!
//! With `func_ref = Unresolved(_)`, L6 and L7 are checked at call time against
//! the resolved callee.
//!
//! # Out of scope
//!
//! Not checked statically, because they depend on runtime data or dataflow:
//!
//! - the extent of a `SlotBorrow`;
//! - heap offsets against the pointee's size, for ops that carry only a
//!   pointer;
//! - `elem_size` against a vector's real stride;
//! - enum offset tables against the pointee's variant count;
//! - whether a GC layout slot holds a pointer at a given pc, or whether
//!   `base` within `[0, P)` matches the pointer positions of `param_tys`
//!   (that walk lives in the specializer and is what derives `base`, so
//!   re-running it here would check the specializer against itself);
//! - write-before-read of slots.
//!
//! Descriptor contents are validated by `ObjectDescriptor`'s constructors at
//! publish time. Branch gas fields are TODO(metering).

use crate::{
    align::{is_valid_align, MAX_ALIGN},
    native::NativeABI,
    types::{view_type, view_type_list, InternedType, Type, PTR_SLOT},
    ClosureFuncRef, CodeOffset, ConstantPoolProvider, DescriptorId, DescriptorProvider,
    FrameOffset, Function, LayoutProvider, MicroOp, ObjectDescriptorInner, OperandKind,
    PackClosureOp, SizedSlot, UnknownOperandLayout, CLOSURE_DESCRIPTOR_ID, ENUM_DATA_OFFSET,
    FRAME_METADATA_SIZE,
};
use smallvec::SmallVec;
use std::{fmt, ops::Range};

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct WellFormednessError {
    pub func_name: String,
    pub pc: Option<usize>,
    pub message: String,
}

impl fmt::Display for WellFormednessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.pc {
            Some(pc) => write!(f, "'{}', pc {}: {}", self.func_name, pc, self.message),
            None => write!(f, "'{}': {}", self.func_name, self.message),
        }
    }
}

/// Records a well-formedness error at `pc` (a `usize`, an `Option<usize>`, or
/// `None` for function-level errors) with a formatted message.
macro_rules! fail {
    ($self:ident, $pc:expr, $($arg:tt)*) => {
        $self.err($pc, format!($($arg)*))
    };
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Validate a single function against the providers. Returns an empty `Vec`
/// on success.
///
/// Complexity: O(N * log(N)) in the size of the function: its ops and their
/// operands, slot lists, and GC layouts.
pub fn check_well_formedness<P, C>(
    func: &Function,
    provider: &P,
    constants: &C,
) -> Vec<WellFormednessError>
where
    P: DescriptorProvider + LayoutProvider + ?Sized,
    C: ConstantPoolProvider + ?Sized,
{
    let mut errors = Vec::new();
    FunctionChecker {
        func,
        provider,
        constants,
        errors: &mut errors,
    }
    .run();
    errors
}

/// Panics with the checker's findings unless `function` is well-formed.
///
/// Complexity: O(N * log(N)) in the size of the function, as
/// [`check_well_formedness`].
pub fn assert_well_formed<P, C>(function: &Function, provider: &P, constants: &C)
where
    P: DescriptorProvider + LayoutProvider + ?Sized,
    C: ConstantPoolProvider + ?Sized,
{
    let errors = check_well_formedness(function, provider, constants);
    assert!(
        errors.is_empty(),
        "well-formedness check failed:\n{}",
        errors
            .iter()
            .map(|e| format!("  {}", e))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

// ---------------------------------------------------------------------------
// Per-function checker
// ---------------------------------------------------------------------------

struct FunctionChecker<'a, P: ?Sized, C: ?Sized> {
    func: &'a Function,
    provider: &'a P,
    constants: &'a C,
    errors: &'a mut Vec<WellFormednessError>,
}

impl<'a, P, C> FunctionChecker<'a, P, C>
where
    P: DescriptorProvider + LayoutProvider + ?Sized,
    C: ConstantPoolProvider + ?Sized,
{
    /// Runs every check and records each violation.
    ///
    /// Complexity: O(N * log(N)) in the size of the function: every operand is
    /// checked against the sorted GC layouts.
    fn run(&mut self) {
        let code = self.func.code.ops();
        // [F1]
        if code.is_empty() {
            self.err(None, "code must be non-empty");
        }
        // [F2]
        if code.last().is_some_and(|last| !last.is_terminator()) {
            self.err(
                Some(code.len() - 1),
                "last op must be a terminator (Return, Abort, AbortMsg, or Jump)",
            );
        }

        self.check_frame_geometry();
        self.check_param_and_return_slots();
        self.check_gc_layouts();

        // [F7]
        let origins = self.func.code.origins();
        if !origins.is_empty() && origins.len() != code.len() {
            fail!(
                self,
                None,
                "number of origins ({}) must be zero or equal number of micro-ops ({})",
                origins.len(),
                code.len()
            );
        }

        // [O1-O6]
        //
        // Operand accesses through the schema, then the op-specific checks.
        for (pc, instr) in code.iter().enumerate() {
            let safe_point = self
                .func
                .safe_point_layout_at(pc)
                .map(|layout| layout.heap_ptr_offsets.as_slice());
            let mut destinations = SmallVec::<[Range<u64>; 1]>::new();
            instr.for_each_frame_operand(&mut |off, kind, access| {
                if let Some(width) = self.check_frame_operand(Some(pc), safe_point, off, kind) {
                    if access.writes() && width != 0 {
                        destinations.push(off.0 as u64..off.0 as u64 + width as u64);
                    }
                }
            });
            self.check_disjoint_destinations(pc, &mut destinations);
            self.check_instruction(pc, instr);
        }
    }

    // -----------------------------------------------------------------------
    // Function-level checks
    // -----------------------------------------------------------------------

    fn check_frame_geometry(&mut self) {
        let func = self.func;
        // [F3]
        if func.frame_size() > func.extended_frame_size {
            fail!(self, None, "extended_frame_size ({}) must be >= frame_size() (param_and_local_sizes_sum {} + FRAME_METADATA_SIZE {} = {})", func.extended_frame_size, func.param_and_local_sizes_sum, FRAME_METADATA_SIZE, func.frame_size());
        }
        // [F4]
        if func.param_region_size > func.param_and_local_sizes_sum {
            fail!(
                self,
                None,
                "param_region_size ({}) must be <= param_and_local_sizes_sum ({})",
                func.param_region_size,
                func.param_and_local_sizes_sum
            );
        }
        // [F5]
        if !func.param_and_local_sizes_sum.is_multiple_of(MAX_ALIGN) {
            fail!(
                self,
                None,
                "param_and_local_sizes_sum ({}) must be {}-byte aligned",
                func.param_and_local_sizes_sum,
                MAX_ALIGN
            );
        }
        // [F6]
        if !func.frame_size().is_multiple_of(MAX_ALIGN) {
            fail!(
                self,
                None,
                "frame_size() ({}) must be {}-byte aligned so the callee fp is aligned",
                func.frame_size(),
                MAX_ALIGN
            );
        }
    }

    fn check_param_and_return_slots(&mut self) {
        let func = self.func;
        let param_end = self.check_signature_slots(
            "param",
            &func.param_slots,
            &func.param_tys,
            "param_region_size",
            func.param_region_size,
        );
        // [P5]
        if let Some(end) = param_end {
            if end as usize != func.param_region_size {
                fail!(
                    self,
                    None,
                    "param_region_size ({}) must equal parameter signature layout size ({end})",
                    func.param_region_size
                );
            }
        }
        self.check_signature_slots(
            "return",
            &func.return_slots,
            view_type_list(func.return_tys),
            "param_and_local_sizes_sum",
            func.param_and_local_sizes_sum,
        );
    }

    /// Slots match the signature's naturally aligned, contiguous layout at
    /// base zero. Parameters and returns are checked independently because
    /// they reuse storage. Returns the expected end, without trailing padding.
    // [P1-P4, R1-R4]
    fn check_signature_slots(
        &mut self,
        kind: &str,
        slots: &[SizedSlot],
        tys: &[InternedType],
        region_name: &str,
        region_end: usize,
    ) -> Option<u32> {
        if slots.len() != tys.len() {
            fail!(
                self,
                None,
                "number of {kind} slots ({}) must equal number of {kind} types ({})",
                slots.len(),
                tys.len()
            );
        }
        for (i, slot) in slots.iter().enumerate() {
            self.check_sized_slot(None, format_args!("{kind} slot {i}"), slot);
            let end = slot.end();
            if end > region_end {
                fail!(
                    self,
                    None,
                    "{kind} slot [{}, {end}) exceeds {region_name} ({region_end})",
                    slot.offset.0
                );
            }
        }
        let mut cursor = Some(0);
        for (i, &ty) in tys.iter().enumerate() {
            let Some((size, align)) = self.provider.size_and_align(ty) else {
                fail!(
                    self,
                    None,
                    "{kind} type {i} has no known size and alignment"
                );
                cursor = None;
                continue;
            };
            if let Some(slot) = slots.get(i) {
                if (slot.size, slot.align) != (size, align) {
                    fail!(
                        self,
                        None,
                        "{kind} slot {i}: size/alignment ({}, {}) != type layout ({size}, {align})",
                        slot.size,
                        slot.align
                    );
                }
            }
            let Some(start) = cursor else { continue };
            match checked_packed_slot(start, size, align) {
                Some((offset, end)) => {
                    if let Some(slot) = slots.get(i) {
                        if slot.offset.0 != offset {
                            fail!(
                                self,
                                None,
                                "{kind} slot {i}: offset {} != signature layout offset {offset}",
                                slot.offset.0
                            );
                        }
                    }
                    cursor = Some(end);
                },
                None => {
                    fail!(self, None, "{kind} signature layout at type {i} has invalid size/alignment or exceeds u32");
                    cursor = None;
                },
            }
        }
        if let Some(end) = cursor {
            if end as usize > region_end {
                fail!(
                    self,
                    None,
                    "{kind} signature layout size {end} exceeds {region_name} ({region_end})"
                );
            }
        }
        cursor
    }

    /// Slot well-formedness.
    // [P2, R2, L4, L9]
    fn check_sized_slot(&mut self, pc: Option<usize>, what: fmt::Arguments<'_>, slot: &SizedSlot) {
        if slot.size == 0 {
            fail!(self, pc, "{what}: size must be > 0");
        }
        if !is_valid_align(slot.align as usize) {
            fail!(
                self,
                pc,
                "{what}: align {} must be a power of two in [1, {}]",
                slot.align,
                MAX_ALIGN
            );
        } else if !slot.offset.0.is_multiple_of(slot.align) {
            fail!(
                self,
                pc,
                "{what}: offset {} is not {}-byte aligned",
                slot.offset.0,
                slot.align
            );
        }
    }

    /// Complexity: O(N * log(N)) in the number of layout slots: each safe-point
    /// slot is one binary search into `base`.
    // [G1-G7]
    fn check_gc_layouts(&mut self) {
        let code = self.func.code.ops();
        let base_offsets = &self.func.frame_layout.heap_ptr_offsets;
        let safe_points = self.func.safe_point_layouts.entries();

        // [G1, G2]
        self.check_pointer_offsets(None, base_offsets);

        // [G3]
        for off in base_offsets {
            if (off.0 as usize) < self.func.param_region_size {
                if !range_inside_slot(off.0, PTR_SLOT.0, &self.func.param_slots, |slot| {
                    (slot.offset.0, slot.size)
                }) {
                    fail!(
                        self,
                        None,
                        "frame_layout pointer slot {} is not inside one parameter slot",
                        off.0
                    );
                }
            } else if !self.func.zero_frame {
                fail!(self, None, "frame_layout names pointer slot {} beyond param_region_size ({}) but zero_frame is false", off.0, self.func.param_region_size);
            }
        }
        // [G4]
        if let Some(w) = safe_points
            .windows(2)
            .find(|w| w[0].code_offset.0 >= w[1].code_offset.0)
        {
            fail!(
                self,
                None,
                "safe_point_layouts: entries not strictly sorted (code_offset {} >= {})",
                w[0].code_offset.0,
                w[1].code_offset.0
            );
        }

        for entry in safe_points {
            let co = entry.code_offset.0 as usize;
            // [G5]
            match code.get(co) {
                None => fail!(self, None, "safe_point_layouts: code_offset {co} out of bounds (code length {})", code.len()),
                Some(op) if !op.is_allocating() => fail!(self, co, "safe_point_layouts: code_offset {co} is not at an allocating op; top-frame-only contract — see `SafePointEntry`"),
                Some(_) => {},
            }
            let sp_offsets = &entry.layout.heap_ptr_offsets;
            // [G6]
            self.check_pointer_offsets(Some(co), sp_offsets);
            // [G7]
            //
            // `base` is strictly increasing (G2), so each safe-point offset is
            // one binary search. If `base` is unsorted that is already
            // reported, and a duplicate missed here is moot.
            for sp in sp_offsets {
                if base_offsets.binary_search_by_key(&sp.0, |b| b.0).is_ok() {
                    fail!(
                        self,
                        co,
                        "safe_point_layouts: offset {} duplicates frame_layout",
                        sp.0
                    );
                }
            }
        }
    }

    /// Pointer offsets the GC reads with aligned `read_ptr`: each an in-frame,
    /// aligned pointer slot; the list strictly increasing.
    // [G1, G2, G6]
    fn check_pointer_offsets(&mut self, pc: Option<usize>, offsets: &[FrameOffset]) {
        for &off in offsets {
            self.check_frame_operand(pc, None, off, OperandKind::Ptr);
        }
        if let Some(w) = offsets.windows(2).find(|w| w[0].0 >= w[1].0) {
            fail!(
                self,
                pc,
                "pointer_offsets not strictly sorted ({} >= {})",
                w[0].0,
                w[1].0
            );
        }
    }

    // -----------------------------------------------------------------------
    // Per-instruction checks beyond the operand schema
    // -----------------------------------------------------------------------

    /// Op-specific checks beyond the operand schema.
    ///
    /// Complexity: O(N) in the op's immediate lists, except native argument
    /// containment, which uses binary search.
    fn check_instruction(&mut self, pc: usize, instr: &MicroOp) {
        use MicroOp::*;
        match *instr {
            // Fully described by the operand schema.
            StoreImm1 { .. }
            | StoreImm2 { .. }
            | StoreImm4 { .. }
            | StoreImm8 { .. }
            | StoreImm16 { .. }
            | StoreImm32 { .. }
            | Move8 { .. }
            | StoreRandomU64 { .. }
            | AddU64 { .. }
            | SubU64 { .. }
            | MulU64 { .. }
            | DivU64 { .. }
            | ModU64 { .. }
            | BitAndU64 { .. }
            | BitOrU64 { .. }
            | BitXorU64 { .. }
            | ShlU64 { .. }
            | ShrU64 { .. }
            | AddU64Imm { .. }
            | SubU64Imm { .. }
            | RSubU64Imm { .. }
            | MulU64Imm { .. }
            | IntAdd(_)
            | IntSub(_)
            | IntMul(_)
            | IntDiv(_)
            | IntMod(_)
            | IntCast(_)
            | IntCmp(_)
            | BoolNot { .. }
            | BoolAnd { .. }
            | BoolOr { .. }
            | CallIndirect { .. }
            | Return
            | Abort { .. }
            | AbortMsg { .. }
            | VecNew { .. }
            | VecLen { .. }
            | StoreImmVec { .. }
            | HeapBorrow { .. }
            | DeriveRefOffsetImm { .. }
            | Exists { .. }
            | BorrowGlobal { .. }
            | BorrowGlobalMut { .. }
            | MoveFrom { .. }
            | MoveTo { .. }
            | ForceGC
            | EnumTestTag { .. }
            | EnumCheckVariant { .. } => {},

            // [I1]
            DivU64Imm { imm, .. } | ModU64Imm { imm, .. } if imm == 0 => {
                self.err(pc, "division by zero (imm)");
            },
            DivU64Imm { .. } | ModU64Imm { .. } => {},
            // [I2]
            ShlU64Imm { imm, .. } | ShrU64Imm { imm, .. } if imm >= 64 => {
                fail!(self, pc, "shift amount {imm} exceeds 63 (imm)");
            },
            ShlU64Imm { .. } | ShrU64Imm { .. } => {},

            // [I3]
            IntBitAnd(ref op) | IntBitOr(ref op) | IntBitXor(ref op) if op.rhs.is_signed() => {
                self.err(pc, "bitwise on signed type");
            },
            IntBitAnd(_) | IntBitOr(_) | IntBitXor(_) => {},
            // [I4]
            IntShl(op) | IntShr(op) if op.ty.is_signed() => {
                self.err(pc, "shift on signed type");
            },
            IntShl(_) | IntShr(_) => {},
            // [I5]
            IntNegate(op) if !op.ty.is_signed() => {
                self.err(pc, "negate on unsigned type");
            },
            IntNegate(_) => {},

            // [Z1]
            Move { size, .. } => self.check_nonzero_size(pc, size),

            // [B1]
            SlotBorrow { local, .. } if local.0 as usize >= self.func.param_and_local_sizes_sum => {
                fail!(
                    self,
                    pc,
                    "SlotBorrow local {} is outside the data region [0, {})",
                    local.0,
                    self.func.param_and_local_sizes_sum
                );
            },
            SlotBorrow { .. } => {},

            // [J1]
            Jump { target, .. }
            | JumpNotZeroU64 { target, .. }
            | JumpNotZeroByte { target, .. }
            | JumpZeroByte { target, .. }
            | JumpGreaterEqualU64Imm { target, .. }
            | JumpLessU64Imm { target, .. }
            | JumpGreaterU64Imm { target, .. }
            | JumpLessEqualU64Imm { target, .. }
            | JumpLessU64 { target, .. }
            | JumpGreaterEqualU64 { target, .. }
            | JumpNotEqualU64 { target, .. } => self.check_jump(pc, target),
            JumpIntCmp(ref op) => self.check_jump(pc, op.target),
            // [J1, O3]
            JumpValueCmp(ref op) => {
                self.check_value_type(pc, op.ty);
                self.check_jump(pc, op.target);
            },
            JumpValueRefCmp(ref op) => {
                self.check_value_type(pc, op.ty);
                self.check_jump(pc, op.target);
            },
            // [O3]
            ValueCmp(ref op) => self.check_value_type(pc, op.ty),
            ValueRefCmp(ref op) => self.check_value_type(pc, op.ty),

            // [C4]
            CallDirect { ref ptr } => {
                // SAFETY: the function pointer lives in the global context,
                // which the caller's guard keeps alive during the check.
                let callee = unsafe { ptr.as_ref_unchecked() };
                self.check_direct_callee(pc, callee);
            },
            // [C1, C2, C3]
            CallNative { ref abi, .. } => self.check_native_abi(pc, abi),

            // [Z1, Z2]
            HeapMoveFrom8 { offset, .. }
            | HeapMoveTo8 { offset, .. }
            | HeapMoveToImm8 { offset, .. } => {
                self.check_offset_size(pc, offset, PTR_SLOT.0);
            },
            HeapMoveFrom { offset, size, .. }
            | HeapMoveTo { offset, size, .. }
            | HeapReadOffset { offset, size, .. }
            | HeapWriteOffset { offset, size, .. }
            | ReadRefOffset { offset, size, .. }
            | WriteRefOffset { offset, size, .. } => {
                self.check_nonzero_size(pc, size);
                self.check_offset_size(pc, offset, size);
            },
            ReadRef { size, .. } | WriteRef { size, .. } => self.check_nonzero_size(pc, size),
            EnumReadVariantFieldByTag {
                ref offsets, size, ..
            }
            | EnumWriteVariantFieldByTag {
                ref offsets, size, ..
            } => {
                self.check_nonzero_size(pc, size);
                for &offset in offsets.iter().flatten() {
                    self.check_enum_field_offset(pc, offset);
                    self.check_offset_size(pc, offset, size);
                }
            },
            EnumBorrowVariantFieldByTag { ref offsets, .. } => {
                for &offset in offsets.iter().flatten() {
                    self.check_enum_field_offset(pc, offset);
                }
            },
            // [Z3]
            //
            // The schema reports the saturated slot; this reports the overflow
            // itself.
            DeepCopyHeapPtrs { base, ref offsets } => {
                for &off in offsets
                    .iter()
                    .filter(|&&off| base.0.checked_add(off).is_none())
                {
                    fail!(
                        self,
                        pc,
                        "DeepCopyHeapPtrs: base {} + offset {off} overflows u32",
                        base.0
                    );
                }
            },

            // [Z1, K3]
            VecPushBack {
                elem_size,
                descriptor_id,
                ..
            } => {
                self.check_nonzero_size(pc, elem_size);
                self.check_vector_descriptor(pc, "VecPushBack", descriptor_id, elem_size);
            },
            VecPopBack { elem_size, .. }
            | VecLoadElem { elem_size, .. }
            | VecStoreElem { elem_size, .. }
            | VecSwap { elem_size, .. }
            | VecBorrow { elem_size, .. } => self.check_nonzero_size(pc, elem_size),
            VecPack(ref op) => {
                self.check_nonzero_size(pc, op.elem_size);
                self.check_vector_descriptor(pc, "VecPack", op.descriptor_id, op.elem_size);
            },
            VecUnpack(ref op) => self.check_nonzero_size(pc, op.elem_size),

            // [K1]
            HeapNew { descriptor_id, .. } => {
                if let Some(inner) = self.descriptor_or_report(pc, "HeapNew", descriptor_id) {
                    if !matches!(
                        inner,
                        ObjectDescriptorInner::Struct { .. } | ObjectDescriptorInner::Enum { .. }
                    ) {
                        fail!(
                            self,
                            pc,
                            "HeapNew: descriptor_id {descriptor_id} is not a Struct or Enum"
                        );
                    }
                }
            },
            // [K2]
            EnumNew {
                descriptor_id,
                variant,
                ..
            } => match self.descriptor_or_report(pc, "EnumNew", descriptor_id) {
                Some(ObjectDescriptorInner::Enum {
                    variant_pointer_offsets,
                    ..
                }) => {
                    let variant_count = variant_pointer_offsets.len();
                    if variant as usize >= variant_count {
                        fail!(self, pc, "EnumNew: tag {variant} out of range (descriptor {descriptor_id} has {variant_count} variants)");
                    }
                },
                Some(_) => fail!(
                    self,
                    pc,
                    "EnumNew: descriptor_id {descriptor_id} is not an Enum"
                ),
                None => {},
            },
            // [L1-L8]
            PackClosure(ref op) => self.check_pack_closure(pc, op),
            // [L9, L10]
            CallClosure(ref op) => {
                for (i, slot) in op.provided_args.iter().enumerate() {
                    self.check_sized_slot(
                        Some(pc),
                        format_args!("CallClosure: provided_args[{i}]"),
                        slot,
                    );
                    if slot.end() > self.func.param_and_local_sizes_sum {
                        fail!(
                            self,
                            pc,
                            "CallClosure: provided_args[{i}] ends at {}, beyond data region ({})",
                            slot.end(),
                            self.func.param_and_local_sizes_sum
                        );
                    }
                }
            },
        }
    }

    fn check_pack_closure(&mut self, pc: usize, op: &PackClosureOp) {
        // The closure object uses the implicit reserved `CLOSURE_DESCRIPTOR_ID`
        // (no per-op field); every provider installs `Closure` there.
        debug_assert!(
            matches!(
                self.provider
                    .descriptor(CLOSURE_DESCRIPTOR_ID)
                    .map(|d| d.inner()),
                Some(ObjectDescriptorInner::Closure)
            ),
            "reserved descriptor[{}] must be Closure",
            CLOSURE_DESCRIPTOR_ID
        );
        // [L1, L2, L3]
        match (op.captured_data_descriptor_id, op.captured.is_empty()) {
            (None, true) => {},
            (Some(id), false) => match self.descriptor_or_report(pc, "PackClosure", id) {
                Some(ObjectDescriptorInner::Trivial) | None => {},
                Some(ObjectDescriptorInner::CapturedData { pointer_offsets }) => {
                    // The constructor keeps `pointer_offsets` strictly
                    // increasing, so the last one bounds them all: O(1) per
                    // op regardless of the descriptor's size.
                    if let Some(&off) = pointer_offsets
                        .last()
                        .filter(|&&off| off as u64 + PTR_SLOT.0 as u64 > op.values_size as u64)
                    {
                        fail!(self, pc, "PackClosure: captured_data pointer offset {off} out of bounds of values_size {}", op.values_size);
                    }
                },
                Some(_) => fail!(
                    self,
                    pc,
                    "PackClosure: descriptor_id {id} is not a Trivial or CapturedData"
                ),
            },
            (Some(id), true) => fail!(
                self,
                pc,
                "PackClosure: captured_data_descriptor_id {id} provided but no captures"
            ),
            (None, false) => self.err(
                Some(pc),
                "PackClosure: captured non-empty but captured_data_descriptor_id is None",
            ),
        }
        let mut all_valid = true;
        // [L4]
        for (i, slot) in op.captured.iter().enumerate() {
            self.check_sized_slot(Some(pc), format_args!("PackClosure: captured[{i}]"), slot);
            all_valid &= slot.size > 0 && is_valid_align(slot.align as usize);
        }
        let captured_count = op.mask.count_ones() as usize;
        // [L5]
        if op.captured.len() != captured_count {
            fail!(self, pc, "PackClosure: captured list length {} does not match mask captured count {captured_count}", op.captured.len());
        }
        match &op.func_ref {
            ClosureFuncRef::Resolved(func_ptr) => {
                // SAFETY: the function pointer lives in the global context,
                // which the caller's guard keeps alive during the check.
                let callee = unsafe { func_ptr.as_ref_unchecked() };
                let param_count = callee.param_slots.len();
                // [L6]
                if param_count > u64::BITS as usize {
                    fail!(self, pc, "PackClosure: callee has {param_count} params, exceeds 64-bit mask capacity");
                } else if op
                    .mask
                    .checked_shr(param_count as u32)
                    .is_some_and(|m| m != 0)
                {
                    fail!(self, pc, "PackClosure: mask 0x{:x} sets bits beyond callee param count {param_count}", op.mask);
                }
                // The captured list is in mask-bit-set order through the param
                // list. Bounded to the 64 parameters a mask can address; more is
                // already an L6 error.
                let captured_params = callee
                    .param_slots
                    .iter()
                    .take(u64::BITS as usize)
                    .enumerate()
                    .filter(|(i, _)| (op.mask >> i) & 1 != 0);
                // [L7]
                for (k, ((i, param_slot), slot)) in captured_params.zip(&op.captured).enumerate() {
                    if slot.size != param_slot.size {
                        fail!(
                            self,
                            pc,
                            "PackClosure: captured[{k}].size {} != callee param_slots[{i}].size {}",
                            slot.size,
                            param_slot.size
                        );
                    }
                    if slot.align != param_slot.align {
                        fail!(self, pc, "PackClosure: captured[{k}].align {} != callee param_slots[{i}].align {}", slot.align, param_slot.align);
                    }
                }
            },
            // Symbolic target: the callee-dependent checks run at call time
            // against the resolved callee.
            ClosureFuncRef::Unresolved(_) => {},
        }
        // [L8]
        //
        // Skipped when a captured size/alignment is invalid (already reported),
        // since the layout is then undefined.
        if all_valid {
            let expected = op.captured.iter().try_fold(0, |cursor, slot| {
                checked_packed_slot(cursor, slot.size, slot.align).map(|(_, end)| end)
            });
            let Some(expected) = expected else {
                return self.err(pc, "PackClosure: captured layout size exceeds u32");
            };
            if op.values_size != expected {
                fail!(
                    self,
                    pc,
                    "PackClosure: values_size {} != captured layout size {expected}",
                    op.values_size
                );
            }
        }
    }

    // -----------------------------------------------------------------------
    // Frame access
    // -----------------------------------------------------------------------

    fn err(&mut self, pc: impl Into<Option<usize>>, msg: impl Into<String>) {
        self.errors.push(WellFormednessError {
            func_name: self.func.name().to_string(),
            pc: pc.into(),
            message: msg.into(),
        });
    }

    /// Checks one frame operand per the [`OperandKind`] schema and returns
    /// its width when known. `safe_point` is the layout at its pc, if any.
    ///
    /// Complexity: O(log(N)) in the number of GC pointer slots (O6).
    // [O1-O6]
    fn check_frame_operand(
        &mut self,
        pc: Option<usize>,
        safe_point: Option<&[FrameOffset]>,
        offset: FrameOffset,
        kind: OperandKind,
    ) -> Option<u32> {
        // [O2, O4, O5]
        let (width, align) = match kind.width_and_align(self.provider, self.constants) {
            Ok(wa) => wa,
            Err(UnknownOperandLayout::NoLayout(_)) => {
                self.err(pc, "value comparison operand type has no published layout");
                return None;
            },
            Err(UnknownOperandLayout::NoConstant(idx)) => {
                fail!(
                    self,
                    pc,
                    "StoreImmVec: constant pool index {} out of range",
                    idx.0
                );
                return None;
            },
            Err(UnknownOperandLayout::ConstantHasNoLayout(idx)) => {
                fail!(
                    self,
                    pc,
                    "StoreImmVec: constant {} has no published layout",
                    idx.0
                );
                return None;
            },
        };
        self.check_access(pc, offset, width, align);
        // [O6]
        if let Some(range) = kind.non_pointer_range(self.provider, self.constants) {
            self.check_not_pointer_slot(
                pc,
                safe_point,
                offset.0 as u64 + range.start as u64..offset.0 as u64 + range.end as u64,
            );
        }
        Some(width)
    }

    /// `range` does not intersect any pointer slot in
    /// `frame_layout` or in `safe_point`, the layout at the operand's pc.
    ///
    /// Complexity: O(log(N)) in the number of pointer slots: both lists are
    /// strictly increasing (G2, G6), so each is one binary search.
    // [O6]
    fn check_not_pointer_slot(
        &mut self,
        pc: Option<usize>,
        safe_point: Option<&[FrameOffset]>,
        range: Range<u64>,
    ) {
        let Range { start, end } = range;
        if start == end {
            return;
        }
        let ptr_width = PTR_SLOT.0 as u64;
        let overlapping = |slots: &[FrameOffset]| -> Option<u32> {
            // First pointer slot ending after `start`; it overlaps iff it
            // begins before `end`.
            let i = slots.partition_point(|b| b.0 as u64 + ptr_width <= start);
            slots.get(i).filter(|b| (b.0 as u64) < end).map(|b| b.0)
        };
        let base = &self.func.frame_layout.heap_ptr_offsets;
        if let Some(b) = overlapping(base) {
            fail!(
                self,
                pc,
                "access [{start}, {end}) aliases frame_layout pointer slot {b}"
            );
        }
        if let Some(b) = safe_point.and_then(overlapping) {
            fail!(
                self,
                pc,
                "access [{start}, {end}) aliases safe-point pointer slot {b}"
            );
        }
    }

    /// The access `(offset, size, align)` is valid.
    // [O1]
    fn check_access(&mut self, pc: Option<usize>, offset: FrameOffset, size: u32, align: u32) {
        let offset = offset.0 as usize;
        let Some(end) = offset.checked_add(size as usize) else {
            return fail!(self, pc, "access at offset {offset} overflows");
        };
        if end > self.func.extended_frame_size {
            return fail!(
                self,
                pc,
                "access [{offset}, {end}) exceeds extended_frame_size {}",
                self.func.extended_frame_size
            );
        }
        let meta_start = self.func.param_and_local_sizes_sum;
        let meta_end = meta_start + FRAME_METADATA_SIZE;
        if offset < meta_end && meta_start < end {
            fail!(
                self,
                pc,
                "access [{offset}, {end}) overlaps metadata [{meta_start}, {meta_end})"
            );
        }
        if !offset.is_multiple_of(align as usize) {
            fail!(
                self,
                pc,
                "access [{offset}, {end}) is not {align}-byte aligned"
            );
        }
    }

    // [O2, O3]
    fn check_value_type(&mut self, pc: usize, ty: InternedType) {
        if self.provider.layout_by_ty(ty).is_none() {
            self.err(pc, "value comparison type has no published layout");
        }
        if matches!(view_type(ty), Type::ImmutRef { .. } | Type::MutRef { .. }) {
            self.err(pc, "value comparison on a reference type");
        }
    }

    /// Complexity: O(N * log(N)) in the number of destinations: they are
    /// sorted to find overlaps.
    // [D1]
    fn check_disjoint_destinations(&mut self, pc: usize, dsts: &mut [Range<u64>]) {
        dsts.sort_unstable_by_key(|range| range.start);
        if let Some(w) = dsts.windows(2).find(|w| w[0].end > w[1].start) {
            fail!(
                self,
                pc,
                "destinations [{}, {}) and [{}, {}) overlap",
                w[0].start,
                w[0].end,
                w[1].start,
                w[1].end
            );
        }
    }

    // -----------------------------------------------------------------------
    // Calls
    // -----------------------------------------------------------------------

    /// Complexity: O(N * log(N)) in the ABI's pointer offsets and argument
    /// slots: each offset binary-searches the sorted argument slots.
    // [C1, C2, C3]
    fn check_native_abi(&mut self, pc: usize, abi: &NativeABI) {
        let base = self.func.frame_size();
        let total = abi.total_frame_size();
        // [C1]
        match base.checked_add(total as usize) {
            None => self.err(pc, "native slot region overflows usize"),
            Some(end) if end > self.func.extended_frame_size => fail!(
                self,
                pc,
                "native slot region [{base}, {end}) exceeds extended_frame_size {}",
                self.func.extended_frame_size
            ),
            Some(_) => {},
        }
        // [C2]
        for &off in abi.heap_ptr_offsets() {
            let off_end = off.0 as u64 + PTR_SLOT.0 as u64;
            if off_end > total as u64 {
                fail!(
                    self,
                    pc,
                    "native heap pointer offset {} exceeds the slot region ({total})",
                    off.0
                );
            }
            if !off.0.is_multiple_of(PTR_SLOT.1) {
                fail!(
                    self,
                    pc,
                    "native heap pointer offset {} is not {}-byte aligned",
                    off.0,
                    PTR_SLOT.1
                );
            }
            // `args` is sorted by offset and non-overlapping (`NativeABI::new`),
            // so the only candidate is the last slot starting at or before
            // `off`.
            let inside_arg = range_inside_slot(off.0, PTR_SLOT.0, abi.args(), |slot| {
                (slot.offset, slot.size)
            });
            if !inside_arg {
                fail!(
                    self,
                    pc,
                    "native heap pointer offset {} is not inside an argument slot",
                    off.0
                );
            }
        }
        // [C3]
        for (i, id) in abi.required_descriptors().iter().enumerate() {
            if self.provider.descriptor(*id).is_none() {
                fail!(self, pc, "native required descriptor {i} ({id}) is unknown");
            }
        }
    }

    // [C4]
    fn check_direct_callee(&mut self, pc: usize, callee: &Function) {
        let region = self
            .func
            .extended_frame_size
            .saturating_sub(self.func.frame_size());
        if callee.param_region_size > region {
            fail!(
                self,
                pc,
                "CallDirect: callee param_region_size {} exceeds the callee region ({region})",
                callee.param_region_size
            );
        }
        // Callees are checked before publication, so return slots are sorted
        // and disjoint. Their final end bounds the whole list in O(1).
        if let Some((i, slot)) = callee.return_slots.iter().enumerate().next_back() {
            let end = slot.end();
            if end > region {
                fail!(self, pc, "CallDirect: callee return slot {i} [{}, {end}) exceeds the callee region ({region})", slot.offset.0);
            }
        }
    }

    // -----------------------------------------------------------------------
    // Descriptors and small checks
    // -----------------------------------------------------------------------

    /// Resolves `descriptor_id`, reporting an unknown id on behalf of `op`.
    /// The result borrows the provider, not the checker, so callers can keep
    /// reporting while holding it.
    fn descriptor_or_report(
        &mut self,
        pc: usize,
        op: &str,
        descriptor_id: DescriptorId,
    ) -> Option<&'a ObjectDescriptorInner> {
        let provider: &'a P = self.provider;
        let inner = provider.descriptor(descriptor_id).map(|d| d.inner());
        if inner.is_none() {
            fail!(self, pc, "{op}: unknown descriptor_id {descriptor_id}");
        }
        inner
    }

    // [K3]
    fn check_vector_descriptor(
        &mut self,
        pc: usize,
        op: &str,
        descriptor_id: DescriptorId,
        elem_size: u32,
    ) {
        match self.descriptor_or_report(pc, op, descriptor_id) {
            None | Some(ObjectDescriptorInner::Trivial) => {},
            Some(ObjectDescriptorInner::Vector {
                elem_size: descriptor_elem_size,
                elem_pointer_offsets,
            }) if !elem_pointer_offsets.is_empty() => {
                if *descriptor_elem_size != elem_size {
                    fail!(self, pc, "{op}: elem_size {elem_size} does not match Vector descriptor_id {descriptor_id} elem_size {descriptor_elem_size}");
                }
            },
            Some(_) => fail!(
                self,
                pc,
                "{op}: descriptor_id {descriptor_id} is not a non-empty Vector or Trivial"
            ),
        }
    }

    // TODO(metering): validate branch gas fields are populated.
    // [J1]
    fn check_jump(&mut self, pc: usize, target: CodeOffset) {
        let code_len = self.func.code.ops().len();
        if (target.0 as usize) >= code_len {
            fail!(
                self,
                pc,
                "jump target {} out of bounds (code length {code_len})",
                target.0
            );
        }
    }

    // [Z1]
    fn check_nonzero_size(&mut self, pc: usize, size: u32) {
        if size == 0 {
            self.err(pc, "size must be > 0");
        }
    }

    // [Z2]
    fn check_offset_size(&mut self, pc: usize, offset: u32, size: u32) {
        if offset.checked_add(size).is_none() {
            fail!(self, pc, "offset {offset} + size {size} overflows u32");
        }
    }

    // [Z4]
    fn check_enum_field_offset(&mut self, pc: usize, offset: u32) {
        if offset < ENUM_DATA_OFFSET as u32 {
            fail!(
                self,
                pc,
                "enum field offset {offset} is below ENUM_DATA_OFFSET ({ENUM_DATA_OFFSET})"
            );
        }
    }
}

/// Place a positive-sized value at its natural alignment, without final
/// padding. Used for signature slots and captured payloads; all arithmetic
/// stays checked in the u32 representation consumed by the micro-ops.
fn checked_packed_slot(cursor: u32, size: u32, align: u32) -> Option<(u32, u32)> {
    if size == 0 || !is_valid_align(align as usize) {
        return None;
    }
    let offset = cursor.checked_add(align - 1)? & !(align - 1);
    Some((offset, offset.checked_add(size)?))
}

/// Whether a range fits within one slot of a sorted, disjoint slot list.
/// Native ABIs guarantee that ordering at construction; malformed function
/// parameter lists are independently rejected by the signature checks.
fn range_inside_slot<T>(
    start: u32,
    width: u32,
    slots: &[T],
    bounds: impl Fn(&T) -> (u32, u32),
) -> bool {
    let i = slots.partition_point(|slot| bounds(slot).0 <= start);
    slots[..i].last().is_some_and(|slot| {
        let (offset, size) = bounds(slot);
        start as u64 + width as u64 <= offset as u64 + size as u64
    })
}
