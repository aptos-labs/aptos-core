// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Static well-formedness checker for lowered [`Function`] bodies.
//!
//! The loader runs [`verify_function`] on every lowered function before it is
//! cached, so a rejected lowering never reaches the interpreter. Test harnesses
//! that build `Function`s by hand call [`assert_verified`] instead.
//!
//! # What is checked
//!
//! Everything below is derivable from the `Function` and its micro-ops alone,
//! given the descriptor, layout, and constant-pool providers:
//!
//! - **Frame geometry**: `extended_frame_size >= frame_size()`; the data
//!   region and the callee frame pointer are `MAX_ALIGN`-aligned;
//!   `param_region_size` lies within the data region and covers every
//!   parameter slot.
//! - **Parameter and return slots**: one per type, nonzero size, power-of-two
//!   alignment up to `MAX_ALIGN`, aligned offsets, strictly ascending and
//!   disjoint, parameters inside the parameter region and returns inside the
//!   data region.
//! - **Frame accesses**: every frame operand of every micro-op lies within the
//!   extended frame, does not overlap the frame metadata block, and is aligned
//!   as the interpreter's access to it requires (8 for pointer, `u64`, and fat
//!   pointer slots; the natural width for 2/4/8-byte integer slots; none for
//!   byte copies, booleans, immediates, addresses, and 16/32-byte integers,
//!   which the interpreter reads unaligned).
//! - **GC layouts**: `frame_layout` and every safe-point layout list aligned,
//!   in-bounds pointer slots, strictly sorted; safe points sit at allocating
//!   ops and do not duplicate the base layout; `zero_frame` is set whenever
//!   the base layout names a slot beyond the parameter region.
//! - **Control flow**: jump targets in range; the last op is a terminator.
//! - **Sizes and immediates**: variable-width copies are nonzero; `offset +
//!   size` fits in `u32` for heap and reference offset ops; unchecked `u64`
//!   immediates (divisor, shift amount) are in range; signedness restrictions
//!   the interpreter would otherwise report at runtime.
//! - **Descriptors**: allocation ops name a descriptor of the right kind, with
//!   matching element stride and in-range enum tag; closure captured-data
//!   descriptors keep their pointer offsets inside the values region.
//! - **Constants**: `StoreImmVec` names an existing constant and its
//!   destination is sized and aligned for the constant's type.
//! - **Calls**: native slot regions and their pointer offsets fit; direct
//!   callees' parameter regions and return slots fit the caller's callee
//!   region; multi-destination ops have disjoint destinations.
//!
//! # Out of scope
//!
//! Anything that depends on runtime data or on dataflow: the extent borrowed
//! by `SlotBorrow`; heap offsets against the pointee's size for ops that carry
//! only a pointer; `elem_size` against a vector's real stride; enum offset
//! tables against the variant count; whether a listed pointer slot actually
//! holds a pointer at a given PC; write-before-read of slots. Descriptors
//! themselves are not re-verified; their soundness is enforced by
//! [`crate::ObjectDescriptor`]'s constructors at publish time.
//!
//! TODO(cleanup): rename to a well-formedness checker to avoid ambiguity with
//! the Move bytecode verifier.

use crate::{
    align::MAX_ALIGN,
    captured_values_size,
    interner::InternedModuleId,
    native::NativeABI,
    types::{view_type, view_type_list, InternedType, Type},
    CallClosureOp, ClosureFuncRef, CodeOffset, ConstantPoolIndex, DescriptorId, DescriptorProvider,
    FrameOffset, Function, IntBinaryOp, LayoutProvider, MicroOp, ObjectDescriptorInner,
    PackClosureOp, ShiftOperand, SizedSlot, CLOSURE_DESCRIPTOR_ID, FRAME_METADATA_SIZE,
};
use std::fmt;

// ---------------------------------------------------------------------------
// Access widths and alignments
// ---------------------------------------------------------------------------

/// A boolean slot.
const BOOL_WIDTH: u32 = 1;
/// A heap pointer or `u64` slot.
const PTR_WIDTH: u32 = 8;
/// A reference: `(base: *mut u8, byte_offset: u64)`.
const FAT_PTR_WIDTH: u32 = 16;
/// An inline `address` value.
const ADDRESS_WIDTH: u32 = 32;

/// Alignment required by the interpreter's aligned 8-byte loads and stores
/// (`read_u64`, `read_ptr`, `read_fat_ptr`, and their writers).
const PTR_ALIGN: u32 = 8;
/// No alignment requirement: the access is a byte copy, a single byte, or an
/// explicitly unaligned load/store.
const NO_ALIGN: u32 = 1;

/// Alignment the interpreter's `read_int<T>` / `write_int<T>` require for an
/// integer slot of `width` bytes: natural for 1/2/4/8, none for 16/32 (those
/// are read unaligned because their Rust alignment exceeds `MAX_ALIGN`).
fn int_align(width: u32) -> u32 {
    if width as usize <= MAX_ALIGN {
        width
    } else {
        NO_ALIGN
    }
}

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
    let mut fv = FunctionVerifier {
        func,
        provider,
        errors: &mut errors,
    };
    fv.verify();
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
// Per-function verifier — holds shared state so helpers don't need many args
// ---------------------------------------------------------------------------

struct FunctionVerifier<'a, P: VerifierProvider + ?Sized> {
    func: &'a Function,
    provider: &'a P,
    errors: &'a mut Vec<VerificationError>,
}

impl<P: VerifierProvider + ?Sized> FunctionVerifier<'_, P> {
    fn verify(&mut self) {
        let code = self.func.code.ops();

        // --- Function-level sanity ---
        // Code must be non-empty (at minimum a Return).
        if code.is_empty() {
            self.err(None, "code must be non-empty");
        }
        // The dispatch loop falls through to `pc + 1` after any op that does
        // not set `pc` itself, so the last op must leave the function or jump.
        // (Calls do not qualify: returning to `call_pc + 1` would run off the
        // end.)
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

        // Origins: bytecode provenance of each micro-op.
        // Either absent (hand-built functions with no bytecode ancestry) or
        // one entry per micro-op; a partial table would attribute errors to
        // wrong bytecode offsets.
        let origins = self.func.code.origins();
        if !origins.is_empty() && origins.len() != code.len() {
            self.err(
                None,
                format!(
                    "number of origins ({}) must be zero or equal number of micro-ops ({})",
                    origins.len(),
                    code.len()
                ),
            );
        }

        // --- Per-instruction checks ---
        // Frame access bounds, jump targets, descriptor validity, etc.
        for (pc, instr) in code.iter().enumerate() {
            self.verify_instruction(pc, instr);
        }
    }

    // -----------------------------------------------------------------------
    // Function-level checks
    // -----------------------------------------------------------------------

    fn verify_frame_geometry(&mut self) {
        let func = self.func;
        // extended_frame_size must be large enough to hold locals + metadata.
        if func.frame_size() > func.extended_frame_size {
            self.err(
                None,
                format!(
                    "extended_frame_size ({}) must be >= frame_size() (param_and_local_sizes_sum {} + FRAME_METADATA_SIZE {} = {})",
                    func.extended_frame_size,
                    func.param_and_local_sizes_sum,
                    FRAME_METADATA_SIZE,
                    func.frame_size()
                ),
            );
        }
        // param_region_size must fit within the data region.
        if func.param_region_size > func.param_and_local_sizes_sum {
            self.err(
                None,
                format!(
                    "param_region_size ({}) must be <= param_and_local_sizes_sum ({})",
                    func.param_region_size, func.param_and_local_sizes_sum
                ),
            );
        }
        // The runtime writes frame metadata (saved pc/fp/func_ptr) at
        // `fp + param_and_local_sizes_sum` with aligned 8-byte stores, and the
        // callee frame pointer `fp + frame_size()` must be `MAX_ALIGN`-aligned
        // for the callee's own slot accesses.
        if !func.param_and_local_sizes_sum.is_multiple_of(MAX_ALIGN) {
            self.err(
                None,
                format!(
                    "param_and_local_sizes_sum ({}) must be {}-byte aligned",
                    func.param_and_local_sizes_sum, MAX_ALIGN
                ),
            );
        }
        if !func.frame_size().is_multiple_of(MAX_ALIGN) {
            self.err(
                None,
                format!(
                    "frame_size() ({}) must be {}-byte aligned so the callee fp is aligned",
                    func.frame_size(),
                    MAX_ALIGN
                ),
            );
        }
    }

    fn verify_param_and_return_slots(&mut self) {
        let func = self.func;

        // Parameters: one slot per type, all inside the parameter region.
        // `CallClosure` and `CallBuilder` write `size` bytes at
        // `callee_fp + offset` for each slot, and `call_unchecked` zeroes
        // `[param_region_size, extended_frame_size)` afterwards, so a slot
        // outside the parameter region is either overwritten or out of frame.
        if func.param_slots.len() != func.param_tys.len() {
            self.err(
                None,
                format!(
                    "number of param slots ({}) must equal number of param types ({})",
                    func.param_slots.len(),
                    func.param_tys.len()
                ),
            );
        }
        self.check_slot_list("param", &func.param_slots, func.param_region_size);

        // Returns: one slot per type, all inside the data region.
        let num_return_tys = view_type_list(func.return_tys).len();
        if func.return_slots.len() != num_return_tys {
            self.err(
                None,
                format!(
                    "number of return slots ({}) must equal number of return types ({})",
                    func.return_slots.len(),
                    num_return_tys
                ),
            );
        }
        self.check_slot_list("return", &func.return_slots, func.param_and_local_sizes_sum);
    }

    /// Checks a parameter or return slot list: every slot well-formed and
    /// within `[0, region_end)`, slots strictly ascending and disjoint.
    fn check_slot_list(&mut self, kind: &str, slots: &[SizedSlot], region_end: usize) {
        for (i, slot) in slots.iter().enumerate() {
            self.check_sized_slot(None, &format!("{kind} slot {i}"), slot);
            // Two `u32`s widened to `usize` cannot overflow on a 64-bit target.
            let end = slot.offset.0 as usize + slot.size as usize;
            if end > region_end {
                let region = if kind == "param" {
                    "param_region_size"
                } else {
                    "param_and_local_sizes_sum"
                };
                self.err(
                    None,
                    format!(
                        "{kind} slot [{}, {}) exceeds {region} ({})",
                        slot.offset.0, end, region_end
                    ),
                );
            }
        }
        for (i, w) in slots.windows(2).enumerate() {
            let prev_end = w[0].offset.0 as usize + w[0].size as usize;
            if (w[1].offset.0 as usize) < prev_end {
                self.err(
                    None,
                    format!(
                        "{kind} slots {} and {} are not ascending and disjoint ([{}, {}) then {})",
                        i,
                        i + 1,
                        w[0].offset.0,
                        prev_end,
                        w[1].offset.0
                    ),
                );
            }
        }
    }

    /// A [`SizedSlot`] carries its own alignment, which the closure runtime
    /// feeds to `align_up` (undefined for zero or non-power-of-two) and which
    /// must divide the offset for the slot to be where the layout says.
    fn check_sized_slot(&mut self, pc: Option<usize>, what: &str, slot: &SizedSlot) {
        if slot.size == 0 {
            self.err(pc, format!("{what}: size must be > 0"));
        }
        if slot.align == 0 || !slot.align.is_power_of_two() || slot.align as usize > MAX_ALIGN {
            self.err(
                pc,
                format!(
                    "{what}: align {} must be a power of two in [1, {}]",
                    slot.align, MAX_ALIGN
                ),
            );
        } else if !slot.offset.0.is_multiple_of(slot.align) {
            self.err(
                pc,
                format!(
                    "{what}: offset {} is not {}-byte aligned",
                    slot.offset.0, slot.align
                ),
            );
        }
    }

    fn verify_gc_layouts(&mut self) {
        let code = self.func.code.ops();
        let base_offsets = &self.func.frame_layout.heap_ptr_offsets;
        let safe_point_layouts = self.func.safe_point_layouts.entries();

        // --- Base frame_layout: pointer offsets valid at every PC ---
        // Each offset must be in-bounds, aligned, not overlap metadata, and
        // sorted.
        self.check_pointer_offsets(None, base_offsets);

        // The GC scans the base layout of every frame unconditionally, so a
        // slot beyond the parameter region must start out null rather than
        // holding whatever the previous frame left there.
        if !self.func.zero_frame {
            if let Some(off) = base_offsets
                .iter()
                .find(|off| off.0 as usize >= self.func.param_region_size)
            {
                self.err(
                    None,
                    format!(
                        "frame_layout names pointer slot {} beyond param_region_size ({}) but zero_frame is false",
                        off.0, self.func.param_region_size
                    ),
                );
            }
        }

        // --- Safe-point layouts: per-PC pointer offsets ---

        // Entries must be strictly sorted by code_offset.
        for w in safe_point_layouts.windows(2) {
            if w[0].code_offset.0 >= w[1].code_offset.0 {
                self.err(
                    None,
                    format!(
                        "safe_point_layouts: entries not strictly sorted (code_offset {} >= {})",
                        w[0].code_offset.0, w[1].code_offset.0
                    ),
                );
                break;
            }
        }

        // Per-entry: valid code_offset, op-kind matches top-frame-only
        // contract, pointer offsets in-bounds and sorted, disjoint
        // from frame_layout.
        for entry in safe_point_layouts {
            let co = entry.code_offset.0;

            if (co as usize) >= code.len() {
                self.err(
                    None,
                    format!(
                        "safe_point_layouts: code_offset {} out of bounds (code length {})",
                        co,
                        code.len()
                    ),
                );
            } else {
                // Top-frame-only contract: an entry must sit at the
                // PC of an allocating op (see `MicroOp::is_allocating`).
                let op = &code[co as usize];
                if !op.is_allocating() {
                    self.err(
                        Some(co as usize),
                        format!(
                            "safe_point_layouts: code_offset {} is not at an allocating op; \
                             top-frame-only contract — see `SafePointEntry`",
                            co
                        ),
                    );
                }
            }

            let sp_offsets = &entry.layout.heap_ptr_offsets;
            self.check_pointer_offsets(Some(co as usize), sp_offsets);

            for &off in sp_offsets {
                if base_offsets.contains(&off) {
                    self.err(
                        Some(co as usize),
                        format!(
                            "safe_point_layouts: offset {} duplicates frame_layout",
                            off.0
                        ),
                    );
                }
            }
        }
    }

    /// Validate a set of pointer offsets: each must be an aligned pointer slot
    /// within the extended frame, not overlap the metadata segment, and the
    /// list must be strictly sorted. The GC reads them with aligned `read_ptr`.
    fn check_pointer_offsets(&mut self, pc: Option<usize>, offsets: &[FrameOffset]) {
        for &off in offsets {
            self.check_access(pc, off, PTR_WIDTH, PTR_ALIGN);
        }
        for w in offsets.windows(2) {
            if w[0].0 >= w[1].0 {
                self.err(
                    pc,
                    format!(
                        "pointer_offsets not strictly sorted ({} >= {})",
                        w[0].0, w[1].0
                    ),
                );
                break;
            }
        }
    }

    // -----------------------------------------------------------------------
    // Per-instruction checks
    // -----------------------------------------------------------------------

    fn verify_instruction(&mut self, pc: usize, instr: &MicroOp) {
        match *instr {
            // Immediates are stored as byte arrays, with no alignment requirement.
            MicroOp::StoreImm1 { dst, imm: _ } => self.check_bytes(pc, dst, 1),
            MicroOp::StoreImm2 { dst, imm: _ } => self.check_bytes(pc, dst, 2),
            MicroOp::StoreImm4 { dst, imm: _ } => self.check_bytes(pc, dst, 4),
            MicroOp::StoreImm8 { dst, imm: _ } => self.check_bytes(pc, dst, 8),
            MicroOp::StoreImm16 { dst, imm: _ } => self.check_bytes(pc, dst, 16),
            MicroOp::StoreImm32 { dst, imm: _ } => self.check_bytes(pc, dst, 32),

            MicroOp::StoreRandomU64 { dst } => self.check_u64(pc, dst),

            MicroOp::AddU64Imm { dst, src, imm: _ }
            | MicroOp::SubU64Imm { dst, src, imm: _ }
            | MicroOp::RSubU64Imm { dst, src, imm: _ }
            | MicroOp::MulU64Imm { dst, src, imm: _ } => {
                self.check_u64(pc, src);
                self.check_u64(pc, dst);
            },

            // These unchecked u64 ops require `imm != 0`. Lowering uses checked
            // ops for zero divisors, so an invalid immediate here is a lowering bug.
            MicroOp::DivU64Imm { dst, src, imm } | MicroOp::ModU64Imm { dst, src, imm } => {
                self.check_u64(pc, src);
                self.check_u64(pc, dst);
                if imm == 0 {
                    self.err(Some(pc), "division by zero (imm)");
                }
            },

            // These unchecked u64 ops require `imm < 64`. Lowering uses checked
            // ops for out-of-range shifts, so an invalid immediate here is a lowering bug.
            MicroOp::ShlU64Imm { dst, src, imm } | MicroOp::ShrU64Imm { dst, src, imm } => {
                self.check_u64(pc, src);
                self.check_u64(pc, dst);
                if imm >= 64 {
                    self.err(Some(pc), format!("shift amount {} exceeds 63 (imm)", imm));
                }
            },

            // `Move8` reads and writes unaligned, so its slots need no alignment.
            MicroOp::Move8 { dst, src } => {
                self.check_bytes(pc, src, 8);
                self.check_bytes(pc, dst, 8);
            },

            MicroOp::AddU64 { dst, lhs, rhs }
            | MicroOp::SubU64 { dst, lhs, rhs }
            | MicroOp::MulU64 { dst, lhs, rhs }
            | MicroOp::DivU64 { dst, lhs, rhs }
            | MicroOp::ModU64 { dst, lhs, rhs }
            | MicroOp::BitAndU64 { dst, lhs, rhs }
            | MicroOp::BitOrU64 { dst, lhs, rhs }
            | MicroOp::BitXorU64 { dst, lhs, rhs } => {
                self.check_u64(pc, lhs);
                self.check_u64(pc, rhs);
                self.check_u64(pc, dst);
            },

            // Shifts: `rhs` is a 1-byte slot (the Move shift amount is u8).
            MicroOp::ShlU64 { dst, lhs, rhs } | MicroOp::ShrU64 { dst, lhs, rhs } => {
                self.check_u64(pc, lhs);
                self.check_byte(pc, rhs);
                self.check_u64(pc, dst);
            },

            // Unspecialized integer binary ops. Checks:
            //   - `dst`, `lhs`, and (if `rhs` is a slot) `rhs` are all
            //     in-bounds slots of width `op.rhs.byte_width()`.
            //   - Bitwise ops reject signed operands.
            // Zero immediate divisors are valid here: `IntDiv` and `IntMod`
            // report division by zero only when executed.
            MicroOp::IntAdd(ref op)
            | MicroOp::IntSub(ref op)
            | MicroOp::IntMul(ref op)
            | MicroOp::IntDiv(ref op)
            | MicroOp::IntMod(ref op) => {
                self.check_int_binop_frame_access(pc, op);
            },
            MicroOp::IntBitAnd(ref op) | MicroOp::IntBitOr(ref op) | MicroOp::IntBitXor(ref op) => {
                self.check_int_binop_frame_access(pc, op);
                if op.rhs.is_signed() {
                    self.err(Some(pc), "bitwise on signed type");
                }
            },

            // `lhs` and `dst` use `op.ty.byte_width()` bytes; `rhs` is a
            // one-byte slot or an inline u8. Shift amounts are checked at
            // runtime, so out-of-range immediates are valid here. Signed
            // shifts are an unconditional runtime invariant violation, so
            // reject them statically.
            MicroOp::IntShl(op) | MicroOp::IntShr(op) => {
                self.check_int(pc, op.lhs, op.ty.byte_width() as u32);
                self.check_int(pc, op.dst, op.ty.byte_width() as u32);
                match op.rhs {
                    ShiftOperand::SlotU8(rhs) => self.check_byte(pc, rhs),
                    ShiftOperand::ImmU8(_) => {},
                }
                if op.ty.is_signed() {
                    self.err(Some(pc), "shift on signed type");
                }
            },

            // `IntNegate` is signed-only: an unsigned type is an unconditional
            // runtime invariant violation. The `src == MIN` overflow case is a
            // runtime abort.
            MicroOp::IntNegate(op) => {
                self.check_int(pc, op.src, op.ty.byte_width() as u32);
                self.check_int(pc, op.dst, op.ty.byte_width() as u32);
                if !op.ty.is_signed() {
                    self.err(Some(pc), "negate on unsigned type");
                }
            },

            MicroOp::IntCast(op) => {
                // Note: the Move bytecode permits casting from one integer type to self, effectively a no-op.
                // Therefore we must NOT ban it here.
                self.check_int(pc, op.src, op.from.byte_width() as u32);
                self.check_int(pc, op.dst, op.to.byte_width() as u32);
            },

            // Comparison: `lhs` (and `rhs` slot, if any) are `rhs.byte_width()`
            // wide; `dst` is a 1-byte boolean. Both signed and unsigned
            // operands are valid.
            MicroOp::IntCmp(ref op) => {
                let size = op.rhs.byte_width() as u32;
                self.check_int(pc, op.lhs, size);
                if let Some(rhs_off) = op.rhs.slot_offset() {
                    self.check_int(pc, rhs_off, size);
                }
                self.check_bool(pc, op.dst);
            },

            MicroOp::ValueCmp(ref op) => {
                self.check_value_operands(pc, op.ty, op.lhs, op.rhs);
                self.check_bool(pc, op.dst);
            },
            MicroOp::ValueRefCmp(ref op) => {
                self.check_fat_ptr(pc, op.lhs);
                self.check_fat_ptr(pc, op.rhs);
                self.check_value_type(pc, op.ty);
                self.check_bool(pc, op.dst);
            },

            // Boolean logic: all operands are 1-byte `0`/`1` values.
            MicroOp::BoolNot { dst, src } => {
                self.check_bool(pc, src);
                self.check_bool(pc, dst);
            },
            MicroOp::BoolAnd { dst, lhs, rhs } | MicroOp::BoolOr { dst, lhs, rhs } => {
                self.check_bool(pc, lhs);
                self.check_bool(pc, rhs);
                self.check_bool(pc, dst);
            },

            MicroOp::Move { dst, src, size } => {
                self.check_nonzero_size(pc, size);
                self.check_bytes(pc, src, size);
                self.check_bytes(pc, dst, size);
            },

            MicroOp::Jump { target, .. } => {
                self.check_jump(pc, target);
            },

            MicroOp::JumpNotZeroU64 { target, src, .. } => {
                self.check_u64(pc, src);
                self.check_jump(pc, target);
            },

            MicroOp::JumpNotZeroByte { target, src, .. }
            | MicroOp::JumpZeroByte { target, src, .. } => {
                self.check_byte(pc, src);
                self.check_jump(pc, target);
            },

            // `lhs` and the `rhs` slot (if any) are `rhs.byte_width()` wide;
            // either signedness is allowed.
            MicroOp::JumpIntCmp(ref op) => {
                let size = op.rhs.byte_width() as u32;
                self.check_int(pc, op.lhs, size);
                if let Some(rhs_off) = op.rhs.slot_offset() {
                    self.check_int(pc, rhs_off, size);
                }
                self.check_jump(pc, op.target);
            },

            MicroOp::JumpValueCmp(ref op) => {
                self.check_value_operands(pc, op.ty, op.lhs, op.rhs);
                self.check_jump(pc, op.target);
            },
            MicroOp::JumpValueRefCmp(ref op) => {
                self.check_fat_ptr(pc, op.lhs);
                self.check_fat_ptr(pc, op.rhs);
                self.check_value_type(pc, op.ty);
                self.check_jump(pc, op.target);
            },

            MicroOp::JumpGreaterEqualU64Imm { target, src, .. }
            | MicroOp::JumpLessU64Imm { target, src, .. }
            | MicroOp::JumpGreaterU64Imm { target, src, .. }
            | MicroOp::JumpLessEqualU64Imm { target, src, .. } => {
                self.check_u64(pc, src);
                self.check_jump(pc, target);
            },

            MicroOp::JumpLessU64 {
                target, lhs, rhs, ..
            }
            | MicroOp::JumpGreaterEqualU64 {
                target, lhs, rhs, ..
            }
            | MicroOp::JumpNotEqualU64 {
                target, lhs, rhs, ..
            } => {
                self.check_u64(pc, lhs);
                self.check_u64(pc, rhs);
                self.check_jump(pc, target);
            },

            MicroOp::Return | MicroOp::ForceGC => {},

            MicroOp::Abort { code } => {
                self.check_u64(pc, code);
            },

            // `message` is an owned `vector<u8>` heap pointer, read by value.
            MicroOp::AbortMsg { code, message } => {
                self.check_u64(pc, code);
                self.check_ptr(pc, message);
            },

            MicroOp::CallIndirect { .. } => {},

            MicroOp::CallDirect { ref ptr } => {
                // SAFETY: the function pointer lives in the global context,
                // which the caller's guard keeps alive for the duration of
                // verification.
                let callee = unsafe { ptr.as_ref_unchecked() };
                self.check_direct_callee(pc, callee);
            },

            MicroOp::CallNative { ref abi, .. } => {
                self.check_native_abi(pc, abi);
            },

            // ----- VecNew -----
            MicroOp::VecNew { dst } => {
                self.check_ptr(pc, dst);
            },

            // ----- StoreImmVec: deserializes a constant into `dst` -----
            MicroOp::StoreImmVec { dst, idx } => {
                self.check_store_imm_vec(pc, dst, idx);
            },

            MicroOp::VecLen { dst, vec_ref } => {
                self.check_fat_ptr(pc, vec_ref);
                self.check_u64(pc, dst);
            },

            // The 8-byte field moves read and write the frame side unaligned.
            MicroOp::HeapMoveFrom8 {
                dst,
                heap_ptr,
                offset,
            } => {
                self.check_ptr(pc, heap_ptr);
                self.check_ref_offset_size_no_overflow(pc, offset, PTR_WIDTH);
                self.check_bytes(pc, dst, 8);
            },

            MicroOp::HeapMoveTo8 {
                heap_ptr,
                offset,
                src,
            } => {
                self.check_ptr(pc, heap_ptr);
                self.check_ref_offset_size_no_overflow(pc, offset, PTR_WIDTH);
                self.check_bytes(pc, src, 8);
            },

            MicroOp::EnumTestTag { dst, enum_ref, .. } => {
                self.check_fat_ptr(pc, enum_ref);
                self.check_bool(pc, dst);
            },

            MicroOp::EnumBorrowVariantFieldByTag { dst, enum_ref, .. } => {
                self.check_fat_ptr(pc, enum_ref);
                self.check_fat_ptr(pc, dst);
            },

            MicroOp::EnumCheckVariant { enum_ptr, .. } => {
                self.check_ptr(pc, enum_ptr);
            },

            MicroOp::EnumNew {
                dst,
                descriptor_id,
                variant,
            } => {
                self.check_ptr(pc, dst);
                self.check_enum_new(pc, descriptor_id, variant);
            },

            // Read's `dst` and write's `src` are both the size-wide frame slot
            // the value moves to/from; the checks are otherwise identical.
            MicroOp::HeapReadOffset {
                dst: value_slot,
                obj_ref,
                offset,
                size,
            }
            | MicroOp::HeapWriteOffset {
                obj_ref,
                offset,
                src: value_slot,
                size,
            } => {
                self.check_fat_ptr(pc, obj_ref);
                self.check_nonzero_size(pc, size);
                self.check_ref_offset_size_no_overflow(pc, offset, size);
                self.check_bytes(pc, value_slot, size);
            },

            // Read's `dst` and write's `src` are both the size-wide frame slot
            // the field value moves to/from; the checks are otherwise identical.
            MicroOp::EnumReadVariantFieldByTag {
                dst: value_slot,
                enum_ref,
                ref offsets,
                size,
            }
            | MicroOp::EnumWriteVariantFieldByTag {
                src: value_slot,
                enum_ref,
                ref offsets,
                size,
            } => {
                self.check_fat_ptr(pc, enum_ref);
                self.check_nonzero_size(pc, size);
                // Any tag may be selected at runtime, so every present offset
                // must keep `offset + size` within `u32`.
                for offset in offsets.iter().flatten() {
                    self.check_ref_offset_size_no_overflow(pc, *offset, size);
                }
                self.check_bytes(pc, value_slot, size);
            },

            // Each owned heap pointer at `base + off` is an aligned 8-byte
            // frame slot. The interpreter adds `base + off` in `u32`.
            MicroOp::DeepCopyHeapPtrs { base, ref offsets } => {
                for &off in offsets.iter() {
                    match base.0.checked_add(off) {
                        Some(slot) => self.check_ptr(pc, FrameOffset(slot)),
                        None => self.err(
                            Some(pc),
                            format!(
                                "DeepCopyHeapPtrs: base {} + offset {} overflows u32",
                                base.0, off
                            ),
                        ),
                    }
                }
            },

            // ----- Vec push/pop: vec_ref (16B fat pointer) + variable-width slot -----
            MicroOp::VecPushBack {
                vec_ref,
                elem,
                elem_size,
                descriptor_id,
            } => {
                self.check_fat_ptr(pc, vec_ref);
                self.check_nonzero_size(pc, elem_size);
                self.check_bytes(pc, elem, elem_size);
                self.check_vector_descriptor(pc, "VecPushBack", descriptor_id, elem_size);
            },

            MicroOp::VecPopBack {
                dst,
                vec_ref,
                elem_size,
            } => {
                self.check_fat_ptr(pc, vec_ref);
                self.check_nonzero_size(pc, elem_size);
                self.check_bytes(pc, dst, elem_size);
            },

            // ----- Vec indexed load/store -----
            MicroOp::VecLoadElem {
                dst,
                vec_ref,
                idx,
                elem_size,
            } => {
                self.check_fat_ptr(pc, vec_ref);
                self.check_u64(pc, idx);
                self.check_nonzero_size(pc, elem_size);
                self.check_bytes(pc, dst, elem_size);
            },

            MicroOp::VecStoreElem {
                vec_ref,
                idx,
                src,
                elem_size,
            } => {
                self.check_fat_ptr(pc, vec_ref);
                self.check_u64(pc, idx);
                self.check_nonzero_size(pc, elem_size);
                self.check_bytes(pc, src, elem_size);
            },

            MicroOp::VecPack(ref op) => {
                self.check_ptr(pc, op.dst);
                self.check_nonzero_size(pc, op.elem_size);
                for &src in &op.srcs {
                    self.check_bytes(pc, src, op.elem_size);
                }
                self.check_vector_descriptor(pc, "VecPack", op.descriptor_id, op.elem_size);
            },

            // Multi-destination: the element copies are independent
            // `copy_nonoverlapping`s, so the destinations must be disjoint.
            MicroOp::VecUnpack(ref op) => {
                self.check_ptr(pc, op.src);
                self.check_nonzero_size(pc, op.elem_size);
                for &dst in &op.dsts {
                    self.check_bytes(pc, dst, op.elem_size);
                }
                self.check_disjoint_destinations(pc, "VecUnpack", &op.dsts, op.elem_size);
            },

            MicroOp::VecSwap {
                vec_ref,
                idx_a,
                idx_b,
                elem_size,
            } => {
                self.check_fat_ptr(pc, vec_ref);
                self.check_u64(pc, idx_a);
                self.check_u64(pc, idx_b);
                self.check_nonzero_size(pc, elem_size);
            },

            // ----- Borrow producing fat pointer (16B dst) -----
            MicroOp::VecBorrow {
                dst,
                vec_ref,
                idx,
                elem_size,
            } => {
                self.check_fat_ptr(pc, vec_ref);
                self.check_u64(pc, idx);
                self.check_nonzero_size(pc, elem_size);
                self.check_fat_ptr(pc, dst);
            },

            MicroOp::SlotBorrow { dst, local } => {
                // Forms a fat pointer to `local` without dereferencing it, so only
                // the base offset is checked: `local` must lie in the data region
                // [0, param_and_local_sizes_sum), not metadata or the callee region.
                // The op carries no size, so the borrowed value's full extent
                // (`local + size`) is not bounds-checked here: a base in-region
                // whose value extends past the region end is not rejected.
                if local.0 as usize >= self.func.param_and_local_sizes_sum {
                    self.err(
                        Some(pc),
                        format!(
                            "SlotBorrow local {} is outside the data region [0, {})",
                            local.0, self.func.param_and_local_sizes_sum
                        ),
                    );
                }
                self.check_fat_ptr(pc, dst);
            },

            MicroOp::HeapBorrow { dst, obj_ref, .. } => {
                self.check_fat_ptr(pc, obj_ref);
                self.check_fat_ptr(pc, dst);
            },

            MicroOp::ReadRef { dst, ref_ptr, size } => {
                self.check_fat_ptr(pc, ref_ptr);
                self.check_nonzero_size(pc, size);
                self.check_bytes(pc, dst, size);
            },

            MicroOp::WriteRef { ref_ptr, src, size } => {
                self.check_fat_ptr(pc, ref_ptr);
                self.check_nonzero_size(pc, size);
                self.check_bytes(pc, src, size);
            },

            MicroOp::DeriveRefOffsetImm {
                dst_ref, src_ref, ..
            } => {
                self.check_fat_ptr(pc, src_ref);
                self.check_fat_ptr(pc, dst_ref);
            },

            MicroOp::ReadRefOffset {
                dst,
                ref_ptr,
                offset,
                size,
            } => {
                self.check_fat_ptr(pc, ref_ptr);
                self.check_nonzero_size(pc, size);
                self.check_ref_offset_size_no_overflow(pc, offset, size);
                self.check_bytes(pc, dst, size);
            },

            MicroOp::WriteRefOffset {
                ref_ptr,
                offset,
                src,
                size,
            } => {
                self.check_fat_ptr(pc, ref_ptr);
                self.check_nonzero_size(pc, size);
                self.check_ref_offset_size_no_overflow(pc, offset, size);
                self.check_bytes(pc, src, size);
            },

            // ----- Heap object instructions -----
            MicroOp::HeapNew { dst, descriptor_id } => {
                self.check_ptr(pc, dst);
                self.check_descriptor_variant(
                    pc,
                    "HeapNew",
                    descriptor_id,
                    |inner| {
                        matches!(
                            inner,
                            ObjectDescriptorInner::Struct { .. }
                                | ObjectDescriptorInner::Enum { .. }
                        )
                    },
                    "a Struct or Enum",
                );
            },

            MicroOp::HeapMoveToImm8 {
                heap_ptr, offset, ..
            } => {
                self.check_ptr(pc, heap_ptr);
                self.check_ref_offset_size_no_overflow(pc, offset, PTR_WIDTH);
            },

            MicroOp::HeapMoveFrom {
                dst,
                heap_ptr,
                offset,
                size,
            } => {
                self.check_ptr(pc, heap_ptr);
                self.check_nonzero_size(pc, size);
                self.check_ref_offset_size_no_overflow(pc, offset, size);
                self.check_bytes(pc, dst, size);
            },

            MicroOp::HeapMoveTo {
                heap_ptr,
                offset,
                src,
                size,
            } => {
                self.check_ptr(pc, heap_ptr);
                self.check_nonzero_size(pc, size);
                self.check_ref_offset_size_no_overflow(pc, offset, size);
                self.check_bytes(pc, src, size);
            },

            MicroOp::PackClosure(ref op) => self.verify_pack_closure(pc, op),
            MicroOp::CallClosure(ref op) => self.verify_call_closure(pc, op),

            // `addr` is a 32-byte inline address, read unaligned.
            MicroOp::Exists { addr, ty: _, dst } => {
                self.check_bytes(pc, addr, ADDRESS_WIDTH);
                self.check_bool(pc, dst);
            },
            MicroOp::MoveFrom { addr, ty: _, dst } => {
                // MoveFrom writes an 8-byte owned heap pointer.
                self.check_bytes(pc, addr, ADDRESS_WIDTH);
                self.check_ptr(pc, dst);
            },
            MicroOp::BorrowGlobal { addr, ty: _, dst }
            | MicroOp::BorrowGlobalMut { addr, ty: _, dst } => {
                // Both produce a reference, i.e. a 16-byte fat pointer.
                self.check_bytes(pc, addr, ADDRESS_WIDTH);
                self.check_fat_ptr(pc, dst);
            },
            MicroOp::MoveTo {
                signer_ref,
                ty: _,
                src,
            } => {
                // `signer_ref` is a 16-byte `&signer` fat pointer; `src` is an
                // 8-byte owned heap pointer to the resource value.
                self.check_fat_ptr(pc, signer_ref);
                self.check_ptr(pc, src);
            },
        }
    }

    fn verify_pack_closure(&mut self, pc: usize, op: &PackClosureOp) {
        // Destination: 8-byte heap pointer slot for the closure heap object.
        self.check_ptr(pc, op.dst);
        // The closure heap object uses the implicit reserved
        // `CLOSURE_DESCRIPTOR_ID` (no per-op field). Every provider installs
        // `Closure` at this slot; assert to catch internal regressions.
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
        // captured_data_descriptor_id and `captured` must agree on emptiness:
        // a captured-data object is allocated iff at least one value is
        // captured.
        match (op.captured_data_descriptor_id, op.captured.is_empty()) {
            (None, true) => {},
            (Some(id), false) => {
                // Pointer-free captures use the reserved `Trivial` slot;
                // pointer-bearing ones use a `CapturedData` descriptor whose
                // heap-pointer offsets must lie within the values region so GC
                // traces stay in bounds. One lookup validates both.
                match self.provider.descriptor(id).map(|d| d.inner()) {
                    None => self.err(
                        Some(pc),
                        format!("PackClosure: unknown descriptor_id {}", id),
                    ),
                    Some(ObjectDescriptorInner::Trivial) => {},
                    Some(ObjectDescriptorInner::CapturedData { pointer_offsets }) => {
                        for &off in pointer_offsets {
                            if off as u64 + 8 > op.values_size as u64 {
                                self.err(
                                    Some(pc),
                                    format!(
                                        "PackClosure: captured_data pointer offset {} out of bounds of values_size {}",
                                        off, op.values_size
                                    ),
                                );
                            }
                        }
                    },
                    Some(_) => self.err(
                        Some(pc),
                        format!(
                            "PackClosure: descriptor_id {} is not a Trivial or CapturedData",
                            id
                        ),
                    ),
                }
            },
            (Some(id), true) => {
                self.err(
                    Some(pc),
                    format!(
                        "PackClosure: captured_data_descriptor_id {} provided but no captures",
                        id
                    ),
                );
            },
            (None, false) => {
                self.err(
                    Some(pc),
                    "PackClosure: captured non-empty but captured_data_descriptor_id is None"
                        .to_string(),
                );
            },
        }
        // Captured sources: each slot well-formed (its `align` drives the
        // captured-data layout) and in-bounds; the copy itself is bytewise.
        for (i, slot) in op.captured.iter().enumerate() {
            self.check_sized_slot(Some(pc), &format!("PackClosure: captured[{i}]"), slot);
            self.check_bytes(pc, slot.offset, slot.size);
        }
        // Captured count must match the mask.
        let captured_count = op.mask.count_ones() as usize;
        if op.captured.len() != captured_count {
            self.err(
                Some(pc),
                format!(
                    "PackClosure: captured list length {} does not match mask captured count {}",
                    op.captured.len(),
                    captured_count
                ),
            );
        }
        // For Resolved targets the mask must not set bits beyond the
        // callee's parameter count, and the callee's parameter count must
        // fit in the u64 mask.
        match &op.func_ref {
            ClosureFuncRef::Resolved(func_ptr) => {
                // SAFETY: the function pointer lives in the global context,
                // which the caller's guard keeps alive for the duration of
                // verification.
                let callee = unsafe { func_ptr.as_ref_unchecked() };
                let param_count = callee.param_slots.len();
                if param_count > 64 {
                    self.err(
                        Some(pc),
                        format!(
                            "PackClosure: callee has {} params, exceeds 64-bit mask capacity",
                            param_count
                        ),
                    );
                }
                if param_count < 64 && op.mask >> param_count != 0 {
                    self.err(
                        Some(pc),
                        format!(
                            "PackClosure: mask 0x{:x} sets bits beyond callee param count {}",
                            op.mask, param_count
                        ),
                    );
                }
                // Each captured slot's size AND alignment must match the
                // corresponding callee parameter's: the runtime writes captured
                // values using the slot's `(size, align)` but reads them back at
                // the callee parameter's natural-aligned offset, so a mismatch
                // (even with equal sizes) desyncs the write and read layouts and
                // can read past the values region. The captured list is in
                // mask-bit-set order through the param list.
                let mut k = 0usize;
                for (i, param_slot) in callee.param_slots.iter().enumerate() {
                    if (op.mask >> i) & 1 != 0 {
                        if let Some(slot) = op.captured.get(k) {
                            if slot.size != param_slot.size {
                                self.err(
                                    Some(pc),
                                    format!(
                                        "PackClosure: captured[{}].size {} != callee param_slots[{}].size {}",
                                        k, slot.size, i, param_slot.size,
                                    ),
                                );
                            }
                            if slot.align != param_slot.align {
                                self.err(
                                    Some(pc),
                                    format!(
                                        "PackClosure: captured[{}].align {} != callee param_slots[{}].align {}",
                                        k, slot.align, i, param_slot.align,
                                    ),
                                );
                            }
                        }
                        k += 1;
                    }
                }
            },
            ClosureFuncRef::Unresolved(_) => {
                // Symbolic target: the callee isn't materialized here, so the
                // callee-dependent checks (param count, mask range, captured
                // layout bounds, provided-arg sizes) are deferred to call time
                // against the resolved callee. The mask/captured-count agreement
                // checked above still applies.
            },
        }
        // `values_size` must equal the natural-aligned captured layout size:
        // the runtime writes captured values at those fixed offsets, so a
        // smaller size would let writes run out of bounds. Skipped when a
        // captured slot's alignment is invalid, since the layout is then
        // undefined (and already reported).
        if op
            .captured
            .iter()
            .all(|slot| slot.align != 0 && slot.align.is_power_of_two())
        {
            let expected_values_size =
                captured_values_size(op.captured.iter().map(|slot| (slot.size, slot.align)));
            if op.values_size != expected_values_size {
                self.err(
                    Some(pc),
                    format!(
                        "PackClosure: values_size {} != captured layout size {}",
                        op.values_size, expected_values_size
                    ),
                );
            }
        }
    }

    fn verify_call_closure(&mut self, pc: usize, op: &CallClosureOp) {
        // Closure source: 8-byte heap pointer slot.
        self.check_ptr(pc, op.closure_src);
        // Provided arg sources: each slot well-formed and in-bounds; the copy
        // into the callee frame is bytewise.
        for (i, slot) in op.provided_args.iter().enumerate() {
            self.check_sized_slot(Some(pc), &format!("CallClosure: provided_args[{i}]"), slot);
            self.check_bytes(pc, slot.offset, slot.size);
        }
    }

    // -----------------------------------------------------------------------
    // Frame access helpers
    // -----------------------------------------------------------------------

    fn err(&mut self, pc: Option<usize>, msg: impl Into<String>) {
        self.errors.push(VerificationError {
            func_name: self.func.name().to_string(),
            pc,
            message: msg.into(),
        });
    }

    /// The core frame-access check: `[offset, offset + size)` lies within the
    /// extended frame, does not overlap the metadata block, and `offset` is a
    /// multiple of `align`. Accesses into the callee arg/return region
    /// (`[frame_size(), extended_frame_size)`) are permitted: that is how
    /// arguments and return values are passed.
    fn check_access(&mut self, pc: Option<usize>, offset: FrameOffset, size: u32, align: u32) {
        let offset = offset.0 as usize;
        let width = size as usize;
        let end = match offset.checked_add(width) {
            Some(e) => e,
            None => {
                self.err(pc, format!("access at offset {} overflows", offset));
                return;
            },
        };

        if end > self.func.extended_frame_size {
            self.err(
                pc,
                format!(
                    "access [{}, {}) exceeds extended_frame_size {}",
                    offset, end, self.func.extended_frame_size
                ),
            );
            return;
        }

        let meta_start = self.func.param_and_local_sizes_sum;
        let meta_end = self.func.param_and_local_sizes_sum + FRAME_METADATA_SIZE;
        if offset < meta_end && meta_start < end {
            self.err(
                pc,
                format!(
                    "access [{}, {}) overlaps metadata [{}, {})",
                    offset, end, meta_start, meta_end
                ),
            );
        }

        if !offset.is_multiple_of(align as usize) {
            self.err(
                pc,
                format!("access [{}, {}) is not {}-byte aligned", offset, end, align),
            );
        }
    }

    /// A byte-copied or unaligned-accessed slot of `size` bytes.
    fn check_bytes(&mut self, pc: usize, offset: FrameOffset, size: u32) {
        self.check_access(Some(pc), offset, size, NO_ALIGN);
    }

    /// A 1-byte boolean slot.
    fn check_bool(&mut self, pc: usize, offset: FrameOffset) {
        self.check_access(Some(pc), offset, BOOL_WIDTH, NO_ALIGN);
    }

    /// A 1-byte integer slot (shift amounts).
    fn check_byte(&mut self, pc: usize, offset: FrameOffset) {
        self.check_access(Some(pc), offset, 1, NO_ALIGN);
    }

    /// An aligned `u64` slot (`read_u64` / `write_u64`).
    fn check_u64(&mut self, pc: usize, offset: FrameOffset) {
        self.check_access(Some(pc), offset, PTR_WIDTH, PTR_ALIGN);
    }

    /// An aligned heap-pointer slot (`read_ptr` / `write_ptr`).
    fn check_ptr(&mut self, pc: usize, offset: FrameOffset) {
        self.check_access(Some(pc), offset, PTR_WIDTH, PTR_ALIGN);
    }

    /// An aligned 16-byte reference slot (`read_fat_ptr` / `write_fat_ptr`).
    fn check_fat_ptr(&mut self, pc: usize, offset: FrameOffset) {
        self.check_access(Some(pc), offset, FAT_PTR_WIDTH, PTR_ALIGN);
    }

    /// An integer slot accessed with `read_int<T>` / `write_int<T>`.
    fn check_int(&mut self, pc: usize, offset: FrameOffset, width: u32) {
        self.check_access(Some(pc), offset, width, int_align(width));
    }

    /// Verify an [`IntBinaryOp`]: dst and lhs are slots of width
    /// `op.rhs.byte_width()`; if rhs is a slot arm, its slot is checked too.
    fn check_int_binop_frame_access(&mut self, pc: usize, op: &IntBinaryOp) {
        let size = op.rhs.byte_width() as u32;
        self.check_int(pc, op.lhs, size);
        self.check_int(pc, op.dst, size);
        if let Some(rhs_off) = op.rhs.slot_offset() {
            self.check_int(pc, rhs_off, size);
        }
    }

    /// The two operands of a by-value comparison occupy `size_and_align(ty)`
    /// at their slots: the full inline value for primitives and structs, an
    /// aligned 8-byte pointer for vectors, enums, and functions.
    fn check_value_operands(
        &mut self,
        pc: usize,
        ty: InternedType,
        lhs: FrameOffset,
        rhs: FrameOffset,
    ) {
        self.check_value_type(pc, ty);
        let Some((size, align)) = self.provider.size_and_align(ty) else {
            self.err(
                Some(pc),
                "value comparison operand type has no known layout",
            );
            return;
        };
        self.check_access(Some(pc), lhs, size, align);
        self.check_access(Some(pc), rhs, size, align);
    }

    /// Structural comparison is defined on values, not references; a
    /// reference type is an unconditional runtime invariant violation.
    fn check_value_type(&mut self, pc: usize, ty: InternedType) {
        if matches!(view_type(ty), Type::ImmutRef { .. } | Type::MutRef { .. }) {
            self.err(Some(pc), "value comparison on a reference type");
        }
    }

    /// Destinations of a multi-destination op must not overlap: the copies
    /// are independent `copy_nonoverlapping`s.
    fn check_disjoint_destinations(
        &mut self,
        pc: usize,
        op: &str,
        dsts: &[FrameOffset],
        width: u32,
    ) {
        let mut sorted: Vec<u64> = dsts.iter().map(|d| d.0 as u64).collect();
        sorted.sort_unstable();
        for w in sorted.windows(2) {
            if w[0] + width as u64 > w[1] {
                self.err(
                    Some(pc),
                    format!(
                        "{op}: destinations [{}, {}) and [{}, {}) overlap",
                        w[0],
                        w[0] + width as u64,
                        w[1],
                        w[1] + width as u64
                    ),
                );
                break;
            }
        }
    }

    // -----------------------------------------------------------------------
    // Call helpers
    // -----------------------------------------------------------------------

    /// Verify the native's slot region fits within the caller's extended
    /// frame, and that its pointer slots (which the GC reads with aligned
    /// `read_ptr` at the native's fp) are aligned, inside the region, and
    /// inside an argument slot.
    fn check_native_abi(&mut self, pc: usize, abi: &NativeABI) {
        let callee_base = self.func.frame_size();
        let total = abi.total_frame_size();
        let end = match callee_base.checked_add(total as usize) {
            Some(e) => e,
            None => {
                self.err(Some(pc), "native slot region overflows usize");
                return;
            },
        };
        if end > self.func.extended_frame_size {
            self.err(
                Some(pc),
                format!(
                    "native slot region [{}, {}) exceeds extended_frame_size {}",
                    callee_base, end, self.func.extended_frame_size,
                ),
            );
        }
        for &off in abi.heap_ptr_offsets() {
            let off_end = off.0 as u64 + PTR_WIDTH as u64;
            if off_end > total as u64 {
                self.err(
                    Some(pc),
                    format!(
                        "native heap pointer offset {} exceeds the slot region ({})",
                        off.0, total
                    ),
                );
            }
            if !off.0.is_multiple_of(PTR_ALIGN) {
                self.err(
                    Some(pc),
                    format!("native heap pointer offset {} is not 8-byte aligned", off.0),
                );
            }
            let inside_arg = abi.args().iter().any(|slot| {
                slot.offset as u64 <= off.0 as u64
                    && off_end <= slot.offset as u64 + slot.size as u64
            });
            if !inside_arg {
                self.err(
                    Some(pc),
                    format!(
                        "native heap pointer offset {} is not inside an argument slot",
                        off.0
                    ),
                );
            }
        }
        for (i, id) in abi.required_descriptors().iter().enumerate() {
            if self.provider.descriptor(*id).is_none() {
                self.err(
                    Some(pc),
                    format!("native required descriptor {i} ({id}) is unknown"),
                );
            }
        }
    }

    /// A direct callee's parameters are written by this function into its
    /// callee region, and its return values are read back from there, so
    /// both must fit in `[frame_size(), extended_frame_size)`.
    fn check_direct_callee(&mut self, pc: usize, callee: &Function) {
        let region = self
            .func
            .extended_frame_size
            .saturating_sub(self.func.frame_size());
        if callee.param_region_size > region {
            self.err(
                Some(pc),
                format!(
                    "CallDirect: callee param_region_size {} exceeds the callee region ({})",
                    callee.param_region_size, region
                ),
            );
        }
        for (i, slot) in callee.return_slots.iter().enumerate() {
            let end = slot.offset.0 as usize + slot.size as usize;
            if end > region {
                self.err(
                    Some(pc),
                    format!(
                        "CallDirect: callee return slot {i} [{}, {}) exceeds the callee region ({})",
                        slot.offset.0, end, region
                    ),
                );
            }
        }
    }

    // -----------------------------------------------------------------------
    // Descriptor and constant helpers
    // -----------------------------------------------------------------------

    /// `StoreImmVec` deserializes the constant into `dst`, writing the
    /// constant type's in-frame image: an aligned pointer for vectors and
    /// enums, the inline bytes otherwise.
    fn check_store_imm_vec(&mut self, pc: usize, dst: FrameOffset, idx: ConstantPoolIndex) {
        let Some(ty) = self.provider.constant_type(self.func.module_id, idx) else {
            self.err(
                Some(pc),
                format!("StoreImmVec: constant pool index {} out of range", idx.0),
            );
            return;
        };
        let Some((size, align)) = self.provider.size_and_align(ty) else {
            self.err(
                Some(pc),
                format!("StoreImmVec: constant {} has no known layout", idx.0),
            );
            return;
        };
        self.check_access(Some(pc), dst, size, align);
    }

    /// Checks `descriptor_id` for a vector allocation of element stride
    /// `elem_size`: it must be `Trivial`, or a `Vector` with a non-empty
    /// pointer-offset list and a matching `elem_size`. The sizes must agree
    /// because the GC strides the data region by the descriptor's `elem_size`,
    /// so a mismatch would trace past the allocation.
    fn check_vector_descriptor(
        &mut self,
        pc: usize,
        op: &str,
        descriptor_id: DescriptorId,
        elem_size: u32,
    ) {
        match self
            .provider
            .descriptor(descriptor_id)
            .map(|desc| desc.inner())
        {
            None => self.err(
                Some(pc),
                format!("{}: unknown descriptor_id {}", op, descriptor_id),
            ),
            Some(ObjectDescriptorInner::Vector {
                elem_size: descriptor_elem_size,
                elem_pointer_offsets,
            }) => {
                if elem_pointer_offsets.is_empty() {
                    self.err(
                        Some(pc),
                        format!(
                            "{}: descriptor_id {} is not a non-empty Vector or Trivial",
                            op, descriptor_id
                        ),
                    );
                } else if *descriptor_elem_size != elem_size {
                    self.err(
                        Some(pc),
                        format!(
                            "{}: elem_size {} does not match Vector descriptor_id {} elem_size {}",
                            op, elem_size, descriptor_id, descriptor_elem_size
                        ),
                    );
                }
            },
            Some(ObjectDescriptorInner::Trivial) => {},
            Some(
                ObjectDescriptorInner::Closure
                | ObjectDescriptorInner::Struct { .. }
                | ObjectDescriptorInner::Enum { .. }
                | ObjectDescriptorInner::CapturedData { .. },
            ) => self.err(
                Some(pc),
                format!(
                    "{}: descriptor_id {} is not a non-empty Vector or Trivial",
                    op, descriptor_id
                ),
            ),
        }
    }

    /// Check that `descriptor_id` resolves and its variant satisfies `pred`.
    /// `op` names the calling micro-op and `expected` names the expected
    /// variant, both for the error message.
    fn check_descriptor_variant(
        &mut self,
        pc: usize,
        op: &str,
        descriptor_id: DescriptorId,
        pred: impl FnOnce(&ObjectDescriptorInner) -> bool,
        expected: &str,
    ) {
        match self.provider.descriptor(descriptor_id) {
            None => self.err(
                Some(pc),
                format!("{}: unknown descriptor_id {}", op, descriptor_id),
            ),
            Some(desc) if !pred(desc.inner()) => self.err(
                Some(pc),
                format!(
                    "{}: descriptor_id {} is not {}",
                    op, descriptor_id, expected
                ),
            ),
            Some(_) => {},
        }
    }

    /// `EnumNew` allocates an enum object and stamps `tag` into it. The
    /// descriptor must be an `Enum`, and `tag` must name one of its variants
    fn check_enum_new(&mut self, pc: usize, descriptor_id: DescriptorId, tag: u64) {
        match self.provider.descriptor(descriptor_id) {
            None => self.err(
                Some(pc),
                format!("EnumNew: unknown descriptor_id {}", descriptor_id),
            ),
            Some(desc) => match desc.inner() {
                ObjectDescriptorInner::Enum {
                    variant_pointer_offsets,
                    ..
                } => {
                    let variant_count = variant_pointer_offsets.len();
                    if tag as usize >= variant_count {
                        self.err(
                            Some(pc),
                            format!(
                                "EnumNew: tag {} out of range (descriptor {} has {} variants)",
                                tag, descriptor_id, variant_count
                            ),
                        );
                    }
                },
                _ => self.err(
                    Some(pc),
                    format!("EnumNew: descriptor_id {} is not an Enum", descriptor_id),
                ),
            },
        }
    }

    // TODO(metering): validate branch gas fields are populated.
    fn check_jump(&mut self, pc: usize, target: CodeOffset) {
        let code_len = self.func.code.ops().len();
        if (target.0 as usize) >= code_len {
            self.err(
                Some(pc),
                format!(
                    "jump target {} out of bounds (code length {})",
                    target.0, code_len,
                ),
            );
        }
    }

    fn check_nonzero_size(&mut self, pc: usize, size: u32) {
        if size == 0 {
            self.err(Some(pc), "size must be > 0");
        }
    }

    /// Requires `offset + size` to fit in `u32`, so the access window
    /// `[offset, offset + size)` cannot wrap.
    fn check_ref_offset_size_no_overflow(&mut self, pc: usize, offset: u32, size: u32) {
        if offset.checked_add(size).is_none() {
            self.err(
                Some(pc),
                format!("offset {} + size {} overflows u32", offset, size),
            );
        }
    }
}

/// Ops after which the dispatch loop does not fall through to `pc + 1`
/// within this frame: they leave the function or set `pc` to a target.
fn is_terminator(op: &MicroOp) -> bool {
    matches!(
        op,
        MicroOp::Return | MicroOp::Abort { .. } | MicroOp::AbortMsg { .. } | MicroOp::Jump { .. }
    )
}
