// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Static well-formedness checker for lowered [`Function`] bodies.
//!
//! The loader runs [`verify_function`] on every lowered function before it is
//! cached, so a rejected lowering never reaches the interpreter. Test harnesses
//! that build `Function`s by hand call [`assert_verified`] instead.
//!
//! The checks are specified in `docs/micro_op_verifier.md`. Each check has a
//! stable identifier there (`F1`, `P3`, `O1`, ...), and the implementation
//! cites the identifier at the point where it is evaluated. Frame operands and
//! the kind of access the interpreter performs on each one come from the
//! schema in `instruction::operands`; everything else is specific to an op or
//! to the function's metadata. All checks are evaluated and every violation is
//! reported; no check depends on another having passed.
//!
//! TODO(cleanup): rename to a well-formedness checker to avoid ambiguity with
//! the Move bytecode verifier.

use crate::{
    align::MAX_ALIGN,
    captured_values_size,
    interner::InternedModuleId,
    native::NativeABI,
    types::{view_type, view_type_list, InternedType, Type},
    ClosureFuncRef, CodeOffset, ConstantPoolIndex, DescriptorId, DescriptorProvider, FrameOffset,
    Function, LayoutProvider, MicroOp, ObjectDescriptorInner, OperandKind, PackClosureOp,
    SizedSlot, CLOSURE_DESCRIPTOR_ID, FRAME_METADATA_SIZE,
};
use std::{cmp::Ordering, fmt};

/// Width and alignment of a heap-pointer slot, as the GC and the call
/// protocol read it.
const PTR_WIDTH: u32 = 8;
const PTR_ALIGN: u32 = 8;

// ---------------------------------------------------------------------------
// Providers
// ---------------------------------------------------------------------------

/// Constant-pool view of the modules whose functions are being verified.
pub trait ConstantPoolProvider {
    /// Interned type of constant `idx` in `module_id`'s pool, or `None` if
    /// the module is unknown or `idx` is out of range.
    fn constant_type(
        &self,
        module_id: InternedModuleId,
        idx: ConstantPoolIndex,
    ) -> Option<InternedType>;
}

/// Everything the verifier needs to resolve a function's operands.
pub trait VerifierProvider: DescriptorProvider + LayoutProvider + ConstantPoolProvider {}

impl<P: DescriptorProvider + LayoutProvider + ConstantPoolProvider + ?Sized> VerifierProvider
    for P
{
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct VerificationError {
    pub func_name: String,
    pub pc: Option<usize>,
    pub message: String,
}

impl fmt::Display for VerificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.pc {
            Some(pc) => write!(f, "'{}', pc {}: {}", self.func_name, pc, self.message),
            None => write!(f, "'{}': {}", self.func_name, self.message),
        }
    }
}

/// Records a verification error at `pc` (`Option<usize>`) with a formatted
/// message.
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
pub fn verify_function<P: VerifierProvider + ?Sized>(
    func: &Function,
    provider: &P,
) -> Vec<VerificationError> {
    let mut errors = Vec::new();
    FunctionVerifier {
        func,
        provider,
        errors: &mut errors,
    }
    .verify();
    errors
}

/// Panics with the verifier's findings unless `function` verifies cleanly.
pub fn assert_verified<P: VerifierProvider + ?Sized>(function: &Function, provider: &P) {
    let errors = verify_function(function, provider);
    assert!(
        errors.is_empty(),
        "verification failed:\n{}",
        errors
            .iter()
            .map(|e| format!("  {}", e))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

// ---------------------------------------------------------------------------
// Per-function verifier
// ---------------------------------------------------------------------------

struct FunctionVerifier<'a, P: VerifierProvider + ?Sized> {
    func: &'a Function,
    provider: &'a P,
    errors: &'a mut Vec<VerificationError>,
}

impl<'a, P: VerifierProvider + ?Sized> FunctionVerifier<'a, P> {
    fn verify(&mut self) {
        let code = self.func.code.ops();

        // F1.
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

        self.verify_frame_geometry();
        self.verify_param_and_return_slots();
        self.verify_gc_layouts();

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

        // O1–O5 through the operand schema, then the op-specific checks.
        for (pc, instr) in code.iter().enumerate() {
            instr.frame_operands(&mut |off, kind| self.check(Some(pc), off, kind));
            self.verify_instruction(pc, instr);
        }
    }

    // -----------------------------------------------------------------------
    // Function-level checks
    // -----------------------------------------------------------------------

    fn verify_frame_geometry(&mut self) {
        let func = self.func;
        // F3.
        if func.frame_size() > func.extended_frame_size {
            fail!(
                self,
                None,
                "extended_frame_size ({}) must be >= frame_size() (param_and_local_sizes_sum {} + FRAME_METADATA_SIZE {} = {})",
                func.extended_frame_size,
                func.param_and_local_sizes_sum,
                FRAME_METADATA_SIZE,
                func.frame_size()
            );
        }
        // F4.
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

    fn verify_param_and_return_slots(&mut self) {
        let func = self.func;
        // P1–P4 and R1–R4. `CallClosure` and `CallBuilder` write `size` bytes at
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
                "number of return slots ({}) must equal number of return types ({})",
                func.return_slots.len(),
                num_return_tys
            );
        }
        self.check_slot_list(
            "return",
            &func.return_slots,
            "param_and_local_sizes_sum",
            func.param_and_local_sizes_sum,
        );
    }

    /// P2–P4 / R2–R4. A parameter or return slot list: every slot
    /// well-formed and within `[0, region_end)`, slots ascending and disjoint.
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
                    "{kind} slot [{}, {}) exceeds {region_name} ({})",
                    slot.offset.0,
                    end,
                    region_end
                );
            }
        }
        for (i, w) in slots.windows(2).enumerate() {
            if (w[1].offset.0 as usize) < slot_end(&w[0]) {
                fail!(
                    self,
                    None,
                    "{kind} slots {} and {} are not ascending and disjoint ([{}, {}) then {})",
                    i,
                    i + 1,
                    w[0].offset.0,
                    slot_end(&w[0]),
                    w[1].offset.0
                );
            }
        }
    }

    /// Slot well-formedness (P2, R2, L4, L9). A [`SizedSlot`] carries its own
    /// alignment, which the closure runtime
    /// feeds to `align_up` (undefined for zero or non-power-of-two) and which
    /// must divide the offset for the slot to be where the layout says.
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

    fn verify_gc_layouts(&mut self) {
        let code = self.func.code.ops();
        let base_offsets = &self.func.frame_layout.heap_ptr_offsets;
        let safe_points = self.func.safe_point_layouts.entries();

        // G1, G2.
        self.check_pointer_offsets(None, base_offsets);

        // G3. The GC scans the base layout of every frame unconditionally, so a
        // slot beyond the parameter region must start out null rather than
        // holding whatever the previous frame left there.
        if !self.func.zero_frame {
            if let Some(off) = base_offsets
                .iter()
                .find(|off| off.0 as usize >= self.func.param_region_size)
            {
                fail!(
                    self,
                    None,
                    "frame_layout names pointer slot {} beyond param_region_size ({}) but zero_frame is false",
                    off.0,
                    self.func.param_region_size
                );
            }
        }

        // G4.
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
            // G5.
            match code.get(co) {
                None => fail!(
                    self,
                    None,
                    "safe_point_layouts: code_offset {} out of bounds (code length {})",
                    co,
                    code.len()
                ),
                // Top-frame-only contract: an entry sits at the PC of an
                // allocating op (see `MicroOp::is_allocating`).
                Some(op) if !op.is_allocating() => fail!(
                    self,
                    Some(co),
                    "safe_point_layouts: code_offset {} is not at an allocating op; \
                     top-frame-only contract — see `SafePointEntry`",
                    co
                ),
                Some(_) => {},
            }
            let sp_offsets = &entry.layout.heap_ptr_offsets;
            // G6.
            self.check_pointer_offsets(Some(co), sp_offsets);
            // G7. Both lists are strictly increasing (G2, G6), so a merge finds
            // every duplicate in linear time. If either list is unsorted that
            // is already reported, and a duplicate missed here is moot.
            let (mut i, mut j) = (0, 0);
            while let (Some(sp), Some(base)) = (sp_offsets.get(i), base_offsets.get(j)) {
                match sp.0.cmp(&base.0) {
                    Ordering::Less => i += 1,
                    Ordering::Greater => j += 1,
                    Ordering::Equal => {
                        fail!(
                            self,
                            Some(co),
                            "safe_point_layouts: offset {} duplicates frame_layout",
                            sp.0
                        );
                        i += 1;
                        j += 1;
                    },
                }
            }
        }
    }

    /// G1, G2 (and G6 for a safe point). Pointer offsets the GC reads with
    /// aligned `read_ptr`: each an in-frame, aligned pointer slot; the list
    /// strictly increasing.
    fn check_pointer_offsets(&mut self, pc: Option<usize>, offsets: &[FrameOffset]) {
        for &off in offsets {
            self.check(pc, off, OperandKind::Ptr);
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

    fn verify_instruction(&mut self, pc: usize, instr: &MicroOp) {
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
                self.err(Some(pc), "division by zero (imm)");
            },
            DivU64Imm { .. } | ModU64Imm { .. } => {},
            ShlU64Imm { imm, .. } | ShrU64Imm { imm, .. } if imm >= 64 => {
                fail!(self, Some(pc), "shift amount {} exceeds 63 (imm)", imm);
            },
            ShlU64Imm { .. } | ShrU64Imm { .. } => {},

            // I3, I4, I5. Signedness the interpreter would otherwise reject at
            // runtime.
            IntBitAnd(ref op) | IntBitOr(ref op) | IntBitXor(ref op) if op.rhs.is_signed() => {
                self.err(Some(pc), "bitwise on signed type");
            },
            IntBitAnd(_) | IntBitOr(_) | IntBitXor(_) => {},
            IntShl(op) | IntShr(op) if op.ty.is_signed() => {
                self.err(Some(pc), "shift on signed type");
            },
            IntShl(_) | IntShr(_) => {},
            IntNegate(op) if !op.ty.is_signed() => {
                self.err(Some(pc), "negate on unsigned type");
            },
            IntNegate(_) => {},

            // Z1.
            Move { size, .. } => self.check_nonzero_size(pc, size),

            // B1. Forms a fat pointer to `local` without dereferencing it, so only
            // the base is checked: it must lie in the data region, not in the
            // metadata or callee region. The op carries no size, so the
            // borrowed extent is not bounds-checked here.
            SlotBorrow { local, .. } if local.0 as usize >= self.func.param_and_local_sizes_sum => {
                fail!(
                    self,
                    Some(pc),
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
            ValueCmp(_) => {}, // `Value` operands are type-checked in `check`.
            ValueRefCmp(ref op) => self.check_value_type(pc, op.ty),

            // C4.
            CallDirect { ref ptr } => {
                // SAFETY: the function pointer lives in the global context,
                // which the caller's guard keeps alive during verification.
                let callee = unsafe { ptr.as_ref_unchecked() };
                self.check_direct_callee(pc, callee);
            },
            // C1, C2, C3.
            CallNative { ref abi, .. } => self.check_native_abi(pc, abi),

            // Z1, Z2. Heap and reference offset ops: nonzero width, `offset +
            // size` must not wrap.
            HeapMoveFrom8 { offset, .. }
            | HeapMoveTo8 { offset, .. }
            | HeapMoveToImm8 { offset, .. } => {
                self.check_offset_size(pc, offset, PTR_WIDTH);
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
                        Some(pc),
                        "DeepCopyHeapPtrs: base {} + offset {} overflows u32",
                        base.0,
                        off
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
                            Some(pc),
                            "HeapNew: descriptor_id {} is not a Struct or Enum",
                            descriptor_id
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
                        fail!(
                            self,
                            Some(pc),
                            "EnumNew: tag {} out of range (descriptor {} has {} variants)",
                            variant,
                            descriptor_id,
                            variant_count
                        );
                    }
                },
                Some(_) => fail!(
                    self,
                    Some(pc),
                    "EnumNew: descriptor_id {} is not an Enum",
                    descriptor_id
                ),
                None => {},
            },

            // L1–L8.
            PackClosure(ref op) => self.verify_pack_closure(pc, op),
            // L9.
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

    fn verify_pack_closure(&mut self, pc: usize, op: &PackClosureOp) {
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
        // L1, L2, L3.
        match (op.captured_data_descriptor_id, op.captured.is_empty()) {
            (None, true) => {},
            (Some(id), false) => match self.descriptor_or_report(pc, "PackClosure", id) {
                Some(ObjectDescriptorInner::Trivial) | None => {},
                Some(ObjectDescriptorInner::CapturedData { pointer_offsets }) => {
                    for &off in pointer_offsets
                        .iter()
                        .filter(|&&off| off as u64 + PTR_WIDTH as u64 > op.values_size as u64)
                    {
                        fail!(
                            self,
                            Some(pc),
                            "PackClosure: captured_data pointer offset {} out of bounds of values_size {}",
                            off,
                            op.values_size
                        );
                    }
                },
                Some(_) => fail!(
                    self,
                    Some(pc),
                    "PackClosure: descriptor_id {} is not a Trivial or CapturedData",
                    id
                ),
            },
            (Some(id), true) => fail!(
                self,
                Some(pc),
                "PackClosure: captured_data_descriptor_id {} provided but no captures",
                id
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
        // L5.
        let captured_count = op.mask.count_ones() as usize;
        if op.captured.len() != captured_count {
            fail!(
                self,
                Some(pc),
                "PackClosure: captured list length {} does not match mask captured count {}",
                op.captured.len(),
                captured_count
            );
        }
        match &op.func_ref {
            ClosureFuncRef::Resolved(func_ptr) => {
                // SAFETY: the function pointer lives in the global context,
                // which the caller's guard keeps alive during verification.
                let callee = unsafe { func_ptr.as_ref_unchecked() };
                let param_count = callee.param_slots.len();
                // L6.
                if param_count > 64 {
                    fail!(
                        self,
                        Some(pc),
                        "PackClosure: callee has {} params, exceeds 64-bit mask capacity",
                        param_count
                    );
                }
                if param_count < 64 && op.mask >> param_count != 0 {
                    fail!(
                        self,
                        Some(pc),
                        "PackClosure: mask 0x{:x} sets bits beyond callee param count {}",
                        op.mask,
                        param_count
                    );
                }
                // L7. The runtime writes captured values with the slot's
                // `(size, align)` and reads them back at the callee parameter's
                // natural-aligned offset, so both must match. The captured list
                // is in mask-bit-set order through the param list.
                let captured_params = callee
                    .param_slots
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| (op.mask >> i) & 1 != 0);
                for (k, ((i, param_slot), slot)) in captured_params.zip(&op.captured).enumerate() {
                    if slot.size != param_slot.size {
                        fail!(
                            self,
                            Some(pc),
                            "PackClosure: captured[{}].size {} != callee param_slots[{}].size {}",
                            k,
                            slot.size,
                            i,
                            param_slot.size
                        );
                    }
                    if slot.align != param_slot.align {
                        fail!(
                            self,
                            Some(pc),
                            "PackClosure: captured[{}].align {} != callee param_slots[{}].align {}",
                            k,
                            slot.align,
                            i,
                            param_slot.align
                        );
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
                    Some(pc),
                    "PackClosure: values_size {} != captured layout size {}",
                    op.values_size,
                    expected
                );
            }
        }
    }

    // -----------------------------------------------------------------------
    // Frame access
    // -----------------------------------------------------------------------

    fn err(&mut self, pc: Option<usize>, msg: impl Into<String>) {
        self.errors.push(VerificationError {
            func_name: self.func.name().to_string(),
            pc,
            message: msg.into(),
        });
    }

    /// O1, with O2–O5 for the provider-dependent kinds. Checks one frame
    /// operand per the [`OperandKind`] schema.
    fn check(&mut self, pc: Option<usize>, offset: FrameOffset, kind: OperandKind) {
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
    }

    /// `[offset, offset + size)` lies within the extended frame, does not
    /// overlap the metadata block, and `offset` is a multiple of `align`.
    /// Accesses into the callee arg/return region are permitted: that is how
    /// arguments and return values are passed.
    fn check_access(&mut self, pc: Option<usize>, offset: FrameOffset, size: u32, align: u32) {
        let offset = offset.0 as usize;
        let Some(end) = offset.checked_add(size as usize) else {
            return fail!(self, pc, "access at offset {} overflows", offset);
        };
        if end > self.func.extended_frame_size {
            return fail!(
                self,
                pc,
                "access [{}, {}) exceeds extended_frame_size {}",
                offset,
                end,
                self.func.extended_frame_size
            );
        }
        let meta_start = self.func.param_and_local_sizes_sum;
        let meta_end = meta_start + FRAME_METADATA_SIZE;
        if offset < meta_end && meta_start < end {
            fail!(
                self,
                pc,
                "access [{}, {}) overlaps metadata [{}, {})",
                offset,
                end,
                meta_start,
                meta_end
            );
        }
        if !offset.is_multiple_of(align as usize) {
            fail!(
                self,
                pc,
                "access [{}, {}) is not {}-byte aligned",
                offset,
                end,
                align
            );
        }
    }

    /// O3. Structural comparison is defined on values, not references; a
    /// reference type is an unconditional runtime invariant violation.
    fn check_value_type(&mut self, pc: usize, ty: InternedType) {
        if matches!(view_type(ty), Type::ImmutRef { .. } | Type::MutRef { .. }) {
            self.err(Some(pc), "value comparison on a reference type");
        }
    }

    /// D1. Destinations of a multi-destination op are pairwise disjoint.
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
                Some(pc),
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
    fn check_native_abi(&mut self, pc: usize, abi: &NativeABI) {
        let base = self.func.frame_size();
        let total = abi.total_frame_size();
        match base.checked_add(total as usize) {
            None => self.err(Some(pc), "native slot region overflows usize"),
            Some(end) if end > self.func.extended_frame_size => fail!(
                self,
                Some(pc),
                "native slot region [{}, {}) exceeds extended_frame_size {}",
                base,
                end,
                self.func.extended_frame_size
            ),
            Some(_) => {},
        }
        for &off in abi.heap_ptr_offsets() {
            let off_end = off.0 as u64 + PTR_WIDTH as u64;
            if off_end > total as u64 {
                fail!(
                    self,
                    Some(pc),
                    "native heap pointer offset {} exceeds the slot region ({})",
                    off.0,
                    total
                );
            }
            if !off.0.is_multiple_of(PTR_ALIGN) {
                fail!(
                    self,
                    Some(pc),
                    "native heap pointer offset {} is not 8-byte aligned",
                    off.0
                );
            }
            let inside_arg = abi.args().iter().any(|slot| {
                slot.offset as u64 <= off.0 as u64
                    && off_end <= slot.offset as u64 + slot.size as u64
            });
            if !inside_arg {
                fail!(
                    self,
                    Some(pc),
                    "native heap pointer offset {} is not inside an argument slot",
                    off.0
                );
            }
        }
        for (i, id) in abi.required_descriptors().iter().enumerate() {
            if self.provider.descriptor(*id).is_none() {
                fail!(
                    self,
                    Some(pc),
                    "native required descriptor {i} ({id}) is unknown"
                );
            }
        }
    }

    /// C4. A direct callee's parameters are written into, and its return values
    /// read back from, this function's callee region
    /// `[frame_size(), extended_frame_size)`.
    fn check_direct_callee(&mut self, pc: usize, callee: &Function) {
        let region = self
            .func
            .extended_frame_size
            .saturating_sub(self.func.frame_size());
        if callee.param_region_size > region {
            fail!(
                self,
                Some(pc),
                "CallDirect: callee param_region_size {} exceeds the callee region ({})",
                callee.param_region_size,
                region
            );
        }
        for (i, slot) in callee.return_slots.iter().enumerate() {
            let end = slot_end(slot);
            if end > region {
                fail!(
                    self,
                    Some(pc),
                    "CallDirect: callee return slot {i} [{}, {}) exceeds the callee region ({})",
                    slot.offset.0,
                    end,
                    region
                );
            }
        }
    }

    // -----------------------------------------------------------------------
    // Descriptors and small checks
    // -----------------------------------------------------------------------

    /// Resolves `descriptor_id`, reporting an unknown id on behalf of `op`.
    /// The result borrows the provider, not the verifier, so callers can keep
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
            fail!(
                self,
                Some(pc),
                "{}: unknown descriptor_id {}",
                op,
                descriptor_id
            );
        }
        inner
    }

    /// K3. A vector allocation's descriptor is `Trivial`, or a `Vector` with a
    /// non-empty pointer-offset list and a matching `elem_size`: the GC
    /// strides the data region by the descriptor's `elem_size`, so a mismatch
    /// would trace past the allocation.
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
                    fail!(
                        self,
                        Some(pc),
                        "{}: elem_size {} does not match Vector descriptor_id {} elem_size {}",
                        op,
                        elem_size,
                        descriptor_id,
                        descriptor_elem_size
                    );
                }
            },
            Some(_) => fail!(
                self,
                Some(pc),
                "{}: descriptor_id {} is not a non-empty Vector or Trivial",
                op,
                descriptor_id
            ),
        }
    }

    /// J1.
    // TODO(metering): validate branch gas fields are populated.
    fn check_jump(&mut self, pc: usize, target: CodeOffset) {
        let code_len = self.func.code.ops().len();
        if (target.0 as usize) >= code_len {
            fail!(
                self,
                Some(pc),
                "jump target {} out of bounds (code length {})",
                target.0,
                code_len
            );
        }
    }

    /// Z1.
    fn check_nonzero_size(&mut self, pc: usize, size: u32) {
        if size == 0 {
            self.err(Some(pc), "size must be > 0");
        }
    }

    /// Z2. `offset + size` fits in `u32`, so the window `[offset, offset + size)`
    /// into a heap object or referent cannot wrap.
    fn check_offset_size(&mut self, pc: usize, offset: u32, size: u32) {
        if offset.checked_add(size).is_none() {
            fail!(
                self,
                Some(pc),
                "offset {} + size {} overflows u32",
                offset,
                size
            );
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
