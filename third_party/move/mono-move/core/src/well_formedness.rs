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
//! implementation cites where the check is evaluated. All checks run and every
//! violation is reported; none depends on another having passed.
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
//! | `layout(ty)`          | `(size, align)` of `ty`, or none                                                       |
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
//! # Checks
//!
//! ## Function shape
//!
//! | Id | Property                       | Condition                                                 | Rationale                                                                 |
//! |----|--------------------------------|-----------------------------------------------------------|---------------------------------------------------------------------------|
//! | F1 | has code                       | `N >= 1`                                                  |                                                                           |
//! | F2 | cannot run off the end         | `code[N - 1]` is `Return`, `Abort`, `AbortMsg`, or `Jump` | every other op falls through to `pc + 1`; a call returns to `call_pc + 1` |
//! | F3 | frame holds its metadata       | `S + M <= E`                                              |                                                                           |
//! | F4 | parameters fit the data region | `P <= S`                                                  |                                                                           |
//! | F5 | metadata is aligned            | `S mod A = 0`                                             | `Meta` is written with aligned `u64` stores                               |
//! | F6 | callee fp is aligned           | `(S + M) mod A = 0`                                       | a callee's frame pointer is `fp + S + M`                                  |
//! | F7 | origins match code             | `len(origins) = 0` or `len(origins) = N`                  |                                                                           |
//!
//! ## Parameter and return slots
//!
//! | Id | Property                               | Condition                             | Rationale                                                                 |
//! |----|----------------------------------------|---------------------------------------|---------------------------------------------------------------------------|
//! | P1 | one slot per parameter                 | `len(params) = len(param_tys)`        |                                                                           |
//! | P2 | parameter slots are well-formed        | every slot in `params` is well-formed |                                                                           |
//! | P3 | parameters lie in the parameter region | every slot in `params` has `end <= P` | callers write each parameter at its slot, then the callee zeroes `[P, E)` |
//! | P4 | parameters do not overlap              | `params` is ascending and disjoint    |                                                                           |
//! | R1 | one slot per return value              | `len(rets) = len(ret_tys)`            |                                                                           |
//! | R2 | return slots are well-formed           | every slot in `rets` is well-formed   |                                                                           |
//! | R3 | return values lie in the data region   | every slot in `rets` has `end <= S`   |                                                                           |
//! | R4 | return values do not overlap           | `rets` is ascending and disjoint      |                                                                           |
//!
//! ## GC layouts
//!
//! | Id | Property                                  | Condition                                                       | Rationale                                                                                        |
//! |----|-------------------------------------------|-----------------------------------------------------------------|--------------------------------------------------------------------------------------------------|
//! | G1 | base pointer slots are real slots         | every `o` in `base` is a valid access `(o, ptr.w, ptr.a)`       |                                                                                                  |
//! | G2 | base layout is sorted                     | `base` is strictly increasing                                   |                                                                                                  |
//! | G3 | unwritten pointer slots start null        | if some `o` in `base` has `o >= P`, then `F.zero_frame`         | the GC scans `base` before the function writes anything; the caller does not fill slots past `P` |
//! | G4 | safe points are sorted                    | the `code_offset`s of `sps` are strictly increasing             |                                                                                                  |
//! | G5 | safe points sit at allocating ops         | every `code_offset < N` and `code[code_offset].is_allocating()` |                                                                                                  |
//! | G6 | safe-point pointer slots are real slots   | every entry's `heap_ptr_offsets` satisfies G1 and G2            |                                                                                                  |
//! | G7 | safe points do not repeat the base layout | no offset is in both an entry's `heap_ptr_offsets` and `base`   |                                                                                                  |
//!
//! ## Operand accesses
//!
//! For every `pc` and every `(o, kind)` reported by `code[pc].for_each_frame_operand` (see `instruction::operands`):
//!
//! | Id | Property                                                        | Condition                                                                                                                                                           | Rationale                                                                                      |
//! |----|-----------------------------------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------|------------------------------------------------------------------------------------------------|
//! | O1 | every operand access is in frame, off the metadata, and aligned | the access `(o, w, a)` is valid, with `(w, a)` = `OperandKind::width_and_align`, or `layout(ty)` for `Value(ty)`, or `layout(const_ty(idx))` for `Constant(idx)`    |                                                                                                |
//! | O2 | compared values have a layout                                   | for `Value(ty)`, `layout(ty)` exists                                                                                                                                |                                                                                                |
//! | O3 | comparison is on values, not references                         | for `Value(ty)`, and for the `ty` of `ValueRefCmp` and `JumpValueRefCmp`, `ty` is not a reference type                                                              |                                                                                                |
//! | O4 | constants exist                                                 | for `Constant(idx)`, `const_ty(idx)` exists                                                                                                                         |                                                                                                |
//! | O5 | constants have a layout                                         | for `Constant(idx)`, `layout(const_ty(idx))` exists                                                                                                                 |                                                                                                |
//! | O6 | scalars do not alias GC pointer slots                           | for `kind` in {`Bool`, `Byte`, `U64`, `Int`, `Address`}, `[o, o + w)` does not intersect `[b, b + ptr.w)` for any `b` in `base` or in the safe-point layout at `pc` | a pointer slot read or written as an integer is type confusion; the GC would trace the integer |
//!
//! ## Instruction-local invariants
//!
//! | Id | Property                               | Condition                                                                |
//! |----|----------------------------------------|--------------------------------------------------------------------------|
//! | I1 | no unchecked division by zero          | `DivU64Imm`, `ModU64Imm`: `imm != 0`                                     |
//! | I2 | no unchecked over-shift                | `ShlU64Imm`, `ShrU64Imm`: `imm < 64`                                     |
//! | I3 | bitwise ops are unsigned               | `IntBitAnd`, `IntBitOr`, `IntBitXor`: `rhs` is unsigned                  |
//! | I4 | shifts are unsigned                    | `IntShl`, `IntShr`: `ty` is unsigned                                     |
//! | I5 | negation is signed                     | `IntNegate`: `ty` is signed                                              |
//! | Z1 | copies move at least one byte          | `size > 0`, or `elem_size > 0`, for the ops listed below                 |
//! | Z2 | heap and reference windows do not wrap | `offset + size` (or `offset + 8`) fits in `u32` for the ops listed below |
//! | Z3 | deep-copy slot offsets do not wrap     | `DeepCopyHeapPtrs`: `base + off` fits in `u32` for every `off`           |
//! | J1 | jumps stay in the function             | every jump `target < N`                                                  |
//! | D1 | multiple destinations do not overlap   | an op writing more than one frame destination writes disjoint ranges     |
//! | B1 | borrowed locals are in the data region | `SlotBorrow`: `local < S`                                                |
//!
//! - Z1 `size`: `Move`, `ReadRef`, `WriteRef`, `ReadRefOffset`, `WriteRefOffset`,
//!   `HeapReadOffset`, `HeapWriteOffset`, `HeapMoveFrom`, `HeapMoveTo`,
//!   `EnumReadVariantFieldByTag`, `EnumWriteVariantFieldByTag`. Z1 `elem_size`:
//!   `VecPushBack`, `VecPopBack`, `VecLoadElem`, `VecStoreElem`, `VecSwap`,
//!   `VecBorrow`, `VecPack`, `VecUnpack`.
//! - Z2 `offset + size`: `HeapMoveFrom`, `HeapMoveTo`, `HeapReadOffset`,
//!   `HeapWriteOffset`, `ReadRefOffset`, `WriteRefOffset`, and every `Some(offset)`
//!   in the tables of `EnumReadVariantFieldByTag` and `EnumWriteVariantFieldByTag`.
//!   Z2 `offset + 8`: `HeapMoveFrom8`, `HeapMoveTo8`, `HeapMoveToImm8`.
//! - D1: today only `VecUnpack`, with destinations `[d, d + elem_size)` for each
//!   `d` in `dsts`.
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
//! ## Closures
//!
//! For `PackClosure` with `mask`, `cdd` (`captured_data_descriptor_id`), `values_size`, `captured`, and for `CallClosure` with `provided_args`:
//!
//! | Id | Property                                       | Condition                                                                                                                                             | Rationale                                                                          |
//! |----|------------------------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------|------------------------------------------------------------------------------------|
//! | L1 | captured data exists iff something is captured | `cdd` is `Some` iff `captured` is non-empty                                                                                                           |                                                                                    |
//! | L2 | captured-data descriptor is the right kind     | if `cdd = Some(id)`, `desc(id)` exists and is `Trivial` or `CapturedData`                                                                             |                                                                                    |
//! | L3 | captured-data pointers lie inside the values   | if `desc(id)` is `CapturedData { pointer_offsets }`, every `off` has `off + ptr.w <= values_size`                                                     | these descriptors are shared across closures of different sizes                    |
//! | L4 | captured slots are well-formed                 | every slot in `captured` is well-formed                                                                                                               | its alignment drives `align_up`, undefined for 0 or non-powers of two              |
//! | L5 | captured count matches the mask                | `len(captured) = popcount(mask)`                                                                                                                      |                                                                                    |
//! | L6 | mask fits the callee                           | if `func_ref = Resolved(callee)`: `len(callee.params) <= u64::BITS` and `mask >> len(callee.params) = 0`                                              |                                                                                    |
//! | L7 | captured values match the callee's parameters  | if `func_ref = Resolved(callee)`: for the `k`-th set bit `i` of `mask`, `captured[k].w = callee.params[i].w` and `captured[k].a = callee.params[i].a` | a captured value is written with its own `(w, a)` and read back at the parameter's |
//! | L8 | values size matches the captured layout        | if every slot in `captured` has a valid `a`, `values_size = captured_values_size(captured)`                                                           |                                                                                    |
//! | L9 | provided arguments are well-formed             | every slot in `provided_args` is well-formed                                                                                                          |                                                                                    |
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
    align::MAX_ALIGN,
    captured_values_size,
    interner::InternedModuleId,
    native::NativeABI,
    types::{view_type, view_type_list, InternedType, Type, PTR_SLOT},
    ClosureFuncRef, CodeOffset, ConstantPoolIndex, DescriptorId, DescriptorProvider, FrameOffset,
    Function, LayoutProvider, MicroOp, ObjectDescriptorInner, OperandKind, PackClosureOp,
    SizedSlot, CLOSURE_DESCRIPTOR_ID, FRAME_METADATA_SIZE,
};
use mono_move_checks_macro::checks;
use std::fmt;

// ---------------------------------------------------------------------------
// Providers
// ---------------------------------------------------------------------------

/// Constant-pool view of the modules whose functions are being checked.
pub trait ConstantPoolProvider {
    /// Interned type of constant `idx` in `module_id`'s pool, or `None` if
    /// the module is unknown or `idx` is out of range.
    fn constant_type(
        &self,
        module_id: InternedModuleId,
        idx: ConstantPoolIndex,
    ) -> Option<InternedType>;
}

/// Everything the checker needs to resolve a function's operands.
pub trait WellFormednessProvider:
    DescriptorProvider + LayoutProvider + ConstantPoolProvider
{
}

impl<P: DescriptorProvider + LayoutProvider + ConstantPoolProvider + ?Sized> WellFormednessProvider
    for P
{
}

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
pub fn check_well_formedness<P: WellFormednessProvider + ?Sized>(
    func: &Function,
    provider: &P,
) -> Vec<WellFormednessError> {
    let mut errors = Vec::new();
    FunctionChecker {
        func,
        provider,
        errors: &mut errors,
    }
    .run();
    errors
}

/// Panics with the checker's findings unless `function` is well-formed.
///
/// Complexity: O(N * log(N)) in the size of the function, as
/// [`check_well_formedness`].
pub fn assert_well_formed<P: WellFormednessProvider + ?Sized>(function: &Function, provider: &P) {
    let errors = check_well_formedness(function, provider);
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

struct FunctionChecker<'a, P: WellFormednessProvider + ?Sized> {
    func: &'a Function,
    provider: &'a P,
    errors: &'a mut Vec<WellFormednessError>,
}

#[checks(registry = IMPLEMENTED_CHECKS)]
impl<'a, P: WellFormednessProvider + ?Sized> FunctionChecker<'a, P> {
    /// Runs every check and records each violation.
    #[checks(F1, F2, F7)]
    #[complexity(n_log_n in "the size of the function" because "every operand is checked against the sorted GC layouts")]
    fn run(&mut self) {
        let code = self.func.code.ops();
        if code.is_empty() {
            self.err(None, "code must be non-empty");
        }
        // F2. The dispatch loop falls through to `pc + 1` after any op that does
        // not set `pc` itself, so the last op must leave the function or jump.
        // Calls do not qualify: returning to `call_pc + 1` would run off the
        // end.
        if let Some(last) = code.last() {
            if !is_terminator(last) {
                self.err(
                    Some(code.len() - 1),
                    "last op must be a terminator (Return, Abort, AbortMsg, or Jump)",
                );
            }
        }

        self.check_frame_geometry();
        self.check_param_and_return_slots();
        self.check_gc_layouts();

        // F7. Origins: either absent (hand-built functions) or one per micro-op;
        // a partial table would attribute errors to wrong bytecode offsets.
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

        // O1–O6 through the operand schema, then the op-specific checks.
        for (pc, instr) in code.iter().enumerate() {
            instr.for_each_frame_operand(&mut |off, kind| {
                self.check_frame_operand(Some(pc), off, kind)
            });
            self.check_instruction(pc, instr);
        }
    }

    // -----------------------------------------------------------------------
    // Function-level checks
    // -----------------------------------------------------------------------

    #[checks(F3-F6)]
    #[complexity(constant)]
    fn check_frame_geometry(&mut self) {
        let func = self.func;
        if func.frame_size() > func.extended_frame_size {
            fail!(self, None, "extended_frame_size ({}) must be >= frame_size() (param_and_local_sizes_sum {} + FRAME_METADATA_SIZE {} = {})", func.extended_frame_size, func.param_and_local_sizes_sum, FRAME_METADATA_SIZE, func.frame_size());
        }
        if func.param_region_size > func.param_and_local_sizes_sum {
            fail!(
                self,
                None,
                "param_region_size ({}) must be <= param_and_local_sizes_sum ({})",
                func.param_region_size,
                func.param_and_local_sizes_sum
            );
        }
        // F5, F6. Frame metadata is written at `fp + param_and_local_sizes_sum` with
        // aligned 8-byte stores, and the callee fp `fp + frame_size()` must
        // be `MAX_ALIGN`-aligned for the callee's own slot accesses.
        if !func.param_and_local_sizes_sum.is_multiple_of(MAX_ALIGN) {
            fail!(
                self,
                None,
                "param_and_local_sizes_sum ({}) must be {}-byte aligned",
                func.param_and_local_sizes_sum,
                MAX_ALIGN
            );
        }
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

    #[checks(P1, R1)]
    #[complexity(linear in "the number of parameter and return slots")]
    fn check_param_and_return_slots(&mut self) {
        let func = self.func;
        // P1–P4, R1–R4. `CallClosure` and `CallBuilder` write `size` bytes at
        // `callee_fp + offset` for each parameter slot, and `call_unchecked`
        // zeroes `[param_region_size, extended_frame_size)` afterwards, so a
        // slot outside the parameter region is either overwritten or out of
        // frame.
        if func.param_slots.len() != func.param_tys.len() {
            fail!(
                self,
                None,
                "number of param slots ({}) must equal number of param types ({})",
                func.param_slots.len(),
                func.param_tys.len()
            );
        }
        self.check_slot_list(
            "param",
            &func.param_slots,
            "param_region_size",
            func.param_region_size,
        );

        let num_return_tys = view_type_list(func.return_tys).len();
        if func.return_slots.len() != num_return_tys {
            fail!(
                self,
                None,
                "number of return slots ({}) must equal number of return types ({num_return_tys})",
                func.return_slots.len()
            );
        }
        self.check_slot_list(
            "return",
            &func.return_slots,
            "param_and_local_sizes_sum",
            func.param_and_local_sizes_sum,
        );
    }

    /// P2–P4, R2–R4. A parameter or return slot list: every slot
    /// well-formed and within `[0, region_end)`, slots ascending and disjoint.
    #[checks(P3, P4, R3, R4)]
    #[complexity(linear in "the number of slots")]
    fn check_slot_list(
        &mut self,
        kind: &str,
        slots: &[SizedSlot],
        region_name: &str,
        region_end: usize,
    ) {
        for (i, slot) in slots.iter().enumerate() {
            self.check_sized_slot(None, &format!("{kind} slot {i}"), slot);
            let end = slot_end(slot);
            if end > region_end {
                fail!(
                    self,
                    None,
                    "{kind} slot [{}, {end}) exceeds {region_name} ({region_end})",
                    slot.offset.0
                );
            }
        }
        for (i, w) in slots.windows(2).enumerate() {
            if (w[1].offset.0 as usize) < slot_end(&w[0]) {
                fail!(
                    self,
                    None,
                    "{kind} slots {i} and {} are not ascending and disjoint ([{}, {}) then {})",
                    i + 1,
                    w[0].offset.0,
                    slot_end(&w[0]),
                    w[1].offset.0
                );
            }
        }
    }

    /// Slot well-formedness P2, R2, L4, L9. A [`SizedSlot`] carries its own
    /// alignment, which the closure runtime
    /// feeds to `align_up` (undefined for zero or non-power-of-two) and which
    /// must divide the offset for the slot to be where the layout says.
    #[checks(P2, R2, L4, L9)]
    #[complexity(constant)]
    fn check_sized_slot(&mut self, pc: Option<usize>, what: &str, slot: &SizedSlot) {
        if slot.size == 0 {
            fail!(self, pc, "{what}: size must be > 0");
        }
        if !is_valid_align(slot.align) {
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

    #[checks(G3, G4, G5, G7)]
    #[complexity(n_log_n in "the number of layout slots" because "each safe-point slot is one binary search into `base`")]
    fn check_gc_layouts(&mut self) {
        let code = self.func.code.ops();
        let base_offsets = &self.func.frame_layout.heap_ptr_offsets;
        let safe_points = self.func.safe_point_layouts.entries();
        self.check_pointer_offsets(None, base_offsets);

        // G3. The GC scans the base layout of every frame unconditionally, so a
        // slot beyond the parameter region must start out null rather than
        // holding whatever the previous frame left there.
        if !self.func.zero_frame {
            if let Some(off) = base_offsets
                .iter()
                .find(|off| off.0 as usize >= self.func.param_region_size)
            {
                fail!(self, None, "frame_layout names pointer slot {} beyond param_region_size ({}) but zero_frame is false", off.0, self.func.param_region_size);
            }
        }
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
            match code.get(co) {
                None => fail!(self, None, "safe_point_layouts: code_offset {co} out of bounds (code length {})", code.len()),
                // Top-frame-only contract: an entry sits at the PC of an
                // allocating op (see `MicroOp::is_allocating`).
                Some(op) if !op.is_allocating() => fail!(self, co, "safe_point_layouts: code_offset {co} is not at an allocating op; top-frame-only contract — see `SafePointEntry`"),
                Some(_) => {},
            }
            let sp_offsets = &entry.layout.heap_ptr_offsets;
            self.check_pointer_offsets(Some(co), sp_offsets);
            // G7. `base` is strictly increasing (G2), so each safe-point offset
            // is looked up by binary search: O(log |base|) per offset rather
            // than a rescan of `base` per safe point. If `base` is unsorted
            // that is already reported, and a duplicate missed here is moot.
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

    /// G1, G2 (and G6 for a safe point). Pointer offsets the GC reads with
    /// aligned `read_ptr`: each an in-frame, aligned pointer slot; the list
    /// strictly increasing.
    #[checks(G1, G2, G6)]
    #[complexity(linear in "the number of pointer offsets")]
    fn check_pointer_offsets(&mut self, pc: Option<usize>, offsets: &[FrameOffset]) {
        for &off in offsets {
            self.check_frame_operand(pc, off, OperandKind::Ptr);
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
    #[checks(I1-I5, Z3, B1, K1, K2)]
    #[complexity(n_log_n in "the op's operands" because "`VecUnpack` destinations are sorted")]
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
            | EnumBorrowVariantFieldByTag { .. }
            | EnumCheckVariant { .. } => {},

            // I1, I2. Unchecked u64 immediates: lowering uses the checked ops for
            // zero divisors and out-of-range shifts, so these are lowering bugs.
            DivU64Imm { imm, .. } | ModU64Imm { imm, .. } if imm == 0 => {
                self.err(pc, "division by zero (imm)");
            },
            DivU64Imm { .. } | ModU64Imm { .. } => {},
            ShlU64Imm { imm, .. } | ShrU64Imm { imm, .. } if imm >= 64 => {
                fail!(self, pc, "shift amount {imm} exceeds 63 (imm)");
            },
            ShlU64Imm { .. } | ShrU64Imm { .. } => {},

            // I3, I4, I5. Signedness the interpreter would otherwise reject at
            // runtime.
            IntBitAnd(ref op) | IntBitOr(ref op) | IntBitXor(ref op) if op.rhs.is_signed() => {
                self.err(pc, "bitwise on signed type");
            },
            IntBitAnd(_) | IntBitOr(_) | IntBitXor(_) => {},
            IntShl(op) | IntShr(op) if op.ty.is_signed() => {
                self.err(pc, "shift on signed type");
            },
            IntShl(_) | IntShr(_) => {},
            IntNegate(op) if !op.ty.is_signed() => {
                self.err(pc, "negate on unsigned type");
            },
            IntNegate(_) => {},
            Move { size, .. } => self.check_nonzero_size(pc, size),

            // B1. Forms a fat pointer to `local` without dereferencing it, so only
            // the base is checked: it must lie in the data region, not in the
            // metadata or callee region. The op carries no size, so the
            // borrowed extent is not bounds-checked here.
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

            // J1 (and O3 for the reference comparisons).
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
            JumpValueCmp(ref op) => self.check_jump(pc, op.target),
            JumpValueRefCmp(ref op) => {
                self.check_value_type(pc, op.ty);
                self.check_jump(pc, op.target);
            },
            ValueCmp(_) => {}, // `Value` operands are type-checked in `check_frame_operand`.
            ValueRefCmp(ref op) => self.check_value_type(pc, op.ty),
            CallDirect { ref ptr } => {
                // SAFETY: the function pointer lives in the global context,
                // which the caller's guard keeps alive during the check.
                let callee = unsafe { ptr.as_ref_unchecked() };
                self.check_direct_callee(pc, callee);
            },
            CallNative { ref abi, .. } => self.check_native_abi(pc, abi),

            // Z1, Z2. Heap and reference offset ops: nonzero width, `offset +
            // size` must not wrap.
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
            // Z1, Z2. Any tag may be selected at runtime, so every present offset
            // must keep `offset + size` within `u32`.
            EnumReadVariantFieldByTag {
                ref offsets, size, ..
            }
            | EnumWriteVariantFieldByTag {
                ref offsets, size, ..
            } => {
                self.check_nonzero_size(pc, size);
                for &offset in offsets.iter().flatten() {
                    self.check_offset_size(pc, offset, size);
                }
            },
            // Z3. The interpreter adds `base + off` in `u32`; the schema reports
            // the saturated slot, this reports the overflow itself.
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

            // Z1, K3. Vectors.
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
            // Z1, D1. The element copies are independent `copy_nonoverlapping`s.
            VecUnpack(ref op) => {
                self.check_nonzero_size(pc, op.elem_size);
                self.check_disjoint_destinations(pc, "VecUnpack", &op.dsts, op.elem_size);
            },

            // K1, K2. Allocation descriptors.
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
            PackClosure(ref op) => self.check_pack_closure(pc, op),
            CallClosure(ref op) => {
                for (i, slot) in op.provided_args.iter().enumerate() {
                    self.check_sized_slot(
                        Some(pc),
                        &format!("CallClosure: provided_args[{i}]"),
                        slot,
                    );
                }
            },
        }
    }

    #[checks(L1, L2, L3, L5-L8)]
    #[complexity(linear in "the number of captured values")]
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
        // A captured-data object is allocated iff at least one value is
        // captured. Pointer-free captures use the reserved `Trivial` slot;
        // pointer-bearing ones a `CapturedData` descriptor whose offsets must
        // lie within the values region so GC traces stay in bounds.
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
        // L4. Each captured slot's `align` drives the captured-data layout.
        for (i, slot) in op.captured.iter().enumerate() {
            self.check_sized_slot(Some(pc), &format!("PackClosure: captured[{i}]"), slot);
        }
        let captured_count = op.mask.count_ones() as usize;
        if op.captured.len() != captured_count {
            fail!(self, pc, "PackClosure: captured list length {} does not match mask captured count {captured_count}", op.captured.len());
        }
        match &op.func_ref {
            ClosureFuncRef::Resolved(func_ptr) => {
                // SAFETY: the function pointer lives in the global context,
                // which the caller's guard keeps alive during the check.
                let callee = unsafe { func_ptr.as_ref_unchecked() };
                let param_count = callee.param_slots.len();
                if param_count > u64::BITS as usize {
                    fail!(self, pc, "PackClosure: callee has {param_count} params, exceeds 64-bit mask capacity");
                }
                if param_count < u64::BITS as usize && op.mask >> param_count != 0 {
                    fail!(self, pc, "PackClosure: mask 0x{:x} sets bits beyond callee param count {param_count}", op.mask);
                }
                // L7. The runtime writes captured values with the slot's
                // `(size, align)` and reads them back at the callee parameter's
                // natural-aligned offset, so both must match. The captured list
                // is in mask-bit-set order through the param list. Bounded to
                // the 64 parameters a mask can address; more is already an
                // L6 error.
                let captured_params = callee
                    .param_slots
                    .iter()
                    .take(u64::BITS as usize)
                    .enumerate()
                    .filter(|(i, _)| (op.mask >> i) & 1 != 0);
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
        // L8. `values_size` must equal the natural-aligned captured layout size;
        // a smaller size would let the runtime's writes run out of bounds.
        // Skipped when a captured alignment is invalid (already reported),
        // since the layout is then undefined.
        if op.captured.iter().all(|slot| is_valid_align(slot.align)) {
            let expected =
                captured_values_size(op.captured.iter().map(|slot| (slot.size, slot.align)));
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

    /// O1–O6. Checks one frame operand per the [`OperandKind`] schema,
    /// resolving the provider-dependent kinds first.
    #[checks(O2, O4, O5)]
    #[complexity(log in "the number of GC pointer slots" because "O6 binary-searches the sorted layouts")]
    fn check_frame_operand(&mut self, pc: Option<usize>, offset: FrameOffset, kind: OperandKind) {
        let (width, align) = match kind {
            OperandKind::Value(ty) => {
                if let Some(pc) = pc {
                    self.check_value_type(pc, ty);
                }
                let Some(wa) = self.provider.size_and_align(ty) else {
                    return self.err(pc, "value comparison operand type has no known layout");
                };
                wa
            },
            OperandKind::Constant(idx) => {
                let Some(ty) = self.provider.constant_type(self.func.module_id, idx) else {
                    return fail!(
                        self,
                        pc,
                        "StoreImmVec: constant pool index {} out of range",
                        idx.0
                    );
                };
                let Some(wa) = self.provider.size_and_align(ty) else {
                    return fail!(
                        self,
                        pc,
                        "StoreImmVec: constant {} has no known layout",
                        idx.0
                    );
                };
                wa
            },
            kind => kind
                .width_and_align()
                .expect("every other kind is provider-independent"),
        };
        self.check_access(pc, offset, width, align);
        // O6. A scalar read or written where the GC expects a pointer is type
        // confusion: the GC would trace the integer as an object.
        if matches!(
            kind,
            OperandKind::Bool
                | OperandKind::Byte
                | OperandKind::U64
                | OperandKind::Int(_)
                | OperandKind::Address
        ) {
            self.check_not_pointer_slot(pc, offset, width);
        }
    }

    /// O6. `[offset, offset + width)` does not intersect any pointer slot in
    /// `frame_layout` or in the safe-point layout at `pc`. Both lists are
    /// strictly increasing (G2, G6), so each is one binary search.
    #[checks(O6)]
    #[complexity(log in "the number of pointer slots and safe points" because "one binary search into each sorted list")]
    fn check_not_pointer_slot(&mut self, pc: Option<usize>, offset: FrameOffset, width: u32) {
        let start = offset.0 as u64;
        let end = start + width as u64;
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
        let Some(pc) = pc else { return };
        let sps = self.func.safe_point_layouts.entries();
        let i = sps.partition_point(|e| (e.code_offset.0 as usize) < pc);
        if let Some(entry) = sps.get(i).filter(|e| e.code_offset.0 as usize == pc) {
            if let Some(b) = overlapping(&entry.layout.heap_ptr_offsets) {
                fail!(
                    self,
                    pc,
                    "access [{start}, {end}) aliases safe-point pointer slot {b}"
                );
            }
        }
    }

    /// `[offset, offset + size)` lies within the extended frame, does not
    /// overlap the metadata block, and `offset` is a multiple of `align`.
    /// Accesses into the callee arg/return region are permitted: that is how
    /// arguments and return values are passed.
    #[checks(O1)]
    #[complexity(constant)]
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

    /// O3. Structural comparison is defined on values, not references; a
    /// reference type is an unconditional runtime invariant violation.
    #[checks(O3)]
    #[complexity(constant)]
    fn check_value_type(&mut self, pc: usize, ty: InternedType) {
        if matches!(view_type(ty), Type::ImmutRef { .. } | Type::MutRef { .. }) {
            self.err(pc, "value comparison on a reference type");
        }
    }

    /// D1. Destinations of a multi-destination op are pairwise disjoint.
    #[checks(D1)]
    #[complexity(n_log_n in "the number of destinations" because "they are sorted to find overlaps")]
    fn check_disjoint_destinations(
        &mut self,
        pc: usize,
        op: &str,
        dsts: &[FrameOffset],
        width: u32,
    ) {
        let mut sorted: Vec<u64> = dsts.iter().map(|d| d.0 as u64).collect();
        sorted.sort_unstable();
        if let Some(w) = sorted.windows(2).find(|w| w[0] + width as u64 > w[1]) {
            fail!(
                self,
                pc,
                "{op}: destinations [{}, {}) and [{}, {}) overlap",
                w[0],
                w[0] + width as u64,
                w[1],
                w[1] + width as u64
            );
        }
    }

    // -----------------------------------------------------------------------
    // Calls
    // -----------------------------------------------------------------------

    /// C1, C2, C3. The native's slot region fits the caller's extended frame, and its
    /// pointer slots (read by the GC with aligned `read_ptr` at the native's
    /// fp) are aligned, inside the region, and inside an argument slot.
    #[checks(C1-C3)]
    #[complexity(n_log_n in "the ABI's pointer offsets and argument slots" because "each offset binary-searches the sorted argument slots")]
    fn check_native_abi(&mut self, pc: usize, abi: &NativeABI) {
        let base = self.func.frame_size();
        let total = abi.total_frame_size();
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
            // `off`: O(log |args|) per offset.
            let args = abi.args();
            let inside_arg = args
                .partition_point(|slot| slot.offset <= off.0)
                .checked_sub(1)
                .map(|i| &args[i])
                .is_some_and(|slot| off_end <= slot.offset as u64 + slot.size as u64);
            if !inside_arg {
                fail!(
                    self,
                    pc,
                    "native heap pointer offset {} is not inside an argument slot",
                    off.0
                );
            }
        }
        for (i, id) in abi.required_descriptors().iter().enumerate() {
            if self.provider.descriptor(*id).is_none() {
                fail!(self, pc, "native required descriptor {i} ({id}) is unknown");
            }
        }
    }

    /// C4. A direct callee's parameters are written into, and its return values
    /// read back from, this function's callee region
    /// `[frame_size(), extended_frame_size)`.
    #[checks(C4)]
    #[complexity(linear in "the callee's return slots")]
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
        for (i, slot) in callee.return_slots.iter().enumerate() {
            let end = slot_end(slot);
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

    /// K3. A vector allocation's descriptor is `Trivial`, or a `Vector` with a
    /// non-empty pointer-offset list and a matching `elem_size`: the GC
    /// strides the data region by the descriptor's `elem_size`, so a mismatch
    /// would trace past the allocation.
    #[checks(K3)]
    #[complexity(constant)]
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
    #[checks(J1)]
    #[complexity(constant)]
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

    #[checks(Z1)]
    #[complexity(constant)]
    fn check_nonzero_size(&mut self, pc: usize, size: u32) {
        if size == 0 {
            self.err(pc, "size must be > 0");
        }
    }

    /// Z2. `offset + size` fits in `u32`, so the window `[offset, offset + size)`
    /// into a heap object or referent cannot wrap.
    #[checks(Z2)]
    #[complexity(constant)]
    fn check_offset_size(&mut self, pc: usize, offset: u32, size: u32) {
        if offset.checked_add(size).is_none() {
            fail!(self, pc, "offset {offset} + size {size} overflows u32");
        }
    }
}

// ---------------------------------------------------------------------------
// Free helpers
// ---------------------------------------------------------------------------

/// Ops after which the dispatch loop does not fall through to `pc + 1`
/// within this frame: they leave the function or set `pc` to a target.
fn is_terminator(op: &MicroOp) -> bool {
    matches!(
        op,
        MicroOp::Return | MicroOp::Abort { .. } | MicroOp::AbortMsg { .. } | MicroOp::Jump { .. }
    )
}

/// A slot alignment the runtime's `align_up` accepts.
fn is_valid_align(align: u32) -> bool {
    align != 0 && align.is_power_of_two() && align as usize <= MAX_ALIGN
}

/// One past the last byte of `slot`. Two `u32`s widened to `usize` cannot
/// overflow on a 64-bit target.
fn slot_end(slot: &SizedSlot) -> usize {
    slot.offset.0 as usize + slot.size as usize
}
