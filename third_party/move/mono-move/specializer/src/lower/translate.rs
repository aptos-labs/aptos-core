// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Lowers stackless exec IR to micro-ops.

use super::{
    context::{
        concrete_type_size, resolve_variant_field_access, CallSiteInfo, LoweringContext, TypedSlot,
        VariantFieldAccess,
    },
    gc_layout::type_pointer_offsets,
    lower_utils::ranges_overlap,
    parallel_copy, LoweringError,
};
use crate::{
    gas::{self, CostResolver},
    stackless_exec_ir::{
        instr_utils::{clobbers_xfer, for_each_value_use, is_fallthrough_terminator},
        BinaryOp, FunctionIR, ImmValue, Instr, Label, Slot, UnaryOp,
    },
};
use mono_move_core::{
    native::{FrameSlot, NativeABI},
    types::{is_closed_type, strip_ref, view_type, view_type_list, InternedType, Type},
    CallClosureOp, ClosureFuncRef, CmpKind, CodeOffset, DescriptorId, FrameLayoutInfo, FrameOffset,
    IntBinaryOp, IntCastOp, IntCmpOp, IntNegateOp, IntOperand, IntShiftOp, IntTy, JumpIntCmpOp,
    JumpValueCmpOp, JumpValueRefCmpOp, LayoutKind, LayoutProvider, MicroOp, PackClosureOp,
    SafePointEntry, ShiftOperand, SizedSlot, VMInternalError, VMResult, ValueCmpOp, ValueRefCmpOp,
    VecPackOp, VecUnpackOp, FRAME_METADATA_SIZE,
};
use move_binary_format::file_format::{
    ConstantPoolIndex, FieldHandleIndex, VariantFieldHandleIndex,
};

/// Validates that a primitive constant's BCS bytes are exactly `N` wide and
/// returns them as a fixed array. Fixed-width integers and `address` encode
/// with no length prefix, so these bytes are already the in-memory
/// representation the matching `StoreImm` expects.
fn const_imm<const N: usize>(idx: ConstantPoolIndex, bytes: &[u8]) -> VMResult<[u8; N]> {
    bytes.try_into().map_err(|_| {
        VMInternalError::new(LoweringError::LdConstBadLength {
            idx: idx.0,
            expected: N,
            got: bytes.len(),
        })
    })
}

/// Temporary result of lowering a function to micro-ops.
pub(super) struct LoweredFunction {
    pub code: Vec<MicroOp>,
    /// Static cost of the entry basic block.
    pub entry_gas: u64,
    /// One entry **per allocating micro-op only**, in code-offset order.
    /// Non-allocating PCs are not represented; the vector is sparse.
    /// Each entry's `code_offset` indexes directly into `code`.
    pub safe_points: Vec<SafePointEntry>,
}

/// TODO(cleanup): move this constant into a shared location and reuse elsewhere.
/// Byte width of an enum value in a frame slot: an 8-byte heap pointer.
const ENUM_PTR_SIZE: u32 = 8;

/// Lower a slot-allocated function to its micro-op form.
pub(super) fn lower_function(
    func_ir: &FunctionIR,
    ctx: &LoweringContext,
) -> VMResult<LoweredFunction> {
    let mut state = LoweringState::new(func_ir, ctx);
    for (block_idx, block) in func_ir.blocks.iter().enumerate() {
        // Xfer slots are block-local.
        debug_assert!(
            state.xfer_bindings.iter().all(Option::is_none),
            "xfer_bindings not fully cleared at block boundary",
        );
        debug_assert!(
            state.pending_def_binds.is_empty(),
            "pending_def_binds not committed at block boundary",
        );
        state.xfer_bindings.fill(None);
        state.label_map[block.label.0 as usize] = Some(state.out_buf.len() as u32);
        // Resolve this block's cost formula to concrete gas for this instantiation.
        state.block_costs[block.label.0 as usize] =
            state.resolve_block_cost(&func_ir.block_costs[block.label.0 as usize])?;
        for instr in &block.instrs {
            state.lower_instr(func_ir, instr)?;
            state.commit_xfer_bindings_after(instr);
        }
        // Record a conditional branch's fallthrough block (the next in emission
        // order) on its fixup, so `fixup_branches` can write that block's cost
        // into the jump's `gas_fallthrough`.
        if block.instrs.last().is_some_and(is_fallthrough_terminator) {
            let fallthrough = func_ir
                .blocks
                .get(block_idx + 1)
                .ok_or(LoweringError::FinalBlockNoFallthrough)?;
            let fixup = state
                .branch_fixups
                .last_mut()
                .expect("conditional terminator must have pushed a branch fixup");
            fixup.fallthrough_label = Some(fallthrough.label);
        }
    }
    state.fixup_branches()?;
    let entry_gas = func_ir
        .blocks
        .first()
        .map(|entry| state.block_costs[entry.label.0 as usize])
        .unwrap_or_default();
    Ok(LoweredFunction {
        code: state.out_buf,
        entry_gas,
        safe_points: state.pending_safe_points,
    })
}

/// A branch micro-op awaiting target resolution and gas fill-in.
struct BranchFixup {
    /// Index in `out_buf` of the branch micro-op.
    idx: usize,
    /// For a conditional branch, the label of the fallthrough block (the next
    /// block in emission order). `None` for an unconditional branch.
    fallthrough_label: Option<Label>,
}

struct LoweringState<'a> {
    /// Read-only frame layout for the function being lowered.
    ctx: &'a LoweringContext<'a>,
    /// Output buffer. Micro-ops are appended in emit order.
    out_buf: Vec<MicroOp>,
    /// `Label(i)` → index in `out_buf` where block `i` begins. Dense
    /// (one entry per block); `None` until that block has been lowered.
    /// Read by `fixup_branches` to resolve branch targets after all
    /// blocks are emitted.
    label_map: Vec<Option<u32>>,
    /// `Label(i)` → static gas cost of block `i`, summed as the block lowers.
    /// Dense (one entry per block). Once the block loop has costed every block,
    /// `lower_function` reads it: `fixup_branches` copies each cost into the
    /// jumps targeting that block, and block 0's cost becomes `entry_gas`.
    block_costs: Vec<u64>,
    /// Branch micro-ops emitted with a placeholder label encoding (and
    /// placeholder gas). `fixup_branches` walks this list and rewrites each
    /// target to the real micro-op index from `label_map`, and fills in the
    /// jump's gas fields from the per-block costs.
    branch_fixups: Vec<BranchFixup>,
    /// Monotonic cursor into `ctx.call_sites`. Before a call is lowered,
    /// it points at *that* call's `CallSiteInfo`; immediately after the
    /// `CallFunc` op is emitted, it advances by one.
    call_site_cursor: usize,
    /// Monotonic cursor into `ctx.closure_pack_sites`; advances once per
    /// lowered `Instr::PackClosure`.
    closure_pack_cursor: usize,
    /// Monotonic cursor into `ctx.closure_call_sites`; advances once per
    /// lowered `Instr::CallClosure`.
    closure_call_cursor: usize,
    /// Types of the function IR's home (frame-resident) slots with the
    /// instantiation's type arguments substituted, indexed by Home slot id.
    home_slot_types: &'a [InternedType],
    /// `Some(TypedSlot)` while `Slot::Xfer(j)` holds a fully-written
    /// live value visible to the GC; `None` otherwise. Length
    /// `ctx.num_xfer_positions`.
    xfer_bindings: Vec<Option<TypedSlot>>,
    /// Xfer bindings staged by the current IR instruction's defs and
    /// committed by `commit_xfer_bindings_after`. Each tuple is
    /// `(j, typed_slot)`: the Xfer position `j` and the value bound there.
    pending_def_binds: Vec<(u16, TypedSlot)>,
    /// Safe-point entries in code-offset order. Populated by `emit`
    /// when `op.is_allocating()`.
    pending_safe_points: Vec<SafePointEntry>,
}

impl gas::CostResolver for LoweringState<'_> {
    fn concrete_ty(&self, ty: InternedType) -> VMResult<InternedType> {
        LoweringState::concrete_ty(self, ty)
    }

    fn layouts(&self) -> &dyn LayoutProvider {
        self.ctx.layouts
    }
}

impl<'a> LoweringState<'a> {
    fn new(func_ir: &'a FunctionIR, ctx: &'a LoweringContext<'a>) -> Self {
        let num_xfer_positions = ctx.num_xfer_positions as usize;
        LoweringState {
            ctx,
            out_buf: Vec::new(),
            label_map: vec![None; func_ir.blocks.len()],
            block_costs: vec![0u64; func_ir.blocks.len()],
            branch_fixups: Vec::new(),
            call_site_cursor: 0,
            closure_pack_cursor: 0,
            closure_call_cursor: 0,
            home_slot_types: view_type_list(ctx.home_types),
            xfer_bindings: vec![None; num_xfer_positions],
            pending_def_binds: Vec::new(),
            pending_safe_points: Vec::new(),
        }
    }

    /// Substitutes the instantiation's type arguments into `ty`, producing a
    /// closed type; identity when `ty_args` is empty.
    fn concrete_ty(&self, ty: InternedType) -> VMResult<InternedType> {
        let ty = self.ctx.interner.subst_type(ty, self.ctx.ty_args)?;
        debug_assert!(
            is_closed_type(ty),
            "instruction-embedded type still open after substitution"
        );
        Ok(ty)
    }

    /// Per-field `(offset, size, align)` of a concrete struct type, read from
    /// its published value layout: byte offsets from the struct layout, and each
    /// field's size and alignment from its child layout. Errors if `struct_ty`
    /// has no published layout or is not a struct. `op` labels the caller.
    fn struct_field_layouts(
        &self,
        struct_ty: InternedType,
        op: &'static str,
    ) -> VMResult<Vec<(u32, u32, u32)>> {
        let layout = self
            .ctx
            .layouts
            .layout_by_ty(struct_ty)
            .ok_or(LoweringError::StructLayoutNotPopulated { op })?;
        let LayoutKind::Struct { fields } = &layout.kind else {
            return Err(VMInternalError::new(LoweringError::NominalTypeNotStruct {
                op,
            }));
        };
        fields
            .iter()
            .map(|f| {
                let child = self
                    .ctx
                    .layouts
                    .layout(f.id)
                    .ok_or(LoweringError::FieldLayoutIdUnresolved { op })?;
                Ok((f.offset, child.size, child.align))
            })
            .collect()
    }

    /// Resolve a `FieldHandleIndex` against the struct type `struct_ty`
    /// and return `(field_byte_offset, field_byte_size)`.
    fn resolve_field(&self, struct_ty: InternedType, fh: FieldHandleIndex) -> VMResult<(u32, u32)> {
        let fields = self.struct_field_layouts(struct_ty, "field access")?;
        let pos = self.ctx.module.field_position_at(fh) as usize;
        let (offset, size, _align) = *fields
            .get(pos)
            .ok_or(LoweringError::FieldIndexOutOfRange { pos })?;
        Ok((offset, size))
    }

    /// Resolve a `VariantFieldHandleIndex` against `enum_ty`'s derived
    /// layout into a [`VariantFieldAccess`].
    fn resolve_variant_field(
        &self,
        enum_ty: InternedType,
        vfh: VariantFieldHandleIndex,
    ) -> VMResult<VariantFieldAccess> {
        resolve_variant_field_access(self.ctx.module, &self.ctx.enum_layouts, enum_ty, vfh)
    }

    /// Emit the borrow of a variant field's reference into `dst_ref`. Uses the
    /// static `enum_borrow` when the offset is variant-independent, else the
    /// tag-dispatched `EnumBorrowVariantField` (which also aborts when the
    /// runtime variant does not declare the field).
    fn emit_variant_field_borrow(
        &mut self,
        access: &VariantFieldAccess,
        enum_ref: FrameOffset,
        dst_ref: FrameOffset,
    ) -> VMResult<()> {
        match access.uniform_offset {
            Some(offset) => self.emit(MicroOp::enum_borrow(enum_ref, offset, dst_ref))?,
            None => self.emit(MicroOp::enum_borrow_variant_field(
                enum_ref,
                &access.offsets,
                dst_ref,
            ))?,
        }
        Ok(())
    }

    fn xfer_binding(&self, j: u16) -> VMResult<TypedSlot> {
        self.xfer_bindings[j as usize]
            .ok_or_else(|| VMInternalError::new(LoweringError::XferReadWithoutDef { xfer: j }))
    }

    fn slot(&self, slot: Slot) -> VMResult<SizedSlot> {
        Ok(match slot {
            Slot::Home(i) => self.ctx.home_slots[i as usize],
            Slot::Xfer(j) => self.xfer_binding(j)?.slot,
            Slot::Vid(_) => return Err(VMInternalError::new(LoweringError::VidInPostAllocationIr)),
        })
    }

    /// Returns sized layout info for a destination slot.
    fn def_slot(&mut self, slot: Slot) -> VMResult<SizedSlot> {
        Ok(self.def_typed_slot(slot)?.slot)
    }

    /// Resolves a destination slot to its sized layout and value type. For
    /// `Slot::Xfer(j)`, stages a pending binding to arg position `j` of the
    /// upcoming call. Errors for `Slot::Vid`.
    fn def_typed_slot(&mut self, slot: Slot) -> VMResult<TypedSlot> {
        Ok(match slot {
            Slot::Home(i) => TypedSlot {
                slot: self.ctx.home_slots[i as usize],
                ty: self.home_slot_types[i as usize],
            },
            Slot::Xfer(j) => {
                let call_site = &self.ctx.call_sites[self.call_site_cursor];
                let typed_slot = call_site.arg_slots[j as usize];
                self.pending_def_binds.push((j, typed_slot));
                typed_slot
            },
            Slot::Vid(_) => return Err(VMInternalError::new(LoweringError::VidInPostAllocationIr)),
        })
    }

    /// Resolves each `slot` to its [`SizedSlot`] frame layout.
    fn slots_to_sized_slots(&self, slots: &[Slot]) -> VMResult<Vec<SizedSlot>> {
        slots.iter().map(|slot| self.slot(*slot)).collect()
    }

    /// Place a call's return values; `ret_slots` are their caller-frame
    /// locations. The call clobbers the whole callee region, so clear all Xfer
    /// bindings, then re-bind each `Xfer` ret (for GC) and copy each `Home` in.
    fn bind_call_returns(&mut self, rets: &[Slot], ret_slots: &[TypedSlot]) -> VMResult<()> {
        self.xfer_bindings.fill(None);
        for (k, ret_slot) in rets.iter().enumerate() {
            match *ret_slot {
                Slot::Xfer(j) => {
                    self.xfer_bindings[j as usize] = Some(ret_slots[k]);
                },
                Slot::Home(i) => {
                    let src = ret_slots[k].slot;
                    let dst = self.ctx.home_slots[i as usize];
                    self.emit_single_move(dst.offset, src)?;
                },
                Slot::Vid(_) => {
                    return Err(VMInternalError::new(LoweringError::VidInPostAllocationIr))
                },
            }
        }
        Ok(())
    }

    /// Append `op` to the output buffer. For allocating `op`s,
    /// also emit a paired `SafePointEntry` whose `code_offset` is
    /// `op`'s index in the buffer and whose `heap_ptr_offsets`
    /// are derived from the current `xfer_bindings`.
    fn emit(&mut self, op: MicroOp) -> VMResult<()> {
        if op.is_allocating() {
            let code_offset = CodeOffset(self.out_buf.len() as u32);
            let mut heap_ptr_offsets = Vec::with_capacity(self.xfer_bindings.len());
            for ts in self.xfer_bindings.iter().flatten() {
                let rels = type_pointer_offsets(self.ctx.layouts, ts.ty)?;
                heap_ptr_offsets
                    .extend(rels.into_iter().map(|r| FrameOffset(ts.slot.offset.0 + r)));
            }
            // TODO(cleanup): revisit the need to sort and dedup.
            heap_ptr_offsets.sort_by_key(|o| o.0);
            heap_ptr_offsets.dedup();
            self.pending_safe_points.push(SafePointEntry {
                code_offset,
                layout: FrameLayoutInfo::new(heap_ptr_offsets),
            });
        }
        self.out_buf.push(op);
        Ok(())
    }

    /// Record a fixup for the branch micro-op about to be emitted next, so
    /// `fixup_branches` can resolve its target and gas. Call immediately
    /// before the corresponding `emit`.
    fn record_branch_fixup(&mut self) {
        self.branch_fixups.push(BranchFixup {
            idx: self.out_buf.len(),
            fallthrough_label: None,
        });
    }

    /// Emits the non-allocating copy that boxes a `size`-byte value from the
    /// frame slot `src` into the heap object at.
    fn emit_heap_move_to(
        &mut self,
        heap_ptr: FrameOffset,
        src: FrameOffset,
        size: u32,
    ) -> VMResult<()> {
        if size == 8 {
            self.emit(MicroOp::HeapMoveTo8 {
                heap_ptr,
                offset: 0,
                src,
            })?;
        } else {
            self.emit(MicroOp::HeapMoveTo {
                heap_ptr,
                offset: 0,
                src,
                size,
            })?;
        }
        Ok(())
    }

    /// Emits the non-allocating copy that unboxes a `size`-byte value from the
    /// heap object at into frame slot `dst`.
    fn emit_heap_move_from(
        &mut self,
        dst: FrameOffset,
        heap_ptr: FrameOffset,
        size: u32,
    ) -> VMResult<()> {
        if size == 8 {
            self.emit(MicroOp::HeapMoveFrom8 {
                dst,
                heap_ptr,
                offset: 0,
            })?;
        } else {
            self.emit(MicroOp::HeapMoveFrom {
                dst,
                heap_ptr,
                offset: 0,
                size,
            })?;
        }
        Ok(())
    }

    /// `DescriptorId` published for `vec_ty`, with the shared diagnostic of
    /// the allocating vector ops. `op_name` prefixes the error message.
    fn vector_descriptor_id(
        &self,
        op_name: &'static str,
        vec_ty: InternedType,
    ) -> VMResult<DescriptorId> {
        self.ctx.descriptor_id(vec_ty).ok_or_else(|| {
            VMInternalError::new(LoweringError::VectorTypeNoDescriptor { op: op_name })
        })
    }

    /// Interned-type corresponding to `slot`.
    fn slot_interned_type(&self, slot: Slot) -> VMResult<InternedType> {
        Ok(match slot {
            Slot::Home(i) => self.home_slot_types[i as usize],
            Slot::Xfer(j) => self.xfer_binding(j)?.ty,
            Slot::Vid(_) => return Err(VMInternalError::new(LoweringError::VidInPostAllocationIr)),
        })
    }

    /// Canonical [`Type`] variant of `slot`. Use [`Self::slot_interned_type`]
    /// when an interned pointer is needed instead.
    fn slot_type(&self, slot: Slot) -> VMResult<&'static Type> {
        Ok(view_type(self.slot_interned_type(slot)?))
    }

    /// After an op materializes an owned value into a slot (`Copy`, `ReadRef`,
    /// `Read*Field`), append a `DeepCopyHeapPtrs` when the value is heap-backed so the
    /// byte copy does not alias the source's heap object (Move value semantics).
    /// `dst_ty` is the materialized value's type (from [`Self::def_typed_slot`]),
    /// `dst_off` its frame offset. A reference is shallow-copied (a copy of a
    /// reference is another reference); a value with no owned heap pointers
    /// (scalar, inline-scalar struct) needs nothing, keeping those hot paths a
    /// bare byte copy.
    fn maybe_deep_copy(&mut self, dst_ty: InternedType, dst_off: FrameOffset) -> VMResult<()> {
        if matches!(
            view_type(dst_ty),
            Type::ImmutRef { .. } | Type::MutRef { .. }
        ) {
            return Ok(());
        }
        let offsets = type_pointer_offsets(self.ctx.layouts, dst_ty)?;
        if !offsets.is_empty() {
            self.emit(MicroOp::deep_copy_heap_ptrs(
                dst_off,
                offsets.into_boxed_slice(),
            ))?;
        }
        Ok(())
    }

    /// Emit an `IntCast` to `to` from `src` into `dst`. The source type comes
    /// from `src`'s slot type and the `to` type is supplied by the caller.
    fn lower_cast(&mut self, dst: Slot, src: Slot, to: IntTy) -> VMResult<()> {
        let from =
            IntTy::from_type(self.slot_type(src)?).ok_or(LoweringError::CastSourceNotInteger)?;
        let src_info = self.slot(src)?;
        let dst_info = self.def_slot(dst)?;
        self.emit(MicroOp::IntCast(IntCastOp {
            from,
            to,
            dst: dst_info.offset,
            src: src_info.offset,
        }))
    }

    /// Size in bytes of `ref_slot`'s pointee.
    fn ref_pointee_size(&self, ref_slot: Slot) -> VMResult<u32> {
        super::context::ref_pointee_size(self.ctx.layouts, self.slot_interned_type(ref_slot)?)
    }

    /// Emit one byte-copy from `src` to `dst_offset`. Caller is
    /// responsible for ensuring no other concurrent move clobbers the
    /// source bytes.
    fn emit_single_move(&mut self, dst_offset: FrameOffset, src: SizedSlot) -> VMResult<()> {
        if dst_offset == src.offset {
            return Ok(());
        }
        if src.size == 8 {
            self.emit(MicroOp::Move8 {
                dst: dst_offset,
                src: src.offset,
            })?;
        } else {
            self.emit(MicroOp::Move {
                dst: dst_offset,
                src: src.offset,
                size: src.size,
            })?;
        }
        Ok(())
    }

    /// Emit a standalone comparison writing a 1-byte boolean to `dst`.
    fn emit_int_cmp(
        &mut self,
        cmp: CmpKind,
        dst: FrameOffset,
        lhs: FrameOffset,
        rhs: IntOperand,
    ) -> VMResult<()> {
        self.emit(MicroOp::IntCmp(IntCmpOp {
            op: cmp,
            dst,
            lhs,
            rhs,
        }))
    }

    /// Emit a fused compare-and-branch to the (encoded) `target`. The caller
    /// is responsible for pushing the branch-fixup index first.
    fn emit_jump_int_cmp(
        &mut self,
        target: CodeOffset,
        cmp: CmpKind,
        lhs: FrameOffset,
        rhs: IntOperand,
    ) -> VMResult<()> {
        self.emit(MicroOp::JumpIntCmp(JumpIntCmpOp {
            target,
            op: cmp,
            lhs,
            rhs,
            gas_taken: 0,
            gas_fallthrough: 0,
        }))
    }

    /// Lower one IR instruction.
    fn lower_instr(&mut self, func_ir: &FunctionIR, instr: &Instr) -> VMResult<()> {
        match instr {
            // --- Loads ---
            Instr::LdU64(dst, v) => {
                let dst_info = self.def_slot(*dst)?;
                self.emit(MicroOp::StoreImm8 {
                    dst: dst_info.offset,
                    imm: v.to_le_bytes(),
                })?;
            },
            Instr::LdTrue(dst) => {
                let dst_info = self.def_slot(*dst)?;
                self.emit(MicroOp::StoreImm1 {
                    dst: dst_info.offset,
                    imm: 1,
                })?;
            },
            Instr::LdFalse(dst) => {
                let dst_info = self.def_slot(*dst)?;
                self.emit(MicroOp::StoreImm1 {
                    dst: dst_info.offset,
                    imm: 0,
                })?;
            },
            // TODO(correctness): audit every integer-width use across the
            // specializer and confirm each is the correct width (slot sizes,
            // immediate widths, sign extension).
            // 1-byte integers store directly into their 1-byte slot.
            Instr::LdU8(dst, v) => {
                let dst_info = self.def_slot(*dst)?;
                self.emit(MicroOp::StoreImm1 {
                    dst: dst_info.offset,
                    imm: *v,
                })?;
            },
            // 2-/4-byte integers store directly into their 2-/4-byte slot.
            Instr::LdU16(dst, v) => {
                let dst_info = self.def_slot(*dst)?;
                self.emit(MicroOp::StoreImm2 {
                    dst: dst_info.offset,
                    imm: v.to_le_bytes(),
                })?;
            },
            Instr::LdU32(dst, v) => {
                let dst_info = self.def_slot(*dst)?;
                self.emit(MicroOp::StoreImm4 {
                    dst: dst_info.offset,
                    imm: v.to_le_bytes(),
                })?;
            },
            // 1-byte integers store directly into their 1-byte slot.
            Instr::LdI8(dst, v) => {
                let dst_info = self.def_slot(*dst)?;
                self.emit(MicroOp::StoreImm1 {
                    dst: dst_info.offset,
                    imm: *v as u8,
                })?;
            },
            // 2-/4-byte signed integers store their two's-complement LE bytes
            // directly into their 2-/4-byte slot.
            Instr::LdI16(dst, v) => {
                let dst_info = self.def_slot(*dst)?;
                self.emit(MicroOp::StoreImm2 {
                    dst: dst_info.offset,
                    imm: v.to_le_bytes(),
                })?;
            },
            Instr::LdI32(dst, v) => {
                let dst_info = self.def_slot(*dst)?;
                self.emit(MicroOp::StoreImm4 {
                    dst: dst_info.offset,
                    imm: v.to_le_bytes(),
                })?;
            },
            Instr::LdI64(dst, v) => {
                let dst_info = self.def_slot(*dst)?;
                self.emit(MicroOp::StoreImm8 {
                    dst: dst_info.offset,
                    imm: v.to_le_bytes(),
                })?;
            },
            Instr::LdU128(dst, v) => {
                let dst_info = self.def_slot(*dst)?;
                self.emit(MicroOp::StoreImm16 {
                    dst: dst_info.offset,
                    imm: Box::new(v.to_le_bytes()),
                })?;
            },
            Instr::LdI128(dst, v) => {
                let dst_info = self.def_slot(*dst)?;
                self.emit(MicroOp::StoreImm16 {
                    dst: dst_info.offset,
                    imm: Box::new(v.to_le_bytes()),
                })?;
            },
            Instr::LdU256(dst, v) => {
                let dst_info = self.def_slot(*dst)?;
                self.emit(MicroOp::StoreImm32 {
                    dst: dst_info.offset,
                    imm: Box::new(v.to_le_bytes()),
                })?;
            },
            Instr::LdI256(dst, v) => {
                let dst_info = self.def_slot(*dst)?;
                self.emit(MicroOp::StoreImm32 {
                    dst: dst_info.offset,
                    imm: Box::new(v.to_le_bytes()),
                })?;
            },
            Instr::LdConst(dst, idx) => {
                let ty = view_type(self.ctx.module.interned_constant_type_at(*idx));
                let bcs_bytes = self.ctx.module.constant_data_at(*idx);
                let dst_info = self.def_slot(*dst)?;
                // Constants store their value BCS-encoded. Fixed-width
                // integers encode as their raw little-endian bytes and
                // `address` as its 32 raw bytes, both with no length prefix,
                // so the constant data drops straight into the matching
                // `StoreImm`. Vectors are heap-allocated at runtime from the
                // constant pool, so they keep their own micro-op.
                //
                // TODO(correctness): revisit this when we fix the endianness
                // story for the VM.
                match ty {
                    Type::Bool | Type::U8 | Type::I8 => {
                        self.emit(MicroOp::StoreImm1 {
                            dst: dst_info.offset,
                            imm: const_imm::<1>(*idx, bcs_bytes)?[0],
                        })?;
                    },
                    Type::U16 | Type::I16 => {
                        self.emit(MicroOp::StoreImm2 {
                            dst: dst_info.offset,
                            imm: const_imm::<2>(*idx, bcs_bytes)?,
                        })?;
                    },
                    Type::U32 | Type::I32 => {
                        self.emit(MicroOp::StoreImm4 {
                            dst: dst_info.offset,
                            imm: const_imm::<4>(*idx, bcs_bytes)?,
                        })?;
                    },
                    Type::U64 | Type::I64 => {
                        self.emit(MicroOp::StoreImm8 {
                            dst: dst_info.offset,
                            imm: const_imm::<8>(*idx, bcs_bytes)?,
                        })?;
                    },
                    Type::U128 | Type::I128 => {
                        self.emit(MicroOp::StoreImm16 {
                            dst: dst_info.offset,
                            imm: Box::new(const_imm::<16>(*idx, bcs_bytes)?),
                        })?;
                    },
                    Type::U256 | Type::I256 | Type::Address => {
                        self.emit(MicroOp::StoreImm32 {
                            dst: dst_info.offset,
                            imm: Box::new(const_imm::<32>(*idx, bcs_bytes)?),
                        })?;
                    },
                    Type::Vector { .. } => {
                        self.emit(MicroOp::StoreImmVec {
                            dst: dst_info.offset,
                            idx: *idx,
                        })?;
                    },
                    // The bytecode verifier rejects constants of these types,
                    // so reaching them here is an invariant violation.
                    Type::Signer
                    | Type::ImmutRef { .. }
                    | Type::MutRef { .. }
                    | Type::Nominal { .. }
                    | Type::Function { .. }
                    | Type::TypeParam { .. } => {
                        return Err(VMInternalError::new(
                            LoweringError::LdConstTypeNotPermitted { idx: idx.0 },
                        ))
                    },
                }
            },

            // --- Copy/Move ---
            // `Move` transfers ownership: a byte copy is the whole operation.
            Instr::Move(dst, src) => {
                let src_info = self.slot(*src)?;
                let dst_info = self.def_slot(*dst)?;
                self.emit_single_move(dst_info.offset, src_info)?;
            },
            // `Copy` must produce an independent value: byte copy, then deep-copy
            // any owned heap pointers so a heap-backed value does not alias.
            Instr::Copy(dst, src) => {
                let src_info = self.slot(*src)?;
                let dst_info = self.def_typed_slot(*dst)?;
                self.emit_single_move(dst_info.slot.offset, src_info)?;
                self.maybe_deep_copy(dst_info.ty, dst_info.slot.offset)?;
            },

            // --- Binary ops ---
            Instr::BinaryOp(dst, op, lhs, rhs) => {
                // TODO(cleanup): BinaryOp / BinaryOpImm share most of their shape
                // (operand resolution + per-kind emit); consider factoring
                // out the common skeleton once cast / cmp ops settle.
                let lhs_info = self.slot(*lhs)?;
                let rhs_info = self.slot(*rhs)?;
                let dst_info = self.def_slot(*dst)?;
                let lhs_interned = self.slot_interned_type(*lhs)?;
                let lhs_ty = view_type(lhs_interned);
                let dst = dst_info.offset;
                let lhs = lhs_info.offset;
                let rhs = rhs_info.offset;

                // Fast path: emit a specialized u64 variant when one exists.
                // u64 `Cmp(_)` / `Or` / `And` have no specialized variant
                // and fall through to the unspecialized path.
                let mut handled = false;
                if lhs_ty.is_u64() {
                    let emitted = match op {
                        BinaryOp::Add => Some(MicroOp::AddU64 { dst, lhs, rhs }),
                        BinaryOp::Sub => Some(MicroOp::SubU64 { dst, lhs, rhs }),
                        BinaryOp::Mul => Some(MicroOp::MulU64 { dst, lhs, rhs }),
                        BinaryOp::Div => Some(MicroOp::DivU64 { dst, lhs, rhs }),
                        BinaryOp::Mod => Some(MicroOp::ModU64 { dst, lhs, rhs }),
                        BinaryOp::BitAnd => Some(MicroOp::BitAndU64 { dst, lhs, rhs }),
                        BinaryOp::BitOr => Some(MicroOp::BitOrU64 { dst, lhs, rhs }),
                        BinaryOp::BitXor => Some(MicroOp::BitXorU64 { dst, lhs, rhs }),
                        BinaryOp::Shl => Some(MicroOp::ShlU64 { dst, lhs, rhs }),
                        BinaryOp::Shr => Some(MicroOp::ShrU64 { dst, lhs, rhs }),
                        BinaryOp::Cmp(_) | BinaryOp::Or | BinaryOp::And => None,
                    };
                    if let Some(micro) = emitted {
                        self.emit(micro)?;
                        handled = true;
                    }
                }

                if !handled {
                    match op {
                        BinaryOp::Add
                        | BinaryOp::Sub
                        | BinaryOp::Mul
                        | BinaryOp::Div
                        | BinaryOp::Mod
                        | BinaryOp::BitAnd
                        | BinaryOp::BitOr
                        | BinaryOp::BitXor => {
                            let rhs = int_operand_from_slot(lhs_ty, rhs)?;
                            if matches!(op, BinaryOp::BitAnd | BinaryOp::BitOr | BinaryOp::BitXor)
                                && rhs.is_signed()
                            {
                                return Err(VMInternalError::new(
                                    LoweringError::BitwiseOnSignedValue,
                                ));
                            }
                            let binop = IntBinaryOp { dst, lhs, rhs };
                            self.emit(match op {
                                BinaryOp::Add => MicroOp::IntAdd(binop),
                                BinaryOp::Sub => MicroOp::IntSub(binop),
                                BinaryOp::Mul => MicroOp::IntMul(binop),
                                BinaryOp::Div => MicroOp::IntDiv(binop),
                                BinaryOp::Mod => MicroOp::IntMod(binop),
                                BinaryOp::BitAnd => MicroOp::IntBitAnd(binop),
                                BinaryOp::BitOr => MicroOp::IntBitOr(binop),
                                BinaryOp::BitXor => MicroOp::IntBitXor(binop),
                                BinaryOp::Shl
                                | BinaryOp::Shr
                                | BinaryOp::Cmp(_)
                                | BinaryOp::Or
                                | BinaryOp::And => {
                                    return Err(VMInternalError::new(
                                        LoweringError::UnexpectedOpInArithArm,
                                    ))
                                },
                            })?;
                        },
                        BinaryOp::Shl | BinaryOp::Shr => {
                            let ty = IntTy::from_type(lhs_ty)
                                .filter(|t| !t.is_signed())
                                .ok_or(LoweringError::ShiftRequiresUnsignedNonU64)?;
                            let shift_op = IntShiftOp {
                                ty,
                                dst,
                                lhs,
                                rhs: ShiftOperand::SlotU8(rhs),
                            };
                            self.emit(match op {
                                BinaryOp::Shl => MicroOp::IntShl(shift_op),
                                BinaryOp::Shr => MicroOp::IntShr(shift_op),
                                BinaryOp::Add
                                | BinaryOp::Sub
                                | BinaryOp::Mul
                                | BinaryOp::Div
                                | BinaryOp::Mod
                                | BinaryOp::BitAnd
                                | BinaryOp::BitOr
                                | BinaryOp::BitXor
                                | BinaryOp::Cmp(_)
                                | BinaryOp::Or
                                | BinaryOp::And => {
                                    return Err(VMInternalError::new(
                                        LoweringError::UnexpectedOpInShiftArm,
                                    ))
                                },
                            })?;
                        },
                        // Comparison produces a 1-byte boolean.
                        BinaryOp::Cmp(cmp) => match eq_kind(lhs_ty)? {
                            EqKind::Int => {
                                let rhs = cmp_operand_from_slot(lhs_ty, rhs)?;
                                self.emit_int_cmp(*cmp, dst, lhs, rhs)?;
                            },
                            EqKind::NonIntValue => {
                                self.emit(MicroOp::ValueCmp(ValueCmpOp {
                                    negate: eq_negate(*cmp)?,
                                    dst,
                                    lhs,
                                    rhs,
                                    ty: lhs_interned,
                                }))?;
                            },
                            EqKind::Ref => {
                                self.emit(MicroOp::ValueRefCmp(ValueRefCmpOp {
                                    negate: eq_negate(*cmp)?,
                                    dst,
                                    lhs,
                                    rhs,
                                    ty: strip_ref(lhs_interned)
                                        .ok_or(LoweringError::ExpectedReferenceType)?,
                                }))?;
                            },
                        },
                        // Logical and/or on 1-byte booleans.
                        BinaryOp::And => self.emit(MicroOp::BoolAnd { dst, lhs, rhs })?,
                        BinaryOp::Or => self.emit(MicroOp::BoolOr { dst, lhs, rhs })?,
                    }
                }
            },

            // --- Binary ops with immediate ---
            Instr::BinaryOpImm(dst, op, src, imm) => {
                // TODO(cleanup): see [`Instr::BinaryOp`] above on factoring out the
                // shared skeleton between the reg-reg and imm forms.
                let src_info = self.slot(*src)?;
                let dst_info = self.def_slot(*dst)?;
                let src_ty = self.slot_type(*src)?;
                let dst = dst_info.offset;
                let lhs = src_info.offset;

                // Fast path: u64 specialized imm variants where they exist.
                // u64 BitAnd/BitOr/BitXor/Cmp/Or/And imm have no specialized
                // variant and fall through to the unspecialized path below.
                let mut handled = false;
                if src_ty.is_u64() {
                    let emitted = match op {
                        BinaryOp::Add => Some(MicroOp::AddU64Imm {
                            dst,
                            src: lhs,
                            imm: imm_to_u64(imm)?,
                        }),
                        BinaryOp::Sub => Some(MicroOp::SubU64Imm {
                            dst,
                            src: lhs,
                            imm: imm_to_u64(imm)?,
                        }),
                        BinaryOp::Mul => Some(MicroOp::MulU64Imm {
                            dst,
                            src: lhs,
                            imm: imm_to_u64(imm)?,
                        }),
                        BinaryOp::Div => Some(MicroOp::DivU64Imm {
                            dst,
                            src: lhs,
                            imm: imm_to_u64(imm)?,
                        }),
                        BinaryOp::Mod => Some(MicroOp::ModU64Imm {
                            dst,
                            src: lhs,
                            imm: imm_to_u64(imm)?,
                        }),
                        BinaryOp::Shl => Some(MicroOp::ShlU64Imm {
                            dst,
                            src: lhs,
                            imm: shift_imm_u8(imm)?,
                        }),
                        BinaryOp::Shr => Some(MicroOp::ShrU64Imm {
                            dst,
                            src: lhs,
                            imm: shift_imm_u8(imm)?,
                        }),
                        BinaryOp::BitAnd
                        | BinaryOp::BitOr
                        | BinaryOp::BitXor
                        | BinaryOp::Cmp(_)
                        | BinaryOp::Or
                        | BinaryOp::And => None,
                    };
                    if let Some(micro) = emitted {
                        self.emit(micro)?;
                        handled = true;
                    }
                }

                if !handled {
                    match op {
                        BinaryOp::Add
                        | BinaryOp::Sub
                        | BinaryOp::Mul
                        | BinaryOp::Div
                        | BinaryOp::Mod
                        | BinaryOp::BitAnd
                        | BinaryOp::BitOr
                        | BinaryOp::BitXor => {
                            let rhs = int_operand_from_imm(imm)?;
                            if matches!(op, BinaryOp::BitAnd | BinaryOp::BitOr | BinaryOp::BitXor)
                                && rhs.is_signed()
                            {
                                return Err(VMInternalError::new(
                                    LoweringError::BitwiseOnSignedValue,
                                ));
                            }
                            let binop = IntBinaryOp { dst, lhs, rhs };
                            self.emit(match op {
                                BinaryOp::Add => MicroOp::IntAdd(binop),
                                BinaryOp::Sub => MicroOp::IntSub(binop),
                                BinaryOp::Mul => MicroOp::IntMul(binop),
                                BinaryOp::Div => MicroOp::IntDiv(binop),
                                BinaryOp::Mod => MicroOp::IntMod(binop),
                                BinaryOp::BitAnd => MicroOp::IntBitAnd(binop),
                                BinaryOp::BitOr => MicroOp::IntBitOr(binop),
                                BinaryOp::BitXor => MicroOp::IntBitXor(binop),
                                BinaryOp::Shl
                                | BinaryOp::Shr
                                | BinaryOp::Cmp(_)
                                | BinaryOp::Or
                                | BinaryOp::And => {
                                    return Err(VMInternalError::new(
                                        LoweringError::UnexpectedOpInArithArm,
                                    ))
                                },
                            })?;
                        },
                        BinaryOp::Shl | BinaryOp::Shr => {
                            let ty = IntTy::from_type(src_ty)
                                .filter(|t| !t.is_signed())
                                .ok_or(LoweringError::ShiftRequiresUnsignedNonU64)?;
                            let shift_op = IntShiftOp {
                                ty,
                                dst,
                                lhs,
                                rhs: ShiftOperand::ImmU8(shift_imm_u8(imm)?),
                            };
                            self.emit(match op {
                                BinaryOp::Shl => MicroOp::IntShl(shift_op),
                                BinaryOp::Shr => MicroOp::IntShr(shift_op),
                                BinaryOp::Add
                                | BinaryOp::Sub
                                | BinaryOp::Mul
                                | BinaryOp::Div
                                | BinaryOp::Mod
                                | BinaryOp::BitAnd
                                | BinaryOp::BitOr
                                | BinaryOp::BitXor
                                | BinaryOp::Cmp(_)
                                | BinaryOp::Or
                                | BinaryOp::And => {
                                    return Err(VMInternalError::new(
                                        LoweringError::UnexpectedOpInShiftArm,
                                    ))
                                },
                            })?;
                        },
                        // Comparison against an immediate producing a 1-byte boolean.
                        BinaryOp::Cmp(cmp) => {
                            let rhs = cmp_operand_from_imm(imm)?;
                            self.emit_int_cmp(*cmp, dst, lhs, rhs)?;
                        },
                        // Logical and/or against a constant bool, with identity
                        // `true` for `&&` and `false` for `||`: the identity
                        // yields `src`, the other value yields the constant `!identity`.
                        BinaryOp::And | BinaryOp::Or => {
                            let ImmValue::Bool(b) = imm else {
                                return Err(VMInternalError::new(LoweringError::ImmMustBeBool));
                            };
                            let identity = matches!(op, BinaryOp::And);
                            if *b == identity {
                                self.emit_single_move(dst, src_info)?;
                            } else {
                                self.emit(MicroOp::StoreImm1 {
                                    dst,
                                    imm: (!identity) as u8,
                                })?;
                            }
                        },
                    }
                }
            },

            // --- Unary ops ---
            Instr::UnaryOp(dst, op, src) => match op {
                UnaryOp::Cast(to) => self.lower_cast(*dst, *src, *to)?,
                UnaryOp::Negate => {
                    let src_ty = self.slot_type(*src)?;
                    let signed_ty = IntTy::from_type(src_ty)
                        .filter(|t| t.is_signed())
                        .ok_or(LoweringError::NegateRequiresSignedInt)?;
                    let src_info = self.slot(*src)?;
                    let dst_info = self.def_slot(*dst)?;
                    self.emit(MicroOp::IntNegate(IntNegateOp {
                        ty: signed_ty,
                        dst: dst_info.offset,
                        src: src_info.offset,
                    }))?;
                },
                UnaryOp::FreezeRef => {
                    // Runtime no-op: &mut T and &T share the same 16-byte
                    // fat-pointer representation. Propagate the slot value.
                    // TODO(cleanup): fold this away at the stackless exec IR level so
                    // lowering emits nothing at all.
                    let src_info = self.slot(*src)?;
                    let dst_info = self.def_slot(*dst)?;
                    self.emit_single_move(dst_info.offset, src_info)?;
                },
                UnaryOp::Not => {
                    let src_info = self.slot(*src)?;
                    let dst_info = self.def_slot(*dst)?;
                    self.emit(MicroOp::BoolNot {
                        dst: dst_info.offset,
                        src: src_info.offset,
                    })?;
                },
            },

            // --- References ---
            Instr::ImmBorrowLoc(dst, src) | Instr::MutBorrowLoc(dst, src) => {
                let src_info = self.slot(*src)?;
                let dst_info = self.def_slot(*dst)?;
                self.emit(MicroOp::SlotBorrow {
                    dst: dst_info.offset,
                    local: src_info.offset,
                })?;
            },
            Instr::ReadRef(dst, ref_src) => {
                // TODO(perf): `size` could equivalently come from `dst_info.size`
                // (the loaded value's slot) — IR typing forces it to equal
                // the ref's pointee size. `ref_pointee_size` is the more
                // type-faithful path; `dst_info.size` would be cheaper.
                let size = self.ref_pointee_size(*ref_src)?;
                let ref_info = self.slot(*ref_src)?;
                let dst_info = self.def_typed_slot(*dst)?;
                self.emit(MicroOp::ReadRef {
                    dst: dst_info.slot.offset,
                    ref_ptr: ref_info.offset,
                    size,
                })?;
                self.maybe_deep_copy(dst_info.ty, dst_info.slot.offset)?;
            },
            Instr::WriteRef(ref_dst, src) => {
                // TODO(perf): `size` could equivalently come from `src_info.size`
                // (the value being written) — IR typing forces it to equal
                // the ref's pointee size. `ref_pointee_size` is the more
                // type-faithful path; `src_info.size` would be cheaper.
                let size = self.ref_pointee_size(*ref_dst)?;
                let ref_info = self.slot(*ref_dst)?;
                let src_info = self.slot(*src)?;
                self.emit(MicroOp::WriteRef {
                    ref_ptr: ref_info.offset,
                    src: src_info.offset,
                    size,
                })?;
            },

            // --- Control flow ---
            Instr::Branch(Label(l)) => {
                self.record_branch_fixup();
                self.emit(MicroOp::Jump {
                    target: CodeOffset(encode_label(*l)),
                    gas: 0,
                })?;
            },
            Instr::BrTrue(Label(l), cond) => {
                let cond_info = self.slot(*cond)?;
                self.record_branch_fixup();
                self.emit(MicroOp::JumpNotZeroByte {
                    target: CodeOffset(encode_label(*l)),
                    src: cond_info.offset,
                    gas_taken: 0,
                    gas_fallthrough: 0,
                })?;
            },
            Instr::BrFalse(Label(l), cond) => {
                let cond_info = self.slot(*cond)?;
                self.record_branch_fixup();
                self.emit(MicroOp::JumpZeroByte {
                    target: CodeOffset(encode_label(*l)),
                    src: cond_info.offset,
                    gas_taken: 0,
                    gas_fallthrough: 0,
                })?;
            },

            // --- Fused compare+branch ---
            Instr::BrCmp(Label(l), op, lhs, rhs) => {
                let lhs_interned = self.slot_interned_type(*lhs)?;
                let lhs_ty = view_type(lhs_interned);
                let lhs_info = self.slot(*lhs)?;
                let lhs_off = lhs_info.offset;
                let rhs_off = self.slot(*rhs)?.offset;
                let target = CodeOffset(encode_label(*l));
                self.record_branch_fixup();
                // Fast path: unsigned `u64` ordering / not-equal use the
                // specialized jumps. Everything else goes through the general
                // `JumpIntCmp`, which dispatches on the operand type.
                match (lhs_ty.is_u64(), op) {
                    (true, CmpKind::Lt) => self.emit(MicroOp::JumpLessU64 {
                        target,
                        lhs: lhs_off,
                        rhs: rhs_off,
                        gas_taken: 0,
                        gas_fallthrough: 0,
                    })?,
                    (true, CmpKind::Ge) => self.emit(MicroOp::JumpGreaterEqualU64 {
                        target,
                        lhs: lhs_off,
                        rhs: rhs_off,
                        gas_taken: 0,
                        gas_fallthrough: 0,
                    })?,
                    // x > y ↔ y < x
                    (true, CmpKind::Gt) => self.emit(MicroOp::JumpLessU64 {
                        target,
                        lhs: rhs_off,
                        rhs: lhs_off,
                        gas_taken: 0,
                        gas_fallthrough: 0,
                    })?,
                    // x <= y ↔ y >= x
                    (true, CmpKind::Le) => self.emit(MicroOp::JumpGreaterEqualU64 {
                        target,
                        lhs: rhs_off,
                        rhs: lhs_off,
                        gas_taken: 0,
                        gas_fallthrough: 0,
                    })?,
                    (true, CmpKind::Neq) => self.emit(MicroOp::JumpNotEqualU64 {
                        target,
                        lhs: lhs_off,
                        rhs: rhs_off,
                        gas_taken: 0,
                        gas_fallthrough: 0,
                    })?,
                    _ => match eq_kind(lhs_ty)? {
                        EqKind::Int => {
                            let rhs_op = cmp_operand_from_slot(lhs_ty, rhs_off)?;
                            self.emit_jump_int_cmp(target, *op, lhs_off, rhs_op)?;
                        },
                        EqKind::NonIntValue => {
                            self.emit(MicroOp::JumpValueCmp(JumpValueCmpOp {
                                target,
                                negate: eq_negate(*op)?,
                                lhs: lhs_off,
                                rhs: rhs_off,
                                ty: lhs_interned,
                                gas_taken: 0,
                                gas_fallthrough: 0,
                            }))?;
                        },
                        EqKind::Ref => {
                            self.emit(MicroOp::JumpValueRefCmp(JumpValueRefCmpOp {
                                target,
                                negate: eq_negate(*op)?,
                                lhs: lhs_off,
                                rhs: rhs_off,
                                ty: strip_ref(lhs_interned)
                                    .ok_or(LoweringError::ExpectedReferenceType)?,
                                gas_taken: 0,
                                gas_fallthrough: 0,
                            }))?;
                        },
                    },
                }
            },
            Instr::BrCmpImm(Label(l), op, src, imm) => {
                let src_ty = self.slot_type(*src)?;
                let src_off = self.slot(*src)?.offset;
                let target = CodeOffset(encode_label(*l));
                self.record_branch_fixup();
                if src_ty.is_u64() {
                    // Fast path: specialized unsigned `u64` ordering jumps.
                    // Note: equality has no specialized imm jump, so it uses
                    // the general `JumpIntCmp`.
                    let v = imm_to_u64(imm)?;
                    match op {
                        CmpKind::Ge => self.emit(MicroOp::JumpGreaterEqualU64Imm {
                            target,
                            src: src_off,
                            imm: v,
                            gas_taken: 0,
                            gas_fallthrough: 0,
                        })?,
                        CmpKind::Lt => self.emit(MicroOp::JumpLessU64Imm {
                            target,
                            src: src_off,
                            imm: v,
                            gas_taken: 0,
                            gas_fallthrough: 0,
                        })?,
                        CmpKind::Gt => self.emit(MicroOp::JumpGreaterU64Imm {
                            target,
                            src: src_off,
                            imm: v,
                            gas_taken: 0,
                            gas_fallthrough: 0,
                        })?,
                        CmpKind::Le => self.emit(MicroOp::JumpLessEqualU64Imm {
                            target,
                            src: src_off,
                            imm: v,
                            gas_taken: 0,
                            gas_fallthrough: 0,
                        })?,
                        CmpKind::Eq | CmpKind::Neq => {
                            self.emit_jump_int_cmp(target, *op, src_off, IntOperand::ImmU64(v))?
                        },
                    }
                } else {
                    let rhs_op = cmp_operand_from_imm(imm)?;
                    self.emit_jump_int_cmp(target, *op, src_off, rhs_op)?;
                }
            },

            // --- Calls ---
            Instr::Call(rets, _handle_idx, _ty_args, args) => {
                self.lower_call(func_ir, args, rets)?;
            },

            // --- Return ---
            Instr::Ret(slots) => {
                // `return_slots` overlap the home region (calling
                // convention reuses that space), so a function like
                // `swap(a, b) -> (b, a)` produces a swap-cycle in the
                // copy graph. `emit_parallel_copy` handles arbitrary
                // cycles via the function's reserved scratch slot.
                let mut copies = Vec::with_capacity(slots.len());
                for (k, slot) in slots.iter().enumerate() {
                    let src = self.slot(*slot)?;
                    let dst = self.ctx.return_slots[k];
                    copies.push(parallel_copy::Copy {
                        src: src.offset,
                        dst: dst.offset,
                        width: src.size,
                    });
                }
                parallel_copy::emit_parallel_copy(&mut self.out_buf, &copies, self.ctx.scratch)?;
                self.emit(MicroOp::Return)?;
            },

            // --- Abort ---
            Instr::Abort(code) => {
                let code = self.slot(*code)?;
                self.emit(MicroOp::Abort { code: code.offset })?;
            },
            Instr::AbortMsg(code, message) => {
                let code = self.slot(*code)?;
                let message = self.slot(*message)?;
                self.emit(MicroOp::AbortMsg {
                    code: code.offset,
                    message: message.offset,
                })?;
            },

            // --- Vector ---
            Instr::VecLen(dst, _elem_ty, vec_ref) => {
                let vec_ref_info = self.slot(*vec_ref)?;
                let dst_info = self.def_slot(*dst)?;
                self.emit(MicroOp::VecLen {
                    dst: dst_info.offset,
                    vec_ref: vec_ref_info.offset,
                })?;
            },
            Instr::VecImmBorrow(dst, elem_ty, vec_ref, idx)
            | Instr::VecMutBorrow(dst, elem_ty, vec_ref, idx) => {
                let elem_size = concrete_type_size(
                    self.ctx.layouts,
                    self.concrete_ty(*elem_ty)?,
                    "vector elem type",
                )?;
                let vec_ref_info = self.slot(*vec_ref)?;
                let idx_info = self.slot(*idx)?;
                let dst_info = self.def_slot(*dst)?;
                // The fat pointer does not distinguish between mutable and immutable borrow.
                self.emit(MicroOp::VecBorrow {
                    dst: dst_info.offset,
                    vec_ref: vec_ref_info.offset,
                    idx: idx_info.offset,
                    elem_size,
                })?;
            },
            Instr::VecPopBack(dst, elem_ty, vec_ref) => {
                let elem_size = concrete_type_size(
                    self.ctx.layouts,
                    self.concrete_ty(*elem_ty)?,
                    "vector elem type",
                )?;
                let vec_ref_info = self.slot(*vec_ref)?;
                let dst_info = self.def_slot(*dst)?;
                self.emit(MicroOp::VecPopBack {
                    dst: dst_info.offset,
                    vec_ref: vec_ref_info.offset,
                    elem_size,
                })?;
            },
            Instr::VecPack(dst, elem_ty, elems) => {
                let dst_typed = self.def_typed_slot(*dst)?;
                // Empty literal keeps the fast path: null pointer is the empty
                // vector — no allocation, no descriptor needed.
                if elems.is_empty() {
                    self.emit(MicroOp::VecNew {
                        dst: dst_typed.slot.offset,
                    })?;
                } else {
                    let elem_size = concrete_type_size(
                        self.ctx.layouts,
                        self.concrete_ty(*elem_ty)?,
                        "vector elem type",
                    )?;
                    let descriptor_id = self.vector_descriptor_id("VecPack", dst_typed.ty)?;
                    let srcs = elems
                        .iter()
                        .map(|elem| Ok(self.slot(*elem)?.offset))
                        .collect::<VMResult<Vec<_>>>()?;
                    self.emit(MicroOp::VecPack(Box::new(VecPackOp {
                        dst: dst_typed.slot.offset,
                        descriptor_id,
                        elem_size,
                        srcs,
                    })))?;
                }
            },
            Instr::VecUnpack(dsts, elem_ty, src) => {
                // TODO(completeness): the Move compiler only emits `VecUnpack` with count 0,
                // but handwritten bytecode may use it with any count. If we
                // restrict to count 0, we can apply simplifications.
                let elem_size = concrete_type_size(
                    self.ctx.layouts,
                    self.concrete_ty(*elem_ty)?,
                    "vector elem type",
                )?;
                let src_info = self.slot(*src)?;
                let dst_offsets = dsts
                    .iter()
                    .map(|dst| Ok(self.def_slot(*dst)?.offset))
                    .collect::<VMResult<Vec<_>>>()?;
                self.emit(MicroOp::VecUnpack(Box::new(VecUnpackOp {
                    src: src_info.offset,
                    elem_size,
                    dsts: dst_offsets,
                })))?;
            },
            Instr::VecSwap(elem_ty, vec_ref, idx_a, idx_b) => {
                let elem_size = concrete_type_size(
                    self.ctx.layouts,
                    self.concrete_ty(*elem_ty)?,
                    "vector elem type",
                )?;
                let vec_ref_info = self.slot(*vec_ref)?;
                let idx_a_info = self.slot(*idx_a)?;
                let idx_b_info = self.slot(*idx_b)?;
                self.emit(MicroOp::VecSwap {
                    vec_ref: vec_ref_info.offset,
                    idx_a: idx_a_info.offset,
                    idx_b: idx_b_info.offset,
                    elem_size,
                })?;
            },
            Instr::VecPushBack(elem_ty, vec_ref, val) => {
                let elem_size = concrete_type_size(
                    self.ctx.layouts,
                    self.concrete_ty(*elem_ty)?,
                    "vector elem type",
                )?;
                let vec_ty = strip_ref(self.slot_interned_type(*vec_ref)?)
                    .ok_or(LoweringError::ExpectedReferenceType)?;
                let descriptor_id = self.vector_descriptor_id("VecPushBack", vec_ty)?;
                let vec_ref_info = self.slot(*vec_ref)?;
                let val_info = self.slot(*val)?;
                self.emit(MicroOp::VecPushBack {
                    vec_ref: vec_ref_info.offset,
                    elem: val_info.offset,
                    elem_size,
                    descriptor_id,
                })?;
            },

            // --- Inline-struct: by-value ---
            //
            // The struct lives in a frame slot at compile-time-known offset;
            // the field's absolute frame offset is therefore also compile-time.
            // No fat pointer is materialized.
            Instr::ImmBorrowLocField(dst, owner, fh, local)
            | Instr::MutBorrowLocField(dst, owner, fh, local) => {
                let (field_offset, _) = self.resolve_field(self.concrete_ty(*owner)?, *fh)?;
                let local_info = self.slot(*local)?;
                let dst_info = self.def_slot(*dst)?;
                self.emit(MicroOp::SlotBorrow {
                    dst: dst_info.offset,
                    local: FrameOffset(local_info.offset.0 + field_offset),
                })?;
            },
            Instr::ReadLocalField(dst, owner, fh, local) => {
                let (field_offset, field_size) =
                    self.resolve_field(self.concrete_ty(*owner)?, *fh)?;
                let local_info = self.slot(*local)?;
                let dst_info = self.def_typed_slot(*dst)?;
                let src = SizedSlot {
                    offset: FrameOffset(local_info.offset.0 + field_offset),
                    size: field_size,
                    align: local_info.align,
                };
                self.emit_single_move(dst_info.slot.offset, src)?;
                self.maybe_deep_copy(dst_info.ty, dst_info.slot.offset)?;
            },
            Instr::WriteLocalField(owner, fh, local, val) => {
                let (field_offset, field_size) =
                    self.resolve_field(self.concrete_ty(*owner)?, *fh)?;
                let local_info = self.slot(*local)?;
                let val_info = self.slot(*val)?;
                let src = SizedSlot {
                    offset: val_info.offset,
                    size: field_size,
                    align: val_info.align,
                };
                self.emit_single_move(FrameOffset(local_info.offset.0 + field_offset), src)?;
            },

            // --- Inline-struct: by-ref ---
            //
            // The struct's location is only known at runtime via the fat
            // pointer in `src` (or `dst_ref`). Use the offset-bearing ref
            // micro-ops to fold the field offset into the address compute in a
            // single dispatch.
            Instr::ImmBorrowField(dst, owner, fh, src)
            | Instr::MutBorrowField(dst, owner, fh, src) => {
                let (field_offset, _) = self.resolve_field(self.concrete_ty(*owner)?, *fh)?;
                let src_info = self.slot(*src)?;
                let dst_info = self.def_slot(*dst)?;
                self.emit(MicroOp::DeriveRefOffsetImm {
                    dst_ref: dst_info.offset,
                    src_ref: src_info.offset,
                    offset: field_offset,
                })?;
            },
            Instr::ReadField(dst, owner, fh, src) => {
                let (field_offset, field_size) =
                    self.resolve_field(self.concrete_ty(*owner)?, *fh)?;
                let src_info = self.slot(*src)?;
                let dst_info = self.def_typed_slot(*dst)?;
                self.emit(MicroOp::ReadRefOffset {
                    dst: dst_info.slot.offset,
                    ref_ptr: src_info.offset,
                    offset: field_offset,
                    size: field_size,
                })?;
                self.maybe_deep_copy(dst_info.ty, dst_info.slot.offset)?;
            },
            Instr::WriteField(owner, fh, dst_ref, val) => {
                let (field_offset, field_size) =
                    self.resolve_field(self.concrete_ty(*owner)?, *fh)?;
                let ref_info = self.slot(*dst_ref)?;
                let val_info = self.slot(*val)?;
                self.emit(MicroOp::WriteRefOffset {
                    ref_ptr: ref_info.offset,
                    offset: field_offset,
                    src: val_info.offset,
                    size: field_size,
                })?;
            },

            // --- Pack / Unpack: per-field byte copies between frame slots ---
            //
            // Per instance, emit fields in whichever order is overlap-safe
            // for the resolved offsets: reverse if `reverse_emit_is_safe`,
            // else forward if `forward_emit_is_safe`, else bail. A true
            // copy cycle (which neither order resolves) needs a swap-style
            // bytecode op which does not currently exist.
            Instr::Pack(dst, struct_ty, args) => {
                // After substitution, `struct_ty` must be a concrete nominal
                // with a populated layout.
                let struct_ty = self.concrete_ty(*struct_ty)?;
                // (offset, size, align) per field.
                let field_layouts = self.struct_field_layouts(struct_ty, "Pack")?;
                if field_layouts.len() != args.len() {
                    return Err(VMInternalError::new(LoweringError::FieldCountMismatch {
                        op: "Pack",
                        provided: args.len(),
                        expected: field_layouts.len(),
                    }));
                }
                let dst_info = self.def_slot(*dst)?;
                let arg_infos = args
                    .iter()
                    .map(|s| self.slot(*s))
                    .collect::<VMResult<Vec<_>>>()?;
                let copies: Vec<_> = field_layouts
                    .iter()
                    .zip(arg_infos.iter())
                    .map(|(&(offset, size, _align), arg_info)| parallel_copy::Copy {
                        src: arg_info.offset,
                        dst: FrameOffset(dst_info.offset.0 + offset),
                        width: size,
                    })
                    .collect();
                let mut indices: Vec<usize> = (0..field_layouts.len()).collect();
                // TODO(perf): check if we can have cheaper checks for reverse/forward emit safety
                // in the presence of alignments.
                if parallel_copy::reverse_emit_is_safe(&copies) {
                    indices.reverse();
                } else if !parallel_copy::forward_emit_is_safe(&copies) {
                    return Err(VMInternalError::new(LoweringError::NotOverlapSafe {
                        op: "Pack",
                    }));
                }
                for i in indices {
                    let (offset, size, align) = field_layouts[i];
                    self.emit_single_move(FrameOffset(dst_info.offset.0 + offset), SizedSlot {
                        offset: arg_infos[i].offset,
                        size,
                        align,
                    })?;
                }
            },
            Instr::Unpack(dsts, struct_ty, src) => {
                // See the `Instr::Pack` arm above for the `struct_ty` contract.
                let struct_ty = self.concrete_ty(*struct_ty)?;
                let field_layouts = self.struct_field_layouts(struct_ty, "Unpack")?;
                if field_layouts.len() != dsts.len() {
                    return Err(VMInternalError::new(LoweringError::FieldCountMismatch {
                        op: "Unpack",
                        provided: dsts.len(),
                        expected: field_layouts.len(),
                    }));
                }
                let src_info = self.slot(*src)?;
                // Resolve all dst offsets first so the overlap-safety check can
                // run before any load is emitted.
                let mut dst_offsets = Vec::with_capacity(dsts.len());
                for &dst in dsts.iter() {
                    dst_offsets.push(self.def_slot(dst)?.offset);
                }
                let copies: Vec<_> = field_layouts
                    .iter()
                    .zip(dst_offsets.iter())
                    .map(|(&(offset, size, _align), dst_off)| parallel_copy::Copy {
                        src: FrameOffset(src_info.offset.0 + offset),
                        dst: *dst_off,
                        width: size,
                    })
                    .collect();
                let mut indices: Vec<usize> = (0..field_layouts.len()).collect();
                if parallel_copy::reverse_emit_is_safe(&copies) {
                    indices.reverse();
                } else if !parallel_copy::forward_emit_is_safe(&copies) {
                    return Err(VMInternalError::new(LoweringError::NotOverlapSafe {
                        op: "Unpack",
                    }));
                }
                for i in indices {
                    let (offset, size, align) = field_layouts[i];
                    self.emit_single_move(dst_offsets[i], SizedSlot {
                        offset: FrameOffset(src_info.offset.0 + offset),
                        size,
                        align,
                    })?;
                }
            },

            // --- Closures ---
            Instr::PackClosure(dst, _, _, mask, captured) => {
                // Target identity (with composed type arguments for the
                // generic form) + captured-data descriptor were resolved in
                // `try_build_context`; read them positionally.
                let info = &self.ctx.closure_pack_sites[self.closure_pack_cursor];
                self.closure_pack_cursor += 1;
                let dst_off = self.def_slot(*dst)?.offset;
                // Captured sources, in ascending captured-param order (the
                // destacker already ordered the IR `captured` list this way).
                let captured_slots = self.slots_to_sized_slots(captured)?;
                self.emit(MicroOp::PackClosure(Box::new(PackClosureOp {
                    dst: dst_off,
                    func_ref: ClosureFuncRef::Unresolved(info.func_ref),
                    mask: mask.bits(),
                    captured_data_descriptor_id: info.captured_data_descriptor_id,
                    values_size: info.values_size,
                    captured: captured_slots,
                })))?;
            },
            Instr::CallClosure(rets, _sig_types, all_args) => {
                let ret_slots = &self.ctx.closure_call_sites[self.closure_call_cursor];
                self.closure_call_cursor += 1;
                // The destacker pushes the closure as the last operand;
                // everything before it is a provided (non-captured) argument.
                let Some((closure_slot, provided)) = all_args.split_last() else {
                    return Err(VMInternalError::new(LoweringError::CallClosureNoOperand));
                };
                let closure_src = self.slot(*closure_slot)?.offset;
                let provided_args = self.slots_to_sized_slots(provided)?;
                self.emit(MicroOp::CallClosure(Box::new(CallClosureOp {
                    closure_src,
                    provided_args,
                })))?;
                self.bind_call_returns(rets, ret_slots)?;
            },

            // --- Global storage ---
            Instr::Exists(dst, ty, addr) => {
                let concrete_ty = self.concrete_ty(*ty)?;
                let addr_info = self.slot(*addr)?;
                let dst_info = self.def_slot(*dst)?;
                self.emit(MicroOp::Exists {
                    addr: addr_info.offset,
                    ty: concrete_ty,
                    dst: dst_info.offset,
                })?;
            },
            Instr::ImmBorrowGlobal(dst, ty, addr) => {
                let concrete_ty = self.concrete_ty(*ty)?;
                let addr_info = self.slot(*addr)?;
                let dst_info = self.def_slot(*dst)?;
                self.emit(MicroOp::BorrowGlobal {
                    dst: dst_info.offset,
                    addr: addr_info.offset,
                    ty: concrete_ty,
                })?;
            },
            Instr::MutBorrowGlobal(dst, ty, addr) => {
                let concrete_ty = self.concrete_ty(*ty)?;
                let addr_info = self.slot(*addr)?;
                let dst_info = self.def_slot(*dst)?;
                self.emit(MicroOp::BorrowGlobalMut {
                    dst: dst_info.offset,
                    addr: addr_info.offset,
                    ty: concrete_ty,
                })?;
            },
            Instr::MoveFrom(dst, ty, addr) => {
                // Move the resource out as an 8-byte heap pointer into
                // `box_ptr`, then unbox its inline value into `dst`. `MoveFrom`
                // may GC (deep-copy) before it writes `box_ptr`; the unbox copy
                // that follows does not allocate.
                let concrete_ty = self.concrete_ty(*ty)?;
                let box_ptr = self
                    .ctx
                    .resource_box_slot
                    .ok_or(LoweringError::BoxPtrSlotNotReserved { op: "MoveFrom" })?;
                let addr_info = self.slot(*addr)?;
                let dst_info = self.def_slot(*dst)?;
                self.emit(MicroOp::MoveFrom {
                    dst: box_ptr,
                    addr: addr_info.offset,
                    ty: concrete_ty,
                })?;
                debug_assert_eq!(
                    dst_info.size,
                    concrete_type_size(self.ctx.layouts, concrete_ty, "move_from resource")?,
                    "move_from: dst slot width must equal the resource's heap-object payload size",
                );
                self.emit_heap_move_from(dst_info.offset, box_ptr, dst_info.size)?;
            },
            Instr::MoveTo(ty, signer, val) => {
                // Box the inline resource value into a fresh heap object, then
                // publish the pointer. `HeapNew` allocates (and may GC) before
                // it writes `box_ptr`; the `HeapMoveTo` box copy and `MoveTo`
                // that follow do not allocate.
                //
                // If the resource embeds a child heap pointer (e.g. a vector
                // field), the GC inside `HeapNew` relocates it in place before
                // the `HeapMoveTo` reads it: `val` is a home slot, so
                // `derive_frame_layout` lists its pointer offsets in the
                // function's `frame_layout`, which the runtime scans at every
                // collection (not only at frame entry). So `HeapMoveTo` copies
                // the relocated child pointer, not a stale one. This relies on
                // `frame_layout` being a whole-function root set; it would break
                // if safe points became per-PC / liveness-based.
                let concrete_ty = self.concrete_ty(*ty)?;
                let descriptor_id = self
                    .ctx
                    .descriptor_id(concrete_ty)
                    .ok_or(LoweringError::ResourceTypeNoDescriptor { op: "MoveTo" })?;
                let box_ptr = self
                    .ctx
                    .resource_box_slot
                    .ok_or(LoweringError::BoxPtrSlotNotReserved { op: "MoveTo" })?;
                let signer_info = self.slot(*signer)?;
                let val_info = self.slot(*val)?;
                self.emit(MicroOp::HeapNew {
                    dst: box_ptr,
                    descriptor_id,
                })?;
                debug_assert_eq!(
                    val_info.size,
                    concrete_type_size(self.ctx.layouts, concrete_ty, "move_to resource")?,
                    "move_to: val slot width must equal the descriptor payload size HeapNew allocated",
                );
                self.emit_heap_move_to(box_ptr, val_info.offset, val_info.size)?;
                self.emit(MicroOp::MoveTo {
                    signer_ref: signer_info.offset,
                    ty: concrete_ty,
                    src: box_ptr,
                })?;
            },

            // --- Enum variant ops (non-generic) ---
            //
            // Enums are heap objects: `[tag(8)] [variant data]`. Pack
            // allocates, writes the tag, then stores each field; Unpack
            // asserts the runtime tag, then reads each field by value; the
            // others read/write/borrow through the heap pointer. Field offsets
            // come from the derived `EnumLayout`; the `enum_*` helpers add
            // `ENUM_DATA_OFFSET`.
            Instr::PackVariant(dst, enum_ty, variant_ord, args) => {
                let enum_ty = self.concrete_ty(*enum_ty)?;
                let ctx = self.ctx;
                let layout = ctx
                    .enum_layout(enum_ty)
                    .ok_or(LoweringError::EnumLayoutNotDerived { op: "PackVariant" })?;
                let variant_fields = layout.variants.get(*variant_ord as usize).ok_or(
                    LoweringError::VariantOrdinalOutOfRange {
                        op: "PackVariant",
                        ordinal: *variant_ord as usize,
                    },
                )?;
                if variant_fields.len() != args.len() {
                    return Err(VMInternalError::new(LoweringError::FieldCountMismatch {
                        op: "PackVariant",
                        provided: args.len(),
                        expected: variant_fields.len(),
                    }));
                }
                let descriptor_id = layout.descriptor_id;
                // GC-safe ordering: `def_slot` only stages the dst's Xfer bind,
                // so the dst is not a safe-point root at the allocating
                // `EnumNew`; the live field args are. `EnumNew` fuses the
                // allocation and the tag store.
                let dst_off = self.def_slot(*dst)?.offset;
                // Resolve the field arg slots up front so we can detect aliasing
                // before emitting.
                let arg_slots = self.slots_to_sized_slots(args)?;
                // `EnumNew` writes the heap pointer to its target slot BEFORE the
                // field stores read the args. The slot allocator may thread the
                // enum dst and a field arg through one Xfer position (its
                // read-before-write contract assumes an instruction reads all
                // sources before writing dst), so if `dst`'s pointer slot aliases
                // any arg slot, writing the pointer to `dst` would clobber that
                // arg before its store reads it. In that case stage the pointer in
                // the reserved scratch and publish it to `dst` only after the last
                // store; otherwise write straight to `dst`.
                let aliases_arg =
                    variant_fields
                        .iter()
                        .zip(arg_slots.iter())
                        .any(|(field, arg)| {
                            ranges_overlap(dst_off.0, ENUM_PTR_SIZE, arg.offset.0, field.size)
                        });
                let pack_ptr = if aliases_arg {
                    self.ctx
                        .enum_ptr_scratch
                        .ok_or(LoweringError::EnumPtrScratchMissing { op: "PackVariant" })?
                } else {
                    dst_off
                };
                self.emit(MicroOp::enum_new(pack_ptr, descriptor_id, *variant_ord))?;
                // `enum_store` is non-allocating: no safe point between `EnumNew`
                // and the final store, so the scratch pointer never spans a GC.
                for (field, arg) in variant_fields.iter().zip(arg_slots.iter()) {
                    let size = field.size;
                    let store = if size == 8 {
                        MicroOp::enum_store8(pack_ptr, field.offset, arg.offset)
                    } else {
                        MicroOp::enum_store(pack_ptr, field.offset, arg.offset, size)
                    };
                    self.emit(store)?;
                }
                if aliases_arg {
                    self.emit(MicroOp::Move8 {
                        dst: dst_off,
                        src: pack_ptr,
                    })?;
                }
            },
            Instr::UnpackVariant(dsts, enum_ty, variant_ord, src) => {
                let enum_ty = self.concrete_ty(*enum_ty)?;
                let ctx = self.ctx;
                let layout =
                    ctx.enum_layout(enum_ty)
                        .ok_or(LoweringError::EnumLayoutNotDerived {
                            op: "UnpackVariant",
                        })?;
                let variant_fields = layout.variants.get(*variant_ord as usize).ok_or(
                    LoweringError::VariantOrdinalOutOfRange {
                        op: "UnpackVariant",
                        ordinal: *variant_ord as usize,
                    },
                )?;
                if variant_fields.len() != dsts.len() {
                    return Err(VMInternalError::new(LoweringError::FieldCountMismatch {
                        op: "UnpackVariant",
                        provided: dsts.len(),
                        expected: variant_fields.len(),
                    }));
                }
                let src_off = self.slot(*src)?.offset;
                // Resolve all dst slots up front (mirrors struct `Unpack`) so we
                // can detect aliasing before emitting any load.
                let mut dst_offs = Vec::with_capacity(dsts.len());
                for dst in dsts.iter() {
                    dst_offs.push(self.def_slot(*dst)?.offset);
                }
                // Each field load rereads the enum pointer from its source slot.
                // The slot allocator may thread the consumed enum and a field
                // output through one Xfer position (its read-before-write contract
                // assumes sources are read before dsts are written), so if a dst
                // slot aliases `src`, the load that writes that dst corrupts the
                // pointer and the next load dereferences garbage (memory-unsafe).
                // In that case copy the pointer into the reserved scratch and load
                // every field from there; otherwise read straight from `src`.
                let aliases_dst =
                    variant_fields
                        .iter()
                        .zip(dst_offs.iter())
                        .any(|(field, dst_off)| {
                            ranges_overlap(src_off.0, ENUM_PTR_SIZE, dst_off.0, field.size)
                        });
                let load_ptr = if aliases_dst {
                    let scratch =
                        self.ctx
                            .enum_ptr_scratch
                            .ok_or(LoweringError::EnumPtrScratchMissing {
                                op: "UnpackVariant",
                            })?;
                    self.emit(MicroOp::Move8 {
                        dst: scratch,
                        src: src_off,
                    })?;
                    scratch
                } else {
                    src_off
                };
                // Assert the runtime tag matches before reading fields by
                // value. Reachable from a refutable `let E::V { .. } = e` or
                // hand-crafted bytecode; without it, mono-move would read
                // another variant's bytes (potential type confusion).
                //
                // TODO(perf): elide this check when the variant is statically
                // known. The dominant case is a `match` arm, where a preceding
                // `TestVariant(&L, V)` + `BrTrue` already proved the tag. A
                // sound elision is a conservative forward dataflow in the
                // destack phase tracking "local L is known variant V": seed {L:
                // V} on the taken edge of `TestVariant(ref, V)` + `BrTrue`
                // where `ref` borrows L (via `ImmBorrowLoc` provenance); kill
                // on any reassignment of L; intersect at CFG joins; then drop
                // this check when `src` is a direct Move/Copy of a local known
                // to be V. Default to emitting the check, so an imprecise
                // analysis can only keep a redundant check, never drop a needed
                // one. The fact must key on the shared local (TestVariant takes
                // a `&enum`, UnpackVariant the value); there is no
                // `VariantSwitch` in this IR to shortcut through.
                self.emit(MicroOp::enum_check_variant(load_ptr, *variant_ord as u64))?;
                for (field, &dst_off) in variant_fields.iter().zip(dst_offs.iter()) {
                    let size = field.size;
                    let load = if size == 8 {
                        MicroOp::enum_load8(load_ptr, field.offset, dst_off)
                    } else {
                        MicroOp::enum_load(load_ptr, field.offset, dst_off, size)
                    };
                    self.emit(load)?;
                }
            },
            Instr::TestVariant(dst, _enum_ty, variant_ord, src) => {
                // `src` is an enum reference.
                let enum_ref = self.slot(*src)?.offset;
                let dst_off = self.def_slot(*dst)?.offset;
                self.emit(MicroOp::enum_test_tag(
                    enum_ref,
                    *variant_ord as u64,
                    dst_off,
                ))?;
            },
            // By-reference field read/write: `src`/`dst_ref` is an enum fat
            // pointer. The uniform-offset fast path fuses the heap deref and
            // the value copy into one `Enum{Read,Write}VariantField` (no
            // scratch reference). The divergent-offset path resolves the offset
            // by tag: `emit_variant_field_borrow` materializes a field
            // reference in the scratch slot, then `ReadRef`/`WriteRef` accesses
            // it. Every op here is non-allocating, so the scratch reference
            // never spans a safe point.
            Instr::ReadVariantField(dst, enum_ty, vfh, src) => {
                let access = self.resolve_variant_field(self.concrete_ty(*enum_ty)?, *vfh)?;
                let src_off = self.slot(*src)?.offset;
                let dst_typed = self.def_typed_slot(*dst)?;
                let dst_off = dst_typed.slot.offset;
                match access.uniform_offset {
                    Some(offset) => self.emit(MicroOp::enum_read_variant_field(
                        src_off,
                        offset,
                        dst_off,
                        access.field_size,
                    ))?,
                    None => {
                        let scratch = self.ctx.variant_field_scratch.ok_or(
                            LoweringError::VariantFieldScratchMissing {
                                op: "ReadVariantField",
                            },
                        )?;
                        self.emit_variant_field_borrow(&access, src_off, scratch)?;
                        self.emit(MicroOp::ReadRef {
                            dst: dst_off,
                            ref_ptr: scratch,
                            size: access.field_size,
                        })?;
                    },
                }
                // A by-value variant-field read materializes the field; if it is
                // itself heap-backed, make it independent.
                self.maybe_deep_copy(dst_typed.ty, dst_off)?;
            },
            Instr::WriteVariantField(enum_ty, vfh, dst_ref, val) => {
                let access = self.resolve_variant_field(self.concrete_ty(*enum_ty)?, *vfh)?;
                let ref_off = self.slot(*dst_ref)?.offset;
                let val_off = self.slot(*val)?.offset;
                match access.uniform_offset {
                    Some(offset) => self.emit(MicroOp::enum_write_variant_field(
                        ref_off,
                        offset,
                        val_off,
                        access.field_size,
                    ))?,
                    None => {
                        let scratch = self.ctx.variant_field_scratch.ok_or(
                            LoweringError::VariantFieldScratchMissing {
                                op: "WriteVariantField",
                            },
                        )?;
                        self.emit_variant_field_borrow(&access, ref_off, scratch)?;
                        self.emit(MicroOp::WriteRef {
                            ref_ptr: scratch,
                            src: val_off,
                            size: access.field_size,
                        })?;
                    },
                }
            },
            // Borrowing a variant field directly produces a field reference
            // into `dst`; the borrow derefs the enum and offsets into the heap
            // object (statically or by runtime tag).
            Instr::ImmBorrowVariantField(dst, enum_ty, vfh, src)
            | Instr::MutBorrowVariantField(dst, enum_ty, vfh, src) => {
                let access = self.resolve_variant_field(self.concrete_ty(*enum_ty)?, *vfh)?;
                let src_off = self.slot(*src)?.offset;
                let dst_off = self.def_slot(*dst)?.offset;
                self.emit_variant_field_borrow(&access, src_off, dst_off)?;
            },

            Instr::ForceGC => self.emit(MicroOp::ForceGC)?,
        }

        Ok(())
    }

    /// Advance the Xfer state machine after `instr` has been lowered.
    fn commit_xfer_bindings_after(&mut self, instr: &Instr) {
        // Calls manage their own Xfer state in `lower_call`.
        if !clobbers_xfer(instr) {
            // Release Xfer bindings consumed by this instr's value uses.
            // Place uses leave the slot live, so their binding
            // must persist for the GC to scan at the next safe point.
            for_each_value_use(instr, |s| {
                if let Slot::Xfer(j) = s {
                    self.xfer_bindings[j as usize] = None;
                }
            });
            // Clear-then-commit so an instr that uses and re-defs the
            // same `Xfer(j)` ends with the new value visible.
            // Precolor guarantees distinct `j` per instr; assert it,
            // since a duplicate would drop a `TypedSlot` and corrupt
            // the safe-point heap-pointer map.
            #[cfg(debug_assertions)]
            {
                let mut seen = shared_dsa::UnorderedSet::new();
                for (j, _) in &self.pending_def_binds {
                    debug_assert!(
                        seen.insert(*j),
                        "duplicate Xfer({}) staged for one IR instr",
                        j,
                    );
                }
            }
            for (j, ts) in self.pending_def_binds.drain(..) {
                self.xfer_bindings[j as usize] = Some(ts);
            }
        } else {
            debug_assert!(
                self.pending_def_binds.is_empty(),
                "calls must not leave a pending Xfer def bind",
            );
        }
    }

    /// Derive a [`NativeABI`] for a native call site from its arg/ret
    /// slots.
    fn derive_native_abi(&self, cs: &CallSiteInfo) -> VMResult<NativeABI> {
        let callee_base = self.ctx.frame_data_size + FRAME_METADATA_SIZE as u32;
        let to_slot = |s: &TypedSlot| FrameSlot {
            offset: s.slot.offset.0 - callee_base,
            size: s.slot.size,
        };
        let args: Vec<FrameSlot> = cs.arg_slots.iter().map(to_slot).collect();
        let returns: Vec<FrameSlot> = cs.ret_slots.iter().map(to_slot).collect();
        // Pointer slots among the args, frame-relative, for GC root scanning
        // while the native is the top frame.
        let mut heap_ptr_offsets = Vec::new();
        for s in &cs.arg_slots {
            let base = s.slot.offset.0 - callee_base;
            for rel in type_pointer_offsets(self.ctx.layouts, s.ty)? {
                heap_ptr_offsets.push(FrameOffset(base + rel));
            }
        }
        heap_ptr_offsets.sort_by_key(|o| o.0);
        heap_ptr_offsets.dedup();
        NativeABI::new(
            args,
            returns,
            heap_ptr_offsets,
            cs.required_descriptors.clone(),
        )
        .map_err(|e| VMInternalError::new(LoweringError::NativeAbi(e)))
    }

    /// Lower one call. Args are written by reverse iteration over the
    /// arg list (reverse-order emit); soundness rests on arg positionality
    /// + return monotonicity (see `BlockAnalysis::analyze`), which
    /// guarantee a forward-only dependency graph between arg copies.
    ///
    /// Rets are placed lazily in `xfer_bindings` (Xfer rets) or copied
    /// to Home (Home rets), with no eager hoist into the next call's
    /// arg region. Lazy Xfer placement is sound because: (1) Home
    /// writes target a disjoint region; (2) `xfer_precolor`'s
    /// per-position uniqueness keeps intermediate Xfer dsts off the
    /// live ret slot; and (3) the single-use invariant bounds the
    /// bound value's last read to at or before the next call, so the
    /// callee_base region is free to be reused past that point.
    fn lower_call(&mut self, _func_ir: &FunctionIR, args: &[Slot], rets: &[Slot]) -> VMResult<()> {
        let cs = &self.ctx.call_sites[self.call_site_cursor];

        // Debug: assert the byte-overlap precondition that makes
        // reverse-order emit sound. The upstream invariants on
        // `xfer_precolor` should always satisfy it; this guard catches
        // a layer-skipping regression at the lowering site.
        #[cfg(debug_assertions)]
        {
            let mut copies = Vec::with_capacity(args.len());
            for (j, arg_slot) in args.iter().enumerate() {
                let arg_info = self.slot(*arg_slot)?;
                copies.push(parallel_copy::Copy {
                    src: arg_info.offset,
                    dst: cs.arg_slots[j].slot.offset,
                    width: arg_info.size,
                });
            }
            debug_assert!(
                parallel_copy::reverse_emit_is_safe(&copies),
                "lower_call: reverse-order emit unsafe — arg positionality + \
                 return monotonicity should guarantee a forward-only \
                 dependency graph; an upstream invariant has likely \
                 broken."
            );
        }

        // Decreasing-j arg emit: reverse iteration places each value
        // before any later copy could clobber its source. Identity
        // copies (src == dst) are elided.
        // TODO(perf): consider an optimization: if we can safely do a bulk move here.
        for (j, arg_slot) in args.iter().enumerate().rev() {
            let arg_info = self.slot(*arg_slot)?;
            let dst_off = cs.arg_slots[j].slot.offset;
            if arg_info.offset == dst_off {
                continue;
            }
            debug_assert!(
                arg_info.size > 0,
                "lower_call: zero-width arg type. Every Move-IR type \
                 currently passed through call args has size ≥ 1; a \
                 zero-width copy means an empty/zero-sized type started \
                 flowing through the call ABI."
            );
            if arg_info.size == 8 {
                self.emit(MicroOp::Move8 {
                    dst: dst_off,
                    src: arg_info.offset,
                })?;
            } else {
                self.emit(MicroOp::Move {
                    dst: dst_off,
                    src: arg_info.offset,
                    size: arg_info.size,
                })?;
            }
        }

        match cs.native_idx {
            Some(native_idx) => {
                let abi = self.derive_native_abi(cs)?;
                self.emit(MicroOp::CallNative {
                    native_idx,
                    ty_args: cs.ty_args,
                    abi: Box::new(abi),
                })?;
            },
            None => {
                self.emit(MicroOp::CallIndirect {
                    module_id: cs.callee_module_id,
                    func_name: cs.callee_func_name,
                    ty_args: cs.ty_args,
                })?;
            },
        }
        self.call_site_cursor += 1;

        // Place each ret (Xfer rets are already written by `CallIndirect`).
        self.bind_call_returns(rets, &cs.ret_slots)?;
        Ok(())
    }

    /// Resolve each branch's target label to a real micro-op index, and fill
    /// in its gas fields from [`Self::block_costs`] (indexed by block label).
    ///
    /// An unconditional jump stores its target block's cost; a conditional jump
    /// stores both the taken block's cost (`gas_taken`) and the fallthrough
    /// block's cost (`gas_fallthrough`).
    fn fixup_branches(&mut self) -> VMResult<()> {
        for fixup in &self.branch_fixups {
            let idx = fixup.idx;
            // Extract the encoded label from the op, resolve it, then patch.
            let encoded = match &self.out_buf[idx] {
                MicroOp::Jump { target, .. }
                | MicroOp::JumpNotZeroU64 { target, .. }
                | MicroOp::JumpNotZeroByte { target, .. }
                | MicroOp::JumpZeroByte { target, .. }
                | MicroOp::JumpGreaterEqualU64Imm { target, .. }
                | MicroOp::JumpLessU64Imm { target, .. }
                | MicroOp::JumpGreaterU64Imm { target, .. }
                | MicroOp::JumpLessEqualU64Imm { target, .. }
                | MicroOp::JumpLessU64 { target, .. }
                | MicroOp::JumpGreaterEqualU64 { target, .. }
                | MicroOp::JumpNotEqualU64 { target, .. } => target.0,
                MicroOp::JumpIntCmp(op) => op.target.0,
                MicroOp::JumpValueCmp(op) => op.target.0,
                MicroOp::JumpValueRefCmp(op) => op.target.0,
                other => {
                    let _ = other;
                    return Err(VMInternalError::new(
                        LoweringError::UnexpectedNonBranchOpAtFixup { idx },
                    ));
                },
            };
            let label = decode_label(encoded);
            let resolved = self.resolve_label(label)?;
            // Cost of the block this branch transfers into.
            // Conditional jumps additionally need the fallthrough block's cost.
            let taken = self.block_costs[label as usize];
            let fallthrough = fixup
                .fallthrough_label
                .map(|l| self.block_costs[l.0 as usize]);
            match &mut self.out_buf[idx] {
                MicroOp::Jump { target, gas } => {
                    target.0 = resolved;
                    *gas = taken;
                },
                MicroOp::JumpNotZeroU64 {
                    target,
                    gas_taken,
                    gas_fallthrough,
                    ..
                }
                | MicroOp::JumpNotZeroByte {
                    target,
                    gas_taken,
                    gas_fallthrough,
                    ..
                }
                | MicroOp::JumpZeroByte {
                    target,
                    gas_taken,
                    gas_fallthrough,
                    ..
                }
                | MicroOp::JumpGreaterEqualU64Imm {
                    target,
                    gas_taken,
                    gas_fallthrough,
                    ..
                }
                | MicroOp::JumpLessU64Imm {
                    target,
                    gas_taken,
                    gas_fallthrough,
                    ..
                }
                | MicroOp::JumpGreaterU64Imm {
                    target,
                    gas_taken,
                    gas_fallthrough,
                    ..
                }
                | MicroOp::JumpLessEqualU64Imm {
                    target,
                    gas_taken,
                    gas_fallthrough,
                    ..
                }
                | MicroOp::JumpLessU64 {
                    target,
                    gas_taken,
                    gas_fallthrough,
                    ..
                }
                | MicroOp::JumpGreaterEqualU64 {
                    target,
                    gas_taken,
                    gas_fallthrough,
                    ..
                }
                | MicroOp::JumpNotEqualU64 {
                    target,
                    gas_taken,
                    gas_fallthrough,
                    ..
                } => {
                    target.0 = resolved;
                    *gas_taken = taken;
                    *gas_fallthrough =
                        fallthrough.ok_or(LoweringError::ConditionalBranchNoFallthrough { idx })?;
                },
                MicroOp::JumpIntCmp(op) => {
                    op.target.0 = resolved;
                    op.gas_taken = taken;
                    op.gas_fallthrough =
                        fallthrough.ok_or(LoweringError::ConditionalBranchNoFallthrough { idx })?;
                },
                MicroOp::JumpValueCmp(op) => {
                    op.target.0 = resolved;
                    op.gas_taken = taken;
                    op.gas_fallthrough =
                        fallthrough.ok_or(LoweringError::ConditionalBranchNoFallthrough { idx })?;
                },
                MicroOp::JumpValueRefCmp(op) => {
                    op.target.0 = resolved;
                    op.gas_taken = taken;
                    op.gas_fallthrough =
                        fallthrough.ok_or(LoweringError::ConditionalBranchNoFallthrough { idx })?;
                },
                other => {
                    let _ = other;
                    return Err(VMInternalError::new(
                        LoweringError::UnexpectedNonBranchOpAtFixup { idx },
                    ));
                },
            }
        }
        Ok(())
    }

    fn resolve_label(&self, label: u16) -> VMResult<u32> {
        self.label_map
            .get(label as usize)
            .copied()
            .flatten()
            .ok_or_else(|| VMInternalError::new(LoweringError::UnresolvedLabel { label }))
    }
}

/// Encode a label index as a sentinel value in CodeOffset for later fixup.
/// Uses high bit to mark as unresolved.
fn encode_label(label: u16) -> u32 {
    0x8000_0000 | (label as u32)
}

fn decode_label(encoded: u32) -> u16 {
    debug_assert!(encoded & 0x8000_0000 != 0, "not an encoded label");
    (encoded & 0x7FFF_FFFF) as u16
}

fn imm_to_u64(imm: &ImmValue) -> VMResult<u64> {
    Ok(match imm {
        ImmValue::Bool(true) => 1,
        ImmValue::Bool(false) => 0,
        ImmValue::U8(v) => *v as u64,
        ImmValue::U16(v) => *v as u64,
        ImmValue::U32(v) => *v as u64,
        ImmValue::U64(v) => *v,
        ImmValue::I8(v) => *v as u64,
        ImmValue::I16(v) => *v as u64,
        ImmValue::I32(v) => *v as u64,
        ImmValue::I64(v) => *v as u64,
        ImmValue::U128(_) | ImmValue::U256(_) | ImmValue::I128(_) | ImmValue::I256(_) => {
            return Err(VMInternalError::new(LoweringError::U64FastPathWideImm))
        },
    })
}

/// Extract a u8 shift amount. The rhs of a Move `Shl`/`Shr` is always u8
/// by language spec; anything else is an upstream invariant violation.
fn shift_imm_u8(imm: &ImmValue) -> VMResult<u8> {
    match imm {
        ImmValue::U8(v) => Ok(*v),
        other => {
            let _ = other;
            Err(VMInternalError::new(LoweringError::ShiftImmNotU8))
        },
    }
}

/// Build an [`IntOperand`] slot arm from a Move integer type and frame
/// offset.
fn int_operand_from_slot(ty: &Type, off: FrameOffset) -> VMResult<IntOperand> {
    let int_ty = IntTy::from_type(ty).ok_or(LoweringError::ExpectedIntegerType)?;
    Ok(IntOperand::slot(int_ty, off))
}

enum EqKind {
    /// Integer comparison.
    Int,
    /// Non-integer structural comparison.
    NonIntValue,
    /// Reference: compared structurally through the pointer.
    Ref,
}

/// Classify how an equality operand of the given type is lowered.
fn eq_kind(ty: &Type) -> VMResult<EqKind> {
    Ok(match ty {
        Type::Bool
        | Type::Address
        // Signers are just addresses.
        | Type::Signer
        | Type::U8
        | Type::U16
        | Type::U32
        | Type::U64
        | Type::U128
        | Type::U256
        | Type::I8
        | Type::I16
        | Type::I32
        | Type::I64
        | Type::I128
        | Type::I256 => EqKind::Int,
        Type::Vector { .. } | Type::Nominal { .. } => EqKind::NonIntValue,
        Type::ImmutRef { .. } | Type::MutRef { .. } => EqKind::Ref,
        Type::Function { .. } | Type::TypeParam { .. } => {
            return Err(VMInternalError::new(LoweringError::EqualityUnsupportedType))
        },
    })
}

/// Map an equality [`CmpKind`] to the `negate` flag of the structural-equality
/// ops (`false` for `Eq`, `true` for `Neq`).
fn eq_negate(op: CmpKind) -> VMResult<bool> {
    match op {
        CmpKind::Eq => Ok(false),
        CmpKind::Neq => Ok(true),
        CmpKind::Lt | CmpKind::Le | CmpKind::Gt | CmpKind::Ge => {
            Err(VMInternalError::new(LoweringError::OrderingOnNonScalar))
        },
    }
}

/// Build an [`IntOperand`] for a comparison operand. Integer types delegate to
/// [`int_operand_from_slot`]. `bool` (1 byte), `address` and `signer` (both 32
/// bytes) are flat values with only `==`/`!=` (no ordering), and comparing
/// their bit patterns is exactly value equality, so they reuse the integer
/// compare ops at the matching width.
fn cmp_operand_from_slot(ty: &Type, off: FrameOffset) -> VMResult<IntOperand> {
    match ty {
        Type::Bool => Ok(IntOperand::SlotU8(off)),
        // A signer holds an address, so it compares as a 32-byte value.
        Type::Address | Type::Signer => Ok(IntOperand::SlotU256(off)),
        Type::U8
        | Type::U16
        | Type::U32
        | Type::U64
        | Type::U128
        | Type::U256
        | Type::I8
        | Type::I16
        | Type::I32
        | Type::I64
        | Type::I128
        | Type::I256 => int_operand_from_slot(ty, off),
        Type::ImmutRef { .. }
        | Type::MutRef { .. }
        | Type::Vector { .. }
        | Type::Nominal { .. }
        | Type::Function { .. }
        | Type::TypeParam { .. } => Err(VMInternalError::new(LoweringError::ComparisonNoLowering)),
    }
}

/// Immediate counterpart of [`cmp_operand_from_slot`]: a bool immediate
/// compares as the 1-byte value `0`/`1`.
fn cmp_operand_from_imm(imm: &ImmValue) -> VMResult<IntOperand> {
    match imm {
        ImmValue::Bool(b) => Ok(IntOperand::ImmU8(*b as u8)),
        ImmValue::U8(_)
        | ImmValue::U16(_)
        | ImmValue::U32(_)
        | ImmValue::U64(_)
        | ImmValue::U128(_)
        | ImmValue::U256(_)
        | ImmValue::I8(_)
        | ImmValue::I16(_)
        | ImmValue::I32(_)
        | ImmValue::I64(_)
        | ImmValue::I128(_)
        | ImmValue::I256(_) => int_operand_from_imm(imm),
    }
}

/// Build an [`IntOperand`] imm arm matching `imm`. The destacker emits an
/// `ImmValue` variant whose type matches the typed slot's `Ld*` source,
/// so a 1:1 map is enough here.
fn int_operand_from_imm(imm: &ImmValue) -> VMResult<IntOperand> {
    Ok(match imm {
        ImmValue::U8(v) => IntOperand::ImmU8(*v),
        ImmValue::U16(v) => IntOperand::ImmU16(*v),
        ImmValue::U32(v) => IntOperand::ImmU32(*v),
        ImmValue::U64(v) => IntOperand::ImmU64(*v),
        ImmValue::U128(v) => IntOperand::ImmU128(v.clone()),
        ImmValue::U256(v) => IntOperand::ImmU256(v.clone()),
        ImmValue::I8(v) => IntOperand::ImmI8(*v),
        ImmValue::I16(v) => IntOperand::ImmI16(*v),
        ImmValue::I32(v) => IntOperand::ImmI32(*v),
        ImmValue::I64(v) => IntOperand::ImmI64(*v),
        ImmValue::I128(v) => IntOperand::ImmI128(v.clone()),
        ImmValue::I256(v) => IntOperand::ImmI256(v.clone()),
        ImmValue::Bool(_) => return Err(VMInternalError::new(LoweringError::BoolImmNotInteger)),
    })
}
