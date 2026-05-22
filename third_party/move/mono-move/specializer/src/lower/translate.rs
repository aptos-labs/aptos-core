// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Lowers stackless exec IR to micro-ops.

use super::{
    context::{concrete_type_size, LoweringContext, SlotInfo, TypedSlot},
    gc_layout::type_pointer_offsets,
    parallel_copy,
};
use crate::stackless_exec_ir::{
    instr_utils::{clobbers_xfer, for_each_use},
    BinaryOp, CmpOp, FunctionIR, ImmValue, Instr, Label, Slot, UnaryOp,
};
use anyhow::{bail, Context, Result};
use mono_move_core::{
    types::{strip_ref, view_type, InternedType, Type},
    CodeOffset, FrameLayoutInfo, FrameOffset, IntBinaryOp, IntNegateOp, IntOperand, IntShiftOp,
    IntTy, MicroOp, SafePointEntry, ShiftOperand,
};

/// Lower a slot-allocated function to its micro-op form.
///
/// Returns `(ops, safe_points)`:
/// - `ops` — pre-instrumentation micro-ops in emission order.
/// - `safe_points` — one entry **per allocating micro-op only**,
///   in code-offset order. Non-allocating PCs are not represented;
///   the vector is sparse. Each entry's `code_offset` indexes
///   directly into `ops`.
pub fn lower_function(
    func_ir: &FunctionIR,
    ctx: &LoweringContext,
) -> Result<(Vec<MicroOp>, Vec<SafePointEntry>)> {
    let mut state = LoweringState::new(func_ir, ctx);
    for block in &func_ir.blocks {
        // Xfer slots are block-local.
        debug_assert!(
            state.xfer_bindings.iter().all(Option::is_none),
            "xfer_bindings not fully cleared at block boundary",
        );
        debug_assert!(
            state.pending_def_bind.is_none(),
            "pending_def_bind not committed at block boundary",
        );
        state.xfer_bindings.fill(None);
        state.label_map[block.label.0 as usize] = Some(state.out_buf.len() as u32);
        for instr in &block.instrs {
            state.lower_instr(func_ir, instr)?;
            state.commit_xfer_bindings_after(instr);
        }
    }
    state.fixup_branches()?;
    Ok((state.out_buf, state.pending_safe_points))
}

struct LoweringState<'a> {
    /// Read-only frame layout for the function being lowered.
    ctx: &'a LoweringContext,
    /// Output buffer. Micro-ops are appended in emit order.
    out_buf: Vec<MicroOp>,
    /// `Label(i)` → index in `out_buf` where block `i` begins. Dense
    /// (one entry per block); `None` until that block has been lowered.
    /// Read by `fixup_branches` to resolve branch targets after all
    /// blocks are emitted.
    label_map: Vec<Option<u32>>,
    /// Indices into `out_buf` of branch micro-ops whose `target` was
    /// emitted with a placeholder label encoding. `fixup_branches`
    /// walks this list and rewrites each target to the real micro-op
    /// index from `label_map`.
    branch_fixups: Vec<usize>,
    /// Monotonic cursor into `ctx.call_sites`. Before a call is lowered,
    /// it points at *that* call's `CallSiteInfo`; immediately after the
    /// `CallFunc` op is emitted, it advances by one.
    call_site_cursor: usize,
    /// Types of the function IR's home (frame-resident) slots, indexed
    /// by Home slot id.
    home_slot_types: &'a [InternedType],
    /// `Some(TypedSlot)` while `Slot::Xfer(j)` holds a fully-written
    /// live value visible to the GC; `None` otherwise. Length
    /// `ctx.num_xfer_positions`.
    xfer_bindings: Vec<Option<TypedSlot>>,
    /// Holds at most one pending Xfer binding. `Some((j, ts))`
    /// means `Xfer(j)` is to be bound to typed slot `ts`; `None`
    /// means no binding is pending.
    pending_def_bind: Option<(u16, TypedSlot)>,
    /// Safe-point entries in code-offset order. Populated by `emit`
    /// when `op.is_allocating()`.
    pending_safe_points: Vec<SafePointEntry>,
}

impl<'a> LoweringState<'a> {
    fn new(func_ir: &'a FunctionIR, ctx: &'a LoweringContext) -> Self {
        let num_xfer_positions = ctx.num_xfer_positions as usize;
        LoweringState {
            ctx,
            out_buf: Vec::new(),
            label_map: vec![None; func_ir.blocks.len()],
            branch_fixups: Vec::new(),
            call_site_cursor: 0,
            home_slot_types: &func_ir.home_slot_types,
            xfer_bindings: vec![None; num_xfer_positions],
            pending_def_bind: None,
            pending_safe_points: Vec::new(),
        }
    }

    fn xfer_binding(&self, j: u16) -> Result<TypedSlot> {
        self.xfer_bindings[j as usize]
            .with_context(|| format!("Xfer({}) read without a prior def in this block", j))
    }

    fn slot(&self, slot: Slot) -> Result<SlotInfo> {
        Ok(match slot {
            Slot::Home(i) => self.ctx.home_slots[i as usize],
            Slot::Xfer(j) => self.xfer_binding(j)?.slot,
            Slot::Vid(i) => bail!("Vid({}) in post-allocation IR", i),
        })
    }

    /// Returns layout info for a destination slot. For
    /// `Slot::Xfer(j)`, stages a pending binding from `Xfer(j)` to
    /// the typed slot at arg position `j` of the upcoming call.
    /// Errors if a binding is already staged or for `Slot::Vid`.
    fn def_slot(&mut self, slot: Slot) -> Result<SlotInfo> {
        Ok(match slot {
            Slot::Home(i) => self.ctx.home_slots[i as usize],
            Slot::Xfer(j) => {
                let call_site = &self.ctx.call_sites[self.call_site_cursor];
                let typed_slot = call_site.arg_slots[j as usize];
                debug_assert!(
                    self.pending_def_bind.is_none(),
                    "second Xfer def_slot in one IR instr",
                );
                self.pending_def_bind = Some((j, typed_slot));
                typed_slot.slot
            },
            Slot::Vid(i) => bail!("Vid({}) in post-allocation IR", i),
        })
    }

    /// Append `op` to the output buffer. For allocating `op`s,
    /// also emit a paired `SafePointEntry` whose `code_offset` is
    /// `op`'s index in the buffer and whose `heap_ptr_offsets`
    /// are derived from the current `xfer_bindings`.
    fn emit(&mut self, op: MicroOp) -> Result<()> {
        if op.is_allocating() {
            let code_offset = CodeOffset(self.out_buf.len() as u32);
            let mut heap_ptr_offsets = Vec::with_capacity(self.xfer_bindings.len());
            for ts in self.xfer_bindings.iter().flatten() {
                let rels = type_pointer_offsets(ts.ty).with_context(|| {
                    format!(
                        "deriving safe-point heap pointer offsets at code_offset {}",
                        code_offset.0
                    )
                })?;
                heap_ptr_offsets
                    .extend(rels.into_iter().map(|r| FrameOffset(ts.slot.offset.0 + r)));
            }
            // TODO: revisit the need to sort and dedup.
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

    /// Interned-type corresponding to `slot`.
    fn slot_interned_type(&self, slot: Slot) -> Result<InternedType> {
        Ok(match slot {
            Slot::Home(i) => self.home_slot_types[i as usize],
            Slot::Xfer(j) => self.xfer_binding(j)?.ty,
            Slot::Vid(i) => bail!("Vid({}) in post-allocation IR", i),
        })
    }

    /// Canonical [`Type`] variant of `slot`. Use [`Self::slot_interned_type`]
    /// when an interned pointer is needed instead.
    fn slot_type(&self, slot: Slot) -> Result<&'static Type> {
        Ok(view_type(self.slot_interned_type(slot)?))
    }

    /// Size in bytes of `ref_slot`'s pointee.
    fn ref_pointee_size(&self, ref_slot: Slot) -> Result<u32> {
        concrete_type_size(
            strip_ref(self.slot_interned_type(ref_slot)?)?,
            "ref pointee type",
        )
    }

    /// Returns true if the type is exactly 8 bytes wide. Used to gate the
    /// u64-shaped micro-ops (`AddU64`, `JumpLessU64`, etc.), which read and
    /// write 8 bytes unconditionally and so cannot be reused for narrower
    /// types without overrunning into adjacent slots.
    fn is_u64_sized(ty: &Type) -> bool {
        matches!(ty.size_and_align(), Some((8, _)))
    }

    /// Emit one byte-copy from `src` to `dst_offset`. Caller is
    /// responsible for ensuring no other concurrent move clobbers the
    /// source bytes.
    fn emit_single_move(&mut self, dst_offset: FrameOffset, src: SlotInfo) -> Result<()> {
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

    /// Lower one IR instruction.
    fn lower_instr(&mut self, func_ir: &FunctionIR, instr: &Instr) -> Result<()> {
        match instr {
            // --- Loads ---
            Instr::LdU64(dst, v) => {
                let dst_info = self.def_slot(*dst)?;
                self.emit(MicroOp::StoreImm8 {
                    dst: dst_info.offset,
                    imm: *v,
                })?;
            },
            Instr::LdTrue(dst) => {
                let dst_info = self.def_slot(*dst)?;
                self.emit(MicroOp::StoreImm8 {
                    dst: dst_info.offset,
                    imm: 1,
                })?;
            },
            Instr::LdFalse(dst) => {
                let dst_info = self.def_slot(*dst)?;
                self.emit(MicroOp::StoreImm8 {
                    dst: dst_info.offset,
                    imm: 0,
                })?;
            },
            Instr::LdU8(dst, v) => {
                let dst_info = self.def_slot(*dst)?;
                self.emit(MicroOp::StoreImm8 {
                    dst: dst_info.offset,
                    imm: *v as u64,
                })?;
            },
            Instr::LdU16(dst, v) => {
                let dst_info = self.def_slot(*dst)?;
                self.emit(MicroOp::StoreImm8 {
                    dst: dst_info.offset,
                    imm: *v as u64,
                })?;
            },
            Instr::LdU32(dst, v) => {
                let dst_info = self.def_slot(*dst)?;
                self.emit(MicroOp::StoreImm8 {
                    dst: dst_info.offset,
                    imm: *v as u64,
                })?;
            },
            Instr::LdI8(dst, v) => {
                let dst_info = self.def_slot(*dst)?;
                self.emit(MicroOp::StoreImm8 {
                    dst: dst_info.offset,
                    imm: *v as u64,
                })?;
            },
            Instr::LdI16(dst, v) => {
                let dst_info = self.def_slot(*dst)?;
                self.emit(MicroOp::StoreImm8 {
                    dst: dst_info.offset,
                    imm: *v as u64,
                })?;
            },
            Instr::LdI32(dst, v) => {
                let dst_info = self.def_slot(*dst)?;
                self.emit(MicroOp::StoreImm8 {
                    dst: dst_info.offset,
                    imm: *v as u64,
                })?;
            },
            Instr::LdI64(dst, v) => {
                let dst_info = self.def_slot(*dst)?;
                self.emit(MicroOp::StoreImm8 {
                    dst: dst_info.offset,
                    imm: *v as u64,
                })?;
            },

            // --- Copy/Move ---
            Instr::Copy(dst, src) | Instr::Move(dst, src) => {
                let src_info = self.slot(*src)?;
                let dst_info = self.def_slot(*dst)?;
                self.emit_single_move(dst_info.offset, src_info)?;
            },

            // --- Binary ops ---
            Instr::BinaryOp(dst, op, lhs, rhs) => {
                // TODO: BinaryOp / BinaryOpImm share most of their shape
                // (operand resolution + per-kind emit); consider factoring
                // out the common skeleton once cast / cmp ops settle.
                let lhs_info = self.slot(*lhs)?;
                let rhs_info = self.slot(*rhs)?;
                let dst_info = self.def_slot(*dst)?;
                let lhs_ty = self.slot_type(*lhs)?;
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
                                bail!("BinaryOp {:?}: bitwise on a signed value is invalid", op);
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
                                    bail!("internal: unexpected op in arith/bitwise arm")
                                },
                            })?;
                        },
                        BinaryOp::Shl | BinaryOp::Shr => {
                            let ty = IntTy::from_type(lhs_ty)
                                .filter(|t| !t.is_signed())
                                .ok_or_else(|| {
                                    anyhow::anyhow!(
                                        "BinaryOp {:?}: requires an unsigned non-u64 integer type",
                                        op
                                    )
                                })?;
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
                                | BinaryOp::And => bail!("internal: unexpected op in shift arm"),
                            })?;
                        },
                        // Comparison-to-register and logical and/or are not
                        // yet lowered for any integer width.
                        BinaryOp::Cmp(_) | BinaryOp::Or | BinaryOp::And => {
                            bail!("BinaryOp {:?} not yet lowered", op)
                        },
                    }
                }
            },

            // --- Binary ops with immediate ---
            Instr::BinaryOpImm(dst, op, src, imm) => {
                // TODO: see [`Instr::BinaryOp`] above on factoring out the
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
                                bail!("BinaryOpImm {:?}: bitwise on a signed value is invalid", op,);
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
                                    bail!("internal: unexpected op in arith/bitwise arm")
                                },
                            })?;
                        },
                        BinaryOp::Shl | BinaryOp::Shr => {
                            let ty = IntTy::from_type(src_ty)
                                .filter(|t| !t.is_signed())
                                .ok_or_else(|| {
                                    anyhow::anyhow!(
                                    "BinaryOpImm {:?}: requires an unsigned non-u64 integer type",
                                    op
                                )
                                })?;
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
                                | BinaryOp::And => bail!("internal: unexpected op in shift arm"),
                            })?;
                        },
                        BinaryOp::Cmp(_) | BinaryOp::Or | BinaryOp::And => {
                            bail!("BinaryOpImm {:?} not yet lowered", op)
                        },
                    }
                }
            },

            // --- Unary ops ---
            Instr::UnaryOp(dst, UnaryOp::Negate, src) => {
                let src_ty = self.slot_type(*src)?;
                let signed_ty = IntTy::from_type(src_ty)
                    .filter(|t| t.is_signed())
                    .ok_or_else(|| {
                        anyhow::anyhow!("UnaryOp::Negate requires a signed integer type")
                    })?;
                let src_info = self.slot(*src)?;
                let dst_info = self.def_slot(*dst)?;
                self.emit(MicroOp::IntNegate(IntNegateOp {
                    ty: signed_ty,
                    dst: dst_info.offset,
                    src: src_info.offset,
                }))?;
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
                // TODO: `size` could equivalently come from `dst_info.size`
                // (the loaded value's slot) — IR typing forces it to equal
                // the ref's pointee size. `ref_pointee_size` is the more
                // type-faithful path; `dst_info.size` would be cheaper.
                let size = self.ref_pointee_size(*ref_src)?;
                let ref_info = self.slot(*ref_src)?;
                let dst_info = self.def_slot(*dst)?;
                self.emit(MicroOp::ReadRef {
                    dst: dst_info.offset,
                    ref_ptr: ref_info.offset,
                    size,
                })?;
            },
            Instr::WriteRef(ref_dst, src) => {
                // TODO: `size` could equivalently come from `src_info.size`
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
                let idx = self.out_buf.len();
                self.branch_fixups.push(idx);
                self.emit(MicroOp::Jump {
                    target: CodeOffset(encode_label(*l)),
                })?;
            },
            Instr::BrTrue(Label(l), cond) => {
                let cond_info = self.slot(*cond)?;
                let idx = self.out_buf.len();
                self.branch_fixups.push(idx);
                // [TODO]: we are representing booleans as 0/1 in u64 slots here.
                // This needs to be updated with a more compact boolean representation.
                self.emit(MicroOp::JumpNotZeroU64 {
                    target: CodeOffset(encode_label(*l)),
                    src: cond_info.offset,
                })?;
            },
            Instr::BrFalse(Label(l), cond) => {
                let cond_info = self.slot(*cond)?;
                let idx = self.out_buf.len();
                self.branch_fixups.push(idx);
                // [TODO]: we are representing booleans as 0/1 in u64 slots here.
                // This needs to be updated with a more compact boolean representation.
                self.emit(MicroOp::JumpLessU64Imm {
                    target: CodeOffset(encode_label(*l)),
                    src: cond_info.offset,
                    imm: 1,
                })?;
            },

            // --- Fused compare+branch ---
            Instr::BrCmp(Label(l), op, lhs, rhs) => {
                let lhs_ty = self.slot_type(*lhs)?;
                if Self::is_u64_sized(lhs_ty) {
                    let lhs_info = self.slot(*lhs)?;
                    let rhs_info = self.slot(*rhs)?;
                    let idx = self.out_buf.len();
                    self.branch_fixups.push(idx);
                    match op {
                        CmpOp::Lt => self.emit(MicroOp::JumpLessU64 {
                            target: CodeOffset(encode_label(*l)),
                            lhs: lhs_info.offset,
                            rhs: rhs_info.offset,
                        })?,
                        CmpOp::Ge => self.emit(MicroOp::JumpGreaterEqualU64 {
                            target: CodeOffset(encode_label(*l)),
                            lhs: lhs_info.offset,
                            rhs: rhs_info.offset,
                        })?,
                        // x > y ↔ y < x
                        CmpOp::Gt => self.emit(MicroOp::JumpLessU64 {
                            target: CodeOffset(encode_label(*l)),
                            lhs: rhs_info.offset,
                            rhs: lhs_info.offset,
                        })?,
                        // x <= y ↔ y >= x
                        CmpOp::Le => self.emit(MicroOp::JumpGreaterEqualU64 {
                            target: CodeOffset(encode_label(*l)),
                            lhs: rhs_info.offset,
                            rhs: lhs_info.offset,
                        })?,
                        CmpOp::Neq => self.emit(MicroOp::JumpNotEqualU64 {
                            target: CodeOffset(encode_label(*l)),
                            lhs: lhs_info.offset,
                            rhs: rhs_info.offset,
                        })?,
                        CmpOp::Eq => {
                            bail!("BrCmp Eq for u64-sized type not yet lowered")
                        },
                    }
                } else {
                    bail!("BrCmp for non-u64 type not yet lowered");
                }
            },
            Instr::BrCmpImm(Label(l), op, src, imm) => {
                let src_ty = self.slot_type(*src)?;
                if Self::is_u64_sized(src_ty) {
                    let src_info = self.slot(*src)?;
                    let v = imm_to_u64(imm)?;
                    let idx = self.out_buf.len();
                    self.branch_fixups.push(idx);
                    match op {
                        CmpOp::Ge => self.emit(MicroOp::JumpGreaterEqualU64Imm {
                            target: CodeOffset(encode_label(*l)),
                            src: src_info.offset,
                            imm: v,
                        })?,
                        CmpOp::Lt => self.emit(MicroOp::JumpLessU64Imm {
                            target: CodeOffset(encode_label(*l)),
                            src: src_info.offset,
                            imm: v,
                        })?,
                        CmpOp::Gt => self.emit(MicroOp::JumpGreaterU64Imm {
                            target: CodeOffset(encode_label(*l)),
                            src: src_info.offset,
                            imm: v,
                        })?,
                        CmpOp::Le => self.emit(MicroOp::JumpLessEqualU64Imm {
                            target: CodeOffset(encode_label(*l)),
                            src: src_info.offset,
                            imm: v,
                        })?,
                        CmpOp::Eq | CmpOp::Neq => {
                            bail!("BrCmpImm {:?} for u64-sized type not yet lowered", op)
                        },
                    }
                } else {
                    bail!("BrCmpImm for non-u64 type not yet lowered");
                }
            },

            // --- Calls ---
            Instr::Call(rets, _handle_idx, args) => {
                self.lower_call(func_ir, args, rets)?;
            },
            Instr::CallGeneric(rets, _inst_idx, args) => {
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
                let elem_size = concrete_type_size(*elem_ty, "vector elem type")?;
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
                let elem_size = concrete_type_size(*elem_ty, "vector elem type")?;
                let vec_ref_info = self.slot(*vec_ref)?;
                let dst_info = self.def_slot(*dst)?;
                self.emit(MicroOp::VecPopBack {
                    dst: dst_info.offset,
                    vec_ref: vec_ref_info.offset,
                    elem_size,
                })?;
            },
            Instr::VecPack(dst, _elem_ty, _count, elems) => {
                if !elems.is_empty() {
                    bail!("VecPack with elements not yet lowered");
                }
                let dst_info = self.def_slot(*dst)?;
                self.emit(MicroOp::VecNew {
                    dst: dst_info.offset,
                })?;
            },
            Instr::VecPushBack(elem_ty, vec_ref, val) => {
                let elem_size = concrete_type_size(*elem_ty, "vector elem type")?;
                let vec_ty = strip_ref(self.slot_interned_type(*vec_ref)?)?;
                let descriptor_id = self.ctx.vec_descriptor_id(vec_ty).ok_or_else(|| {
                    anyhow::anyhow!(
                        "VecPushBack: no descriptor published for this vector type \
                         (its element type may be generic or have unresolved layout)"
                    )
                })?;
                let vec_ref_info = self.slot(*vec_ref)?;
                let val_info = self.slot(*val)?;
                self.emit(MicroOp::VecPushBack {
                    vec_ref: vec_ref_info.offset,
                    elem: val_info.offset,
                    elem_size,
                    descriptor_id,
                })?;
            },

            _ => bail!("instruction {} not yet lowered", instr.opcode_name()),
        }

        Ok(())
    }

    /// Advance the Xfer state machine after `instr` has been lowered.
    fn commit_xfer_bindings_after(&mut self, instr: &Instr) {
        // Calls manage their own Xfer state in `lower_call`.
        if !clobbers_xfer(instr) {
            // Release Xfer bindings consumed by this instr's uses.
            for_each_use(instr, |s| {
                if let Slot::Xfer(j) = s {
                    self.xfer_bindings[j as usize] = None;
                }
            });
            // Clear-then-commit so an instr that uses and re-defs
            // the same `Xfer(j)` ends with the new value visible.
            if let Some((j, ts)) = self.pending_def_bind.take() {
                self.xfer_bindings[j as usize] = Some(ts);
            }
        } else {
            debug_assert!(
                self.pending_def_bind.is_none(),
                "calls must not leave a pending Xfer def bind",
            );
        }
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
    fn lower_call(&mut self, _func_ir: &FunctionIR, args: &[Slot], rets: &[Slot]) -> Result<()> {
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
        // [TODO] Consider an optimization: if we can safely do a bulk move here.
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

        self.emit(MicroOp::CallIndirect {
            module_id: cs.callee_module_id,
            func_name: cs.callee_func_name,
            ty_args: cs.ty_args,
        })?;
        self.call_site_cursor += 1;

        // Clear all Xfer bindings (calls clobber the entire callee
        // region). The ret loop below re-binds the positions this
        // call returns to.
        self.xfer_bindings.fill(None);

        // Place each ret. Xfer rets are recorded in `xfer_bindings`
        // immediately — `CallIndirect` has already written the
        // values.
        for (k, ret_slot) in rets.iter().enumerate() {
            match *ret_slot {
                Slot::Xfer(j) => {
                    self.xfer_bindings[j as usize] = Some(cs.ret_slots[k]);
                },
                Slot::Home(i) => {
                    let src = cs.ret_slots[k].slot;
                    let dst = self.ctx.home_slots[i as usize];
                    self.emit_single_move(dst.offset, src)?;
                },
                Slot::Vid(_) => bail!("Vid slot in post-allocation IR"),
            }
        }
        Ok(())
    }

    fn fixup_branches(&mut self) -> Result<()> {
        for &idx in &self.branch_fixups {
            // Extract the encoded label from the op, resolve it, then patch.
            let encoded = match &self.out_buf[idx] {
                MicroOp::Jump { target }
                | MicroOp::JumpNotZeroU64 { target, .. }
                | MicroOp::JumpGreaterEqualU64Imm { target, .. }
                | MicroOp::JumpLessU64Imm { target, .. }
                | MicroOp::JumpGreaterU64Imm { target, .. }
                | MicroOp::JumpLessEqualU64Imm { target, .. }
                | MicroOp::JumpLessU64 { target, .. }
                | MicroOp::JumpGreaterEqualU64 { target, .. }
                | MicroOp::JumpNotEqualU64 { target, .. } => target.0,
                other => bail!("unexpected non-branch op at fixup index {}: {}", idx, other),
            };
            let label = decode_label(encoded);
            let resolved = self.resolve_label(label)?;
            match &mut self.out_buf[idx] {
                MicroOp::Jump { target }
                | MicroOp::JumpNotZeroU64 { target, .. }
                | MicroOp::JumpGreaterEqualU64Imm { target, .. }
                | MicroOp::JumpLessU64Imm { target, .. }
                | MicroOp::JumpGreaterU64Imm { target, .. }
                | MicroOp::JumpLessEqualU64Imm { target, .. }
                | MicroOp::JumpLessU64 { target, .. }
                | MicroOp::JumpGreaterEqualU64 { target, .. }
                | MicroOp::JumpNotEqualU64 { target, .. } => target.0 = resolved,
                _ => unreachable!(),
            }
        }
        Ok(())
    }

    fn resolve_label(&self, label: u16) -> Result<u32> {
        self.label_map
            .get(label as usize)
            .copied()
            .flatten()
            .ok_or_else(|| anyhow::anyhow!("unresolved label L{}", label))
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

fn imm_to_u64(imm: &ImmValue) -> Result<u64> {
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
            bail!("u64 fast path received a wide imm — ill-typed IR")
        },
    })
}

/// Extract a u8 shift amount. The rhs of a Move `Shl`/`Shr` is always u8
/// by language spec; anything else is an upstream invariant violation.
fn shift_imm_u8(imm: &ImmValue) -> Result<u8> {
    match imm {
        ImmValue::U8(v) => Ok(*v),
        other => bail!("shift immediate must be u8, got {:?}", other),
    }
}

/// Build an [`IntOperand`] slot arm from a Move integer type and frame
/// offset.
fn int_operand_from_slot(ty: &Type, off: FrameOffset) -> Result<IntOperand> {
    let int_ty = IntTy::from_type(ty).ok_or_else(|| anyhow::anyhow!("expected an integer type"))?;
    Ok(IntOperand::slot(int_ty, off))
}

/// Build an [`IntOperand`] imm arm matching `imm`. The destacker emits an
/// `ImmValue` variant whose type matches the typed slot's `Ld*` source,
/// so a 1:1 map is enough here.
fn int_operand_from_imm(imm: &ImmValue) -> Result<IntOperand> {
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
        ImmValue::Bool(_) => bail!("bool ImmValue cannot be an integer operand"),
    })
}
