// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Gas cost schedule for the stackless execution IR.
//!
//! A cost is either fixed (constant, e.g. arithmetic and branches) or
//! size-dependent (scaling with a type's byte size, e.g. moves and reference
//! reads/writes). Every cost is affine in type sizes — `base + Σ coeff * size(T)`
//! — which lets costing split in two:
//!
//! - [`instrument`] sums each block into a [`BlockCost`] formula. The size
//!   terms stay symbolic (types may be polymorphic).
//! - [`CostResolver::resolve_block_cost`] evaluates a formula for a concrete
//!   instantiation.
//!
//! The costs over-approximate the work, and the numbers are placeholders.
//!
//! A cost may also have a runtime-dependent component, knowable only during
//! execution (e.g. the IO of a global-storage operation, or the size of the
//! deep-copies). Only the fixed and size-dependent parts are computed here; the
//! runtime charges the rest.
//!
//! TODO(metering): the deep-copy component of heap-backed copies/reads is uncharged
//! — these arms charge only the shallow byte move.

use crate::{
    lower::context::concrete_type_size,
    stackless_exec_ir::{
        instr_utils::{clobbers_transfer, for_each_value_use},
        BasicBlock, Instr, ModuleIR, NamedSlot,
    },
};
use mono_move_core::{
    types::{strip_ref, view_type, view_type_list, InternedType, InternedTypeList, Type},
    ExecutionErrorKind, Interner, IntoExecutionError, LayoutProvider, PreparedModule,
    VMInternalError, VMResult,
};
use move_binary_format::file_format::FieldHandleIndex;
use smallvec::SmallVec;
use thiserror::Error;

#[derive(Debug, Error)]
enum GasInstrumentationError {
    #[error("expected a reference type")]
    ExpectedReferenceType,

    #[error("Transfer({transfer}) read without a prior call-return binding")]
    TransferReadWithoutBinding { transfer: u16 },

    #[error("empty field chain")]
    EmptyFieldChain,

    #[error("field owner is not a struct type")]
    FieldOwnerNotStruct,

    #[error("variant owner is not an enum type")]
    VariantOwnerNotEnum,

    #[error("enum definition not found")]
    EnumDefinitionNotFound,

    #[error("type is not an enum")]
    NotAnEnum,

    #[error("call return {ret_idx} has no matching signature type")]
    CallReturnNoSignatureType { ret_idx: usize },

    #[error("CallClosure signature must start with a Function type")]
    ClosureSignatureNotFunction,
}

impl IntoExecutionError for GasInstrumentationError {
    fn kind(&self) -> ExecutionErrorKind {
        use GasInstrumentationError::*;
        match self {
            ExpectedReferenceType
            | TransferReadWithoutBinding { .. }
            | EmptyFieldChain
            | FieldOwnerNotStruct
            | VariantOwnerNotEnum
            | EnumDefinitionNotFound
            | NotAnEnum
            | CallReturnNoSignatureType { .. }
            | ClosureSignatureNotFunction => ExecutionErrorKind::InvariantViolation,
        }
    }
}

// --- Loads ---
const LD: u64 = 2;

// --- Data movement ---
const MOVE_BASE: u64 = 2;
const MOVE_PER_BYTE: u64 = 3;

// --- Operators ---
/// Any unary or binary operator: arithmetic, bitwise, shift, negate,
/// comparison, or boolean.
const OP: u64 = 5;

// --- Structs / variants ---
/// Variant pack/unpack.
const PACK_UNPACK: u64 = 8;
const TEST_VARIANT: u64 = 4;

// --- References ---
const BORROW: u64 = 2;
const REF_RW_BASE: u64 = 2;
const REF_RW_PER_BYTE: u64 = 3;
/// A field read whose size isn't known here (variant forms).
const FIELD_READ: u64 = 5;

// --- Globals ---
const GLOBAL: u64 = 15;

// --- Calls ---
/// Dispatch only; argument and return moves are charged separately.
const CALL: u64 = 10;
const PACK_CLOSURE: u64 = 9;

// --- Vector ---
const VEC_NEW: u64 = 10;
const VEC_LEN: u64 = 2;
const VEC_BORROW: u64 = 3;
const VEC_ELEM_BASE: u64 = 4;
const VEC_ELEM_PER_BYTE: u64 = 3;

// --- Control flow ---
const RETURN: u64 = 2;
const ABORT: u64 = 2;
const ABORT_MSG: u64 = 5;
const JUMP: u64 = 2;
const COND_JUMP: u64 = 3;
const FORCE_GC: u64 = 100;

// =============================================================================
// Cost formula
// =============================================================================

/// A block's cost as an affine formula `base + Σ coeff * size(ty)`, left
/// unresolved until the instantiation's type sizes are known.
/// TODO(metering): consider merging terms, or canonicalizing.
#[derive(Clone)]
pub(crate) struct BlockCost {
    base: u64,
    terms: SmallVec<[(u64, InternedType); 2]>,
}

impl BlockCost {
    /// An empty formula (cost zero).
    fn zero() -> Self {
        Self {
            base: 0,
            terms: SmallVec::new(),
        }
    }

    /// Add a fixed cost.
    fn add_constant(&mut self, c: u64) {
        self.base += c;
    }

    /// Add a size-dependent cost `base + per_byte * size(ty)`.
    fn add_sized(&mut self, base: u64, per_byte: u64, ty: InternedType) {
        self.base += base;
        self.terms.push((per_byte, ty));
    }
}

// =============================================================================
// Emission: polymorphic IR -> per-block cost formulas
// =============================================================================

/// Emit a per-block [`BlockCost`] formula for every function in `module_ir`.
pub(crate) fn instrument<I: Interner>(module_ir: &mut ModuleIR, interner: &I) -> VMResult<()> {
    let ModuleIR { module, functions } = module_ir;
    for func in functions.iter_mut().flatten() {
        let mut emitter = Emitter {
            module,
            interner,
            home_slot_types: &func.home_slot_types,
            transfer_ret_types: vec![None; func.num_transfer_positions as usize],
        };
        // Indexed by block label (dense `0..blocks.len()`).
        let mut costs = vec![BlockCost::zero(); func.blocks.len()];
        for block in &func.blocks {
            costs[block.label.0 as usize] = emitter.block_cost(block)?;
        }
        func.block_costs = costs;
    }
    Ok(())
}

/// Builds cost formulas by walking the IR. Transfer slots carry no type in the IR,
/// so the emitter tracks the type flowing through each one.
struct Emitter<'a, I: Interner> {
    module: &'a PreparedModule,
    interner: &'a I,
    /// Type of each Home slot, indexed by Home slot id.
    home_slot_types: &'a [InternedType],
    /// Type bound to each Transfer position by the most recent call's returns.
    /// Block-local: reset per block and clobbered by every call.
    transfer_ret_types: Vec<Option<InternedType>>,
}

impl<I: Interner> Emitter<'_, I> {
    /// Type of the value in `slot`. A Transfer slot read here always holds a prior
    /// call's return (call arguments are costed from the callee signature).
    fn slot_ty(&self, slot: NamedSlot) -> VMResult<InternedType> {
        match slot {
            NamedSlot::Home(i) => Ok(self.home_slot_types[i.0 as usize]),
            NamedSlot::Transfer(j) => self.transfer_ret_types[j.0 as usize].ok_or_else(|| {
                VMInternalError::new(GasInstrumentationError::TransferReadWithoutBinding {
                    transfer: j.0,
                })
            }),
        }
    }

    /// Pointee type of a reference slot (`ReadRef`/`WriteRef` touch the pointee).
    fn pointee_ty(&self, ref_slot: NamedSlot) -> VMResult<InternedType> {
        strip_ref(self.slot_ty(ref_slot)?)
            .ok_or_else(|| VMInternalError::new(GasInstrumentationError::ExpectedReferenceType))
    }

    /// Type of struct field `fh`, with the owner nominal's type arguments applied.
    fn field_ty(&self, owner: InternedType, fh: FieldHandleIndex) -> VMResult<InternedType> {
        let Type::Nominal { ty_args, .. } = view_type(owner) else {
            return Err(VMInternalError::new(
                GasInstrumentationError::FieldOwnerNotStruct,
            ));
        };
        Ok(self
            .interner
            .subst_type(self.module.interned_field_type_at(fh), *ty_args)?)
    }

    /// Type of a fused field chain's terminal field (its last `path` step).
    /// The whole chain reaches one field, so its read/write cost scales with
    /// that field's size — exactly like the single-field op.
    fn chain_terminal_ty(
        &self,
        path: &[(InternedType, FieldHandleIndex)],
    ) -> VMResult<InternedType> {
        let (owner, fh) = path
            .last()
            .ok_or(GasInstrumentationError::EmptyFieldChain)?;
        self.field_ty(*owner, *fh)
    }

    /// Field types of enum `enum_ty`'s variant `variant`, with the enum's type
    /// arguments applied.
    fn variant_field_tys(
        &self,
        enum_ty: InternedType,
        variant: u16,
    ) -> VMResult<Vec<InternedType>> {
        let Type::Nominal { name, ty_args, .. } = view_type(enum_ty) else {
            return Err(VMInternalError::new(
                GasInstrumentationError::VariantOwnerNotEnum,
            ));
        };
        let def_idx = self
            .module
            .interned_nominal_type_def_idx(*name)
            .ok_or(GasInstrumentationError::EnumDefinitionNotFound)?;
        let fields = self
            .module
            .interned_variant_field_types_at(def_idx, variant)
            .ok_or(GasInstrumentationError::NotAnEnum)?;
        Ok(fields
            .iter()
            .map(|&f| self.interner.subst_type(f, *ty_args))
            .collect::<Result<Vec<_>, _>>()?)
    }

    /// Cost of the formula for one block, resetting Transfer tracking at its start.
    fn block_cost(&mut self, block: &BasicBlock<NamedSlot>) -> VMResult<BlockCost> {
        // Transfer slots are block-local.
        self.transfer_ret_types.fill(None);
        let mut b = BlockCost::zero();
        for instr in &block.instrs {
            self.instr_cost(&mut b, instr)?;
            self.advance_transfer_tracking(instr);
        }
        Ok(b)
    }

    /// After costing `instr`, drop the Transfer slots it consumed. Calls manage
    /// their own return bindings in [`Self::call_cost`].
    fn advance_transfer_tracking(&mut self, instr: &Instr<NamedSlot>) {
        if clobbers_transfer(instr) {
            return;
        }
        for_each_value_use(instr, |s| {
            if let NamedSlot::Transfer(j) = s {
                self.transfer_ret_types[j.0 as usize] = None;
            }
        });
    }

    /// Emit the cost of `instr` into `b`.
    fn instr_cost(&mut self, b: &mut BlockCost, instr: &Instr<NamedSlot>) -> VMResult<()> {
        match instr {
            // --- Loads ---
            Instr::LdConst { .. } | Instr::LdImm { .. } => b.add_constant(LD),

            // --- Slot ops ---
            Instr::Copy { src, .. } | Instr::Move { src, .. } => {
                b.add_sized(MOVE_BASE, MOVE_PER_BYTE, self.slot_ty(*src)?)
            },

            // --- Unary / Binary ---
            Instr::UnaryOp { .. } | Instr::BinaryOp { .. } | Instr::BinaryOpImm { .. } => {
                b.add_constant(OP)
            },

            // --- Structs ---
            Instr::Pack { struct_ty, .. } | Instr::Unpack { struct_ty, .. } => {
                b.add_sized(MOVE_BASE, MOVE_PER_BYTE, *struct_ty)
            },

            // --- Enums ---
            Instr::PackVariant {
                enum_ty, variant, ..
            }
            | Instr::UnpackVariant {
                enum_ty, variant, ..
            } => {
                b.add_constant(PACK_UNPACK);
                for field_ty in self.variant_field_tys(*enum_ty, *variant)? {
                    b.add_sized(MOVE_BASE, MOVE_PER_BYTE, field_ty);
                }
            },
            Instr::TestVariant { .. } => b.add_constant(TEST_VARIANT),

            // --- References ---
            Instr::ImmBorrowLoc { .. }
            | Instr::MutBorrowLoc { .. }
            | Instr::ImmBorrowField { .. }
            | Instr::MutBorrowField { .. }
            | Instr::ImmBorrowVariantField { .. }
            | Instr::MutBorrowVariantField { .. } => b.add_constant(BORROW),
            Instr::ReadRef { src, .. } => {
                b.add_sized(REF_RW_BASE, REF_RW_PER_BYTE, self.pointee_ty(*src)?)
            },
            Instr::WriteRef { dst_ref, .. } => {
                b.add_sized(REF_RW_BASE, REF_RW_PER_BYTE, self.pointee_ty(*dst_ref)?)
            },

            // --- Fused field access (borrow + read/write) ---
            Instr::ReadField {
                owner_ty, field, ..
            } => b.add_sized(
                REF_RW_BASE,
                REF_RW_PER_BYTE,
                self.field_ty(*owner_ty, *field)?,
            ),
            Instr::WriteField { val, .. } => {
                b.add_sized(REF_RW_BASE, REF_RW_PER_BYTE, self.slot_ty(*val)?)
            },
            Instr::ReadVariantField { .. } => b.add_constant(FIELD_READ),
            Instr::WriteVariantField { val, .. } => {
                b.add_sized(REF_RW_BASE, REF_RW_PER_BYTE, self.slot_ty(*val)?)
            },

            // --- Fused inline-struct field access ---
            Instr::ImmBorrowLocField { .. } | Instr::MutBorrowLocField { .. } => {
                b.add_constant(BORROW)
            },
            Instr::ReadLocField {
                owner_ty, field, ..
            } => b.add_sized(MOVE_BASE, MOVE_PER_BYTE, self.field_ty(*owner_ty, *field)?),
            Instr::WriteLocField { val, .. } => {
                b.add_sized(MOVE_BASE, MOVE_PER_BYTE, self.slot_ty(*val)?)
            },

            // --- Fused field chains: charged exactly what the pre-fusion
            // sequence charges, so fusing never changes a program's gas.
            // TODO(metering): emit gas from pre-optimization IR instead of
            // hand-replicating pre-fusion costs per fused op. ---
            Instr::ImmBorrowFieldChain { path, .. }
            | Instr::MutBorrowFieldChain { path, .. }
            | Instr::ImmBorrowLocFieldChain { path, .. }
            | Instr::MutBorrowLocFieldChain { path, .. } => {
                b.add_constant(BORROW * path.len() as u64)
            },
            Instr::ReadFieldChain { path, .. } | Instr::ReadLocFieldChain { path, .. } => {
                b.add_constant(BORROW * (path.len() as u64).saturating_sub(1));
                b.add_sized(REF_RW_BASE, REF_RW_PER_BYTE, self.chain_terminal_ty(path)?);
            },
            Instr::WriteFieldChain { path, val, .. }
            | Instr::WriteLocFieldChain { path, val, .. } => {
                b.add_constant(BORROW * (path.len() as u64).saturating_sub(1));
                b.add_sized(REF_RW_BASE, REF_RW_PER_BYTE, self.slot_ty(*val)?);
            },

            // --- Globals ---
            Instr::Exists { .. }
            | Instr::MoveFrom { .. }
            | Instr::MoveTo { .. }
            | Instr::ImmBorrowGlobal { .. }
            | Instr::MutBorrowGlobal { .. } => b.add_constant(GLOBAL),

            // --- Calls ---
            Instr::Call { data } => {
                let sig = self.module.function_signature_at(data.function_handle);
                let params = self.interner.subst_type_list(sig.params, data.ty_args)?;
                let returns = self.interner.subst_type_list(sig.returns, data.ty_args)?;
                self.call_cost(b, params, &data.rets, returns)?;
            },

            // --- Closures ---
            Instr::PackClosure { data } => {
                b.add_constant(PACK_CLOSURE);
                for arg in &data.captured {
                    b.add_sized(MOVE_BASE, MOVE_PER_BYTE, self.slot_ty(*arg)?);
                }
            },
            Instr::CallClosure { data } => {
                let (params, returns) = closure_signature(data.closure_ty)?;
                self.call_cost(b, params, &data.rets, returns)?;
                // The closure value is passed as the last operand; charge its move.
                b.add_sized(MOVE_BASE, MOVE_PER_BYTE, data.closure_ty);
            },

            // --- Vector ---
            Instr::VecPack { srcs, .. } => {
                b.add_constant(VEC_NEW);
                for elem in srcs {
                    b.add_sized(MOVE_BASE, MOVE_PER_BYTE, self.slot_ty(*elem)?);
                }
            },
            Instr::VecLen { .. } => b.add_constant(VEC_LEN),
            Instr::VecImmBorrow { .. } | Instr::VecMutBorrow { .. } => b.add_constant(VEC_BORROW),
            Instr::VecPushBack { elem_ty, .. } | Instr::VecPopBack { elem_ty, .. } => {
                b.add_sized(VEC_ELEM_BASE, VEC_ELEM_PER_BYTE, *elem_ty)
            },
            Instr::VecUnpack { dsts, elem_ty, .. } => {
                let n = dsts.len() as u64;
                b.add_constant(VEC_NEW);
                b.add_sized(n * MOVE_BASE, n * MOVE_PER_BYTE, *elem_ty);
            },
            Instr::VecSwap { elem_ty, .. } => {
                // 2x per-element: two element accesses.
                b.add_sized(2 * VEC_ELEM_BASE, 2 * VEC_ELEM_PER_BYTE, *elem_ty);
            },

            // --- Control flow ---
            Instr::Branch { .. } => b.add_constant(JUMP),
            Instr::BrTrue { .. }
            | Instr::BrFalse { .. }
            | Instr::BrCmp { .. }
            | Instr::BrCmpImm { .. } => b.add_constant(COND_JUMP),
            Instr::Ret { srcs } => {
                b.add_constant(RETURN);
                // 2x per slot upper-bounds the cycle-breaking scratch moves
                // `emit_parallel_copy` adds for a cyclic (e.g. swap-style) return.
                for slot in srcs {
                    b.add_sized(2 * MOVE_BASE, 2 * MOVE_PER_BYTE, self.slot_ty(*slot)?);
                }
            },
            Instr::Abort { .. } => b.add_constant(ABORT),
            Instr::AbortMsg { .. } => b.add_constant(ABORT_MSG),

            Instr::ForceGC => b.add_constant(FORCE_GC),
        }
        Ok(())
    }

    /// Dispatch, a move per argument (sized from the callee signature), and a
    /// move per Home-slot return. Also binds the call's Transfer returns.
    fn call_cost(
        &mut self,
        b: &mut BlockCost,
        param_types: InternedTypeList,
        ret_slots: &[NamedSlot],
        ret_types: InternedTypeList,
    ) -> VMResult<()> {
        b.add_constant(CALL);
        for &param in view_type_list(param_types) {
            b.add_sized(MOVE_BASE, MOVE_PER_BYTE, param);
        }
        for ret in ret_slots {
            match *ret {
                NamedSlot::Transfer(_) => {
                    // Placed without a copy.
                },
                NamedSlot::Home(i) => {
                    b.add_sized(MOVE_BASE, MOVE_PER_BYTE, self.home_slot_types[i.0 as usize])
                },
            }
        }
        self.bind_call_returns(ret_slots, ret_types)?;
        Ok(())
    }

    /// Clobber all Transfer slots, then bind each Transfer return to its callee type.
    fn bind_call_returns(
        &mut self,
        ret_slots: &[NamedSlot],
        ret_types: InternedTypeList,
    ) -> VMResult<()> {
        self.transfer_ret_types.fill(None);
        let ret_types = view_type_list(ret_types);
        for (k, ret) in ret_slots.iter().enumerate() {
            if let NamedSlot::Transfer(j) = *ret {
                let ty = *ret_types
                    .get(k)
                    .ok_or(GasInstrumentationError::CallReturnNoSignatureType { ret_idx: k })?;
                self.transfer_ret_types[j.0 as usize] = Some(ty);
            }
        }
        Ok(())
    }
}

/// The `(param types, result types)` of a closure's `Function` type.
fn closure_signature(closure_ty: InternedType) -> VMResult<(InternedTypeList, InternedTypeList)> {
    match view_type(closure_ty) {
        Type::Function { args, results, .. } => Ok((*args, *results)),
        _ => Err(VMInternalError::new(
            GasInstrumentationError::ClosureSignatureNotFunction,
        )),
    }
}

// =============================================================================
// Resolution: cost formula -> concrete gas
// =============================================================================

/// Resolves the types in a [`BlockCost`] to concrete sizes for an instantiation.
pub(crate) trait CostResolver {
    /// Substitute the instantiation's type arguments into `ty`.
    fn concrete_ty(&self, ty: InternedType) -> VMResult<InternedType>;

    /// Value layouts, for reading concrete type sizes.
    fn layouts(&self) -> &dyn LayoutProvider;

    /// Evaluate a block's cost formula to concrete gas for this instantiation.
    fn resolve_block_cost(&self, cost: &BlockCost) -> VMResult<u64> {
        let mut total = cost.base;
        for &(coeff, ty) in &cost.terms {
            let concrete_ty = self.concrete_ty(ty)?;
            let size = concrete_type_size(self.layouts(), concrete_ty, "cost term type")?;
            total += coeff * size as u64;
        }
        Ok(total)
    }
}
