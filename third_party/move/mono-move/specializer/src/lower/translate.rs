// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Lowers stackless exec IR to micro-ops.

use super::{
    context::{concrete_type_size, CallSiteInfo, LoweringContext, TypedSlot},
    gc_layout::type_pointer_offsets,
    parallel_copy,
};
use crate::stackless_exec_ir::{
    instr_utils::{clobbers_xfer, for_each_value_use},
    BinaryOp, CmpOp, FunctionIR, ImmValue, Instr, Label, Slot, UnaryOp,
};
use anyhow::{anyhow, bail, Context, Result};
use mono_move_core::{
    native::{FrameSlot, NativeABI},
    types::{strip_ref, view_type, InternedType, Type},
    CallClosureOp, ClosureFuncRef, CmpKind, CodeOffset, FrameLayoutInfo, FrameOffset, IntBinaryOp,
    IntCastOp, IntCmpOp, IntNegateOp, IntOperand, IntShiftOp, IntTy, JumpIntCmpOp, JumpValueCmpOp,
    JumpValueRefCmpOp, MicroOp, PackClosureOp, SafePointEntry, ShiftOperand, SizedSlot, ValueCmpOp,
    ValueRefCmpOp, FRAME_METADATA_SIZE,
};
use move_binary_format::file_format::{ConstantPoolIndex, FieldHandleIndex};

/// Validates that a primitive constant's BCS bytes are exactly `N` wide and
/// returns them as a fixed array. Fixed-width integers and `address` encode
/// with no length prefix, so these bytes are already the in-memory
/// representation the matching `StoreImm` expects.
fn const_imm<const N: usize>(idx: ConstantPoolIndex, bytes: &[u8]) -> Result<[u8; N]> {
    bytes.try_into().map_err(|_| {
        anyhow!(
            "LdConst at constant pool index {}: expected {}-byte constant data, got {}",
            idx.0,
            N,
            bytes.len()
        )
    })
}

/// Lower a slot-allocated function to its micro-op form.
///
/// Returns `(ops, safe_points)`:
/// - `ops` — pre-instrumentation micro-ops in emission order.
/// - `safe_points` — one entry **per allocating micro-op only**,
///   in code-offset order. Non-allocating PCs are not represented;
///   the vector is sparse. Each entry's `code_offset` indexes
///   directly into `ops`.
pub(super) fn lower_function(
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
            state.pending_def_binds.is_empty(),
            "pending_def_binds not committed at block boundary",
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
    ctx: &'a LoweringContext<'a>,
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
    /// Monotonic cursor into `ctx.closure_pack_sites`; advances once per
    /// lowered `Instr::PackClosure`.
    closure_pack_cursor: usize,
    /// Monotonic cursor into `ctx.closure_call_sites`; advances once per
    /// lowered `Instr::CallClosure`.
    closure_call_cursor: usize,
    /// Types of the function IR's home (frame-resident) slots, indexed
    /// by Home slot id.
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

impl<'a> LoweringState<'a> {
    fn new(func_ir: &'a FunctionIR, ctx: &'a LoweringContext<'a>) -> Self {
        let num_xfer_positions = ctx.num_xfer_positions as usize;
        LoweringState {
            ctx,
            out_buf: Vec::new(),
            label_map: vec![None; func_ir.blocks.len()],
            branch_fixups: Vec::new(),
            call_site_cursor: 0,
            closure_pack_cursor: 0,
            closure_call_cursor: 0,
            home_slot_types: &func_ir.home_slot_types,
            xfer_bindings: vec![None; num_xfer_positions],
            pending_def_binds: Vec::new(),
            pending_safe_points: Vec::new(),
        }
    }

    /// Resolve a `FieldHandleIndex` against the struct type `struct_ty`
    /// and return `(field_byte_offset, field_byte_size)`.
    fn resolve_field(&self, struct_ty: InternedType, fh: FieldHandleIndex) -> Result<(u32, u32)> {
        let layout = view_type(struct_ty)
            .layout()
            .ok_or_else(|| anyhow::anyhow!("struct type has no layout populated"))?;
        let fields = layout
            .field_layouts()
            .ok_or_else(|| anyhow::anyhow!("nominal type is not a struct (no field layouts)"))?;
        let pos = self.ctx.module.field_position_at(fh) as usize;
        let field = fields
            .get(pos)
            .ok_or_else(|| anyhow::anyhow!("field index {} out of range for struct", pos))?;
        let (size, _) = view_type(field.ty())
            .size_and_align()
            .ok_or_else(|| anyhow::anyhow!("field type has no concrete size"))?;
        Ok((field.offset, size))
    }

    fn xfer_binding(&self, j: u16) -> Result<TypedSlot> {
        self.xfer_bindings[j as usize]
            .with_context(|| format!("Xfer({}) read without a prior def in this block", j))
    }

    fn slot(&self, slot: Slot) -> Result<SizedSlot> {
        Ok(match slot {
            Slot::Home(i) => self.ctx.home_slots[i as usize],
            Slot::Xfer(j) => self.xfer_binding(j)?.slot,
            Slot::Vid(i) => bail!("Vid({}) in post-allocation IR", i),
        })
    }

    /// Returns layout info for a destination slot. For
    /// `Slot::Xfer(j)`, stages a pending binding from `Xfer(j)` to
    /// the typed slot at arg position `j` of the upcoming call.
    /// Errors for `Slot::Vid`.
    fn def_slot(&mut self, slot: Slot) -> Result<SizedSlot> {
        Ok(match slot {
            Slot::Home(i) => self.ctx.home_slots[i as usize],
            Slot::Xfer(j) => {
                let call_site = &self.ctx.call_sites[self.call_site_cursor];
                let typed_slot = call_site.arg_slots[j as usize];
                self.pending_def_binds.push((j, typed_slot));
                typed_slot.slot
            },
            Slot::Vid(i) => bail!("Vid({}) in post-allocation IR", i),
        })
    }

    /// Resolves each `slot` to its [`SizedSlot`] frame layout.
    fn slots_to_sized_slots(&self, slots: &[Slot]) -> Result<Vec<SizedSlot>> {
        slots.iter().map(|slot| self.slot(*slot)).collect()
    }

    /// Place a call's return values; `ret_slots` are their caller-frame
    /// locations. The call clobbers the whole callee region, so clear all Xfer
    /// bindings, then re-bind each `Xfer` ret (for GC) and copy each `Home` in.
    fn bind_call_returns(&mut self, rets: &[Slot], ret_slots: &[TypedSlot]) -> Result<()> {
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
                Slot::Vid(_) => bail!("Vid slot in post-allocation IR"),
            }
        }
        Ok(())
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

    /// Emit an `IntCast` to `to` from `src` into `dst`. The source type comes
    /// from `src`'s slot type and the `to` type is supplied by the caller.
    fn lower_cast(&mut self, dst: Slot, src: Slot, to: IntTy) -> Result<()> {
        let from = IntTy::from_type(self.slot_type(src)?)
            .ok_or_else(|| anyhow::anyhow!("cast source must be an integer type"))?;
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
    fn ref_pointee_size(&self, ref_slot: Slot) -> Result<u32> {
        concrete_type_size(
            strip_ref(self.slot_interned_type(ref_slot)?)?,
            "ref pointee type",
        )
    }

    /// Emit one byte-copy from `src` to `dst_offset`. Caller is
    /// responsible for ensuring no other concurrent move clobbers the
    /// source bytes.
    fn emit_single_move(&mut self, dst_offset: FrameOffset, src: SizedSlot) -> Result<()> {
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
        cmp: CmpOp,
        dst: FrameOffset,
        lhs: FrameOffset,
        rhs: IntOperand,
    ) -> Result<()> {
        self.emit(MicroOp::IntCmp(IntCmpOp {
            op: cmp_kind(cmp),
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
        cmp: CmpOp,
        lhs: FrameOffset,
        rhs: IntOperand,
    ) -> Result<()> {
        self.emit(MicroOp::JumpIntCmp(JumpIntCmpOp {
            target,
            op: cmp_kind(cmp),
            lhs,
            rhs,
        }))
    }

    /// Lower one IR instruction.
    fn lower_instr(&mut self, func_ir: &FunctionIR, instr: &Instr) -> Result<()> {
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
                // TODO(endianness): revisit this when we fix the endianness
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
                    | Type::TypeParam { .. } => bail!(
                        "LdConst at constant pool index {}: constant type is not \
                         permitted by the bytecode verifier",
                        idx.0,
                    ),
                }
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
                                    ty: strip_ref(lhs_interned)?,
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
                                bail!("BinaryOpImm {:?}: imm must be bool", op);
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
            Instr::UnaryOp(dst, op, src) if op.cast_target_ty().is_some() => {
                let to = op
                    .cast_target_ty()
                    .expect("guard above ensures this is a cast");
                self.lower_cast(*dst, *src, to)?;
            },
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
            Instr::UnaryOp(dst, UnaryOp::FreezeRef, src) => {
                // Runtime no-op: &mut T and &T share the same 16-byte
                // fat-pointer representation. Propagate the slot value.
                // TODO: fold this away at the stackless exec IR level so
                // lowering emits nothing at all.
                let src_info = self.slot(*src)?;
                let dst_info = self.def_slot(*dst)?;
                self.emit_single_move(dst_info.offset, src_info)?;
            },
            Instr::UnaryOp(dst, UnaryOp::Not, src) => {
                let src_info = self.slot(*src)?;
                let dst_info = self.def_slot(*dst)?;
                self.emit(MicroOp::BoolNot {
                    dst: dst_info.offset,
                    src: src_info.offset,
                })?;
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
                self.emit(MicroOp::JumpNotZeroByte {
                    target: CodeOffset(encode_label(*l)),
                    src: cond_info.offset,
                })?;
            },
            Instr::BrFalse(Label(l), cond) => {
                let cond_info = self.slot(*cond)?;
                let idx = self.out_buf.len();
                self.branch_fixups.push(idx);
                self.emit(MicroOp::JumpZeroByte {
                    target: CodeOffset(encode_label(*l)),
                    src: cond_info.offset,
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
                let idx = self.out_buf.len();
                self.branch_fixups.push(idx);
                // Fast path: unsigned `u64` ordering / not-equal use the
                // specialized jumps. Everything else goes through the general
                // `JumpIntCmp`, which dispatches on the operand type.
                match (lhs_ty.is_u64(), op) {
                    (true, CmpOp::Lt) => self.emit(MicroOp::JumpLessU64 {
                        target,
                        lhs: lhs_off,
                        rhs: rhs_off,
                    })?,
                    (true, CmpOp::Ge) => self.emit(MicroOp::JumpGreaterEqualU64 {
                        target,
                        lhs: lhs_off,
                        rhs: rhs_off,
                    })?,
                    // x > y ↔ y < x
                    (true, CmpOp::Gt) => self.emit(MicroOp::JumpLessU64 {
                        target,
                        lhs: rhs_off,
                        rhs: lhs_off,
                    })?,
                    // x <= y ↔ y >= x
                    (true, CmpOp::Le) => self.emit(MicroOp::JumpGreaterEqualU64 {
                        target,
                        lhs: rhs_off,
                        rhs: lhs_off,
                    })?,
                    (true, CmpOp::Neq) => self.emit(MicroOp::JumpNotEqualU64 {
                        target,
                        lhs: lhs_off,
                        rhs: rhs_off,
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
                            }))?;
                        },
                        EqKind::Ref => {
                            self.emit(MicroOp::JumpValueRefCmp(JumpValueRefCmpOp {
                                target,
                                negate: eq_negate(*op)?,
                                lhs: lhs_off,
                                rhs: rhs_off,
                                ty: strip_ref(lhs_interned)?,
                            }))?;
                        },
                    },
                }
            },
            Instr::BrCmpImm(Label(l), op, src, imm) => {
                let src_ty = self.slot_type(*src)?;
                let src_off = self.slot(*src)?.offset;
                let target = CodeOffset(encode_label(*l));
                let idx = self.out_buf.len();
                self.branch_fixups.push(idx);
                if src_ty.is_u64() {
                    // Fast path: specialized unsigned `u64` ordering jumps.
                    // Note: equality has no specialized imm jump, so it uses
                    // the general `JumpIntCmp`.
                    let v = imm_to_u64(imm)?;
                    match op {
                        CmpOp::Ge => self.emit(MicroOp::JumpGreaterEqualU64Imm {
                            target,
                            src: src_off,
                            imm: v,
                        })?,
                        CmpOp::Lt => self.emit(MicroOp::JumpLessU64Imm {
                            target,
                            src: src_off,
                            imm: v,
                        })?,
                        CmpOp::Gt => self.emit(MicroOp::JumpGreaterU64Imm {
                            target,
                            src: src_off,
                            imm: v,
                        })?,
                        CmpOp::Le => self.emit(MicroOp::JumpLessEqualU64Imm {
                            target,
                            src: src_off,
                            imm: v,
                        })?,
                        CmpOp::Eq | CmpOp::Neq => {
                            self.emit_jump_int_cmp(target, *op, src_off, IntOperand::ImmU64(v))?
                        },
                    }
                } else {
                    let rhs_op = cmp_operand_from_imm(imm)?;
                    self.emit_jump_int_cmp(target, *op, src_off, rhs_op)?;
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

            // --- Inline-struct: by-value ---
            //
            // The struct lives in a frame slot at compile-time-known offset;
            // the field's absolute frame offset is therefore also compile-time.
            // No fat pointer is materialized.
            Instr::ImmBorrowLocField(dst, fh, local) | Instr::MutBorrowLocField(dst, fh, local) => {
                let struct_ty = self.slot_interned_type(*local)?;
                let (field_offset, _) = self.resolve_field(struct_ty, *fh)?;
                let local_info = self.slot(*local)?;
                let dst_info = self.def_slot(*dst)?;
                self.emit(MicroOp::SlotBorrow {
                    dst: dst_info.offset,
                    local: FrameOffset(local_info.offset.0 + field_offset),
                })?;
            },
            Instr::ReadLocalField(dst, fh, local) => {
                let struct_ty = self.slot_interned_type(*local)?;
                let (field_offset, field_size) = self.resolve_field(struct_ty, *fh)?;
                let local_info = self.slot(*local)?;
                let dst_info = self.def_slot(*dst)?;
                let src = SizedSlot {
                    offset: FrameOffset(local_info.offset.0 + field_offset),
                    size: field_size,
                    align: local_info.align,
                };
                self.emit_single_move(dst_info.offset, src)?;
            },
            Instr::WriteLocalField(fh, local, val) => {
                let struct_ty = self.slot_interned_type(*local)?;
                let (field_offset, field_size) = self.resolve_field(struct_ty, *fh)?;
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
            Instr::ImmBorrowField(dst, fh, src) | Instr::MutBorrowField(dst, fh, src) => {
                let struct_ty = strip_ref(self.slot_interned_type(*src)?)?;
                let (field_offset, _) = self.resolve_field(struct_ty, *fh)?;
                let src_info = self.slot(*src)?;
                let dst_info = self.def_slot(*dst)?;
                self.emit(MicroOp::DeriveRefOffsetImm {
                    dst_ref: dst_info.offset,
                    src_ref: src_info.offset,
                    offset: field_offset,
                })?;
            },
            Instr::ReadField(dst, fh, src) => {
                let struct_ty = strip_ref(self.slot_interned_type(*src)?)?;
                let (field_offset, field_size) = self.resolve_field(struct_ty, *fh)?;
                let src_info = self.slot(*src)?;
                let dst_info = self.def_slot(*dst)?;
                self.emit(MicroOp::ReadRefOffset {
                    dst: dst_info.offset,
                    ref_ptr: src_info.offset,
                    offset: field_offset,
                    size: field_size,
                })?;
            },
            Instr::WriteField(fh, dst_ref, val) => {
                let struct_ty = strip_ref(self.slot_interned_type(*dst_ref)?)?;
                let (field_offset, field_size) = self.resolve_field(struct_ty, *fh)?;
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
                // `struct_ty` must be a concrete nominal with a populated layout.
                let layout = view_type(*struct_ty)
                    .layout()
                    .ok_or_else(|| anyhow::anyhow!("Pack: struct_ty has no populated layout"))?;
                let fields = layout.field_layouts().ok_or_else(|| {
                    anyhow::anyhow!("Pack: nominal type is not a struct (no field layouts)")
                })?;
                if fields.len() != args.len() {
                    bail!(
                        "Pack: arg count {} does not match struct field count {}",
                        args.len(),
                        fields.len()
                    );
                }
                let dst_info = self.def_slot(*dst)?;
                let arg_infos = args
                    .iter()
                    .map(|s| self.slot(*s))
                    .collect::<Result<Vec<_>>>()?;
                // Pre-compute (size, align) per field once.
                let field_widths: Vec<(u32, u32)> = fields
                    .iter()
                    .map(|field| {
                        view_type(field.ty())
                            .size_and_align()
                            .ok_or_else(|| anyhow::anyhow!("Pack: field type has no concrete size"))
                    })
                    .collect::<Result<_>>()?;
                let copies: Vec<_> = fields
                    .iter()
                    .zip(arg_infos.iter())
                    .zip(field_widths.iter())
                    .map(|((field, arg_info), &(size, _))| parallel_copy::Copy {
                        src: arg_info.offset,
                        dst: FrameOffset(dst_info.offset.0 + field.offset),
                        width: size,
                    })
                    .collect();
                let mut indices: Vec<usize> = (0..fields.len()).collect();
                // TODO: check if we can have cheaper checks for reverse/forward emit safety
                // in the presence of alignments.
                if parallel_copy::reverse_emit_is_safe(&copies) {
                    indices.reverse();
                } else if !parallel_copy::forward_emit_is_safe(&copies) {
                    bail!("Pack: neither reverse nor forward emit is overlap-safe");
                }
                for i in indices {
                    let (size, align) = field_widths[i];
                    self.emit_single_move(
                        FrameOffset(dst_info.offset.0 + fields[i].offset),
                        SizedSlot {
                            offset: arg_infos[i].offset,
                            size,
                            align,
                        },
                    )?;
                }
            },
            Instr::Unpack(dsts, struct_ty, src) => {
                // See the `Instr::Pack` arm above for the `struct_ty` contract.
                let layout = view_type(*struct_ty)
                    .layout()
                    .ok_or_else(|| anyhow::anyhow!("Unpack: struct_ty has no populated layout"))?;
                let fields = layout.field_layouts().ok_or_else(|| {
                    anyhow::anyhow!("Unpack: nominal type is not a struct (no field layouts)")
                })?;
                if fields.len() != dsts.len() {
                    bail!(
                        "Unpack: dst count {} does not match struct field count {}",
                        dsts.len(),
                        fields.len()
                    );
                }
                let src_info = self.slot(*src)?;
                // Pre-compute (size, align) per field and resolve each dst's
                // SizedSlot. We do this in a separate pass so we can build the
                // per-copy view the debug assert needs without interleaving
                // it with the actual emit.
                let mut field_widths = Vec::with_capacity(fields.len());
                let mut dst_offsets = Vec::with_capacity(dsts.len());
                for (field, dst) in fields.iter().zip(dsts.iter()) {
                    let (size, align) =
                        view_type(field.ty()).size_and_align().ok_or_else(|| {
                            anyhow::anyhow!("Unpack: field type has no concrete size")
                        })?;
                    field_widths.push((size, align));
                    let dst_info = self.def_slot(*dst)?;
                    dst_offsets.push(dst_info.offset);
                }
                let copies: Vec<_> = fields
                    .iter()
                    .zip(dst_offsets.iter())
                    .zip(field_widths.iter())
                    .map(|((field, dst_off), &(size, _))| parallel_copy::Copy {
                        src: FrameOffset(src_info.offset.0 + field.offset),
                        dst: *dst_off,
                        width: size,
                    })
                    .collect();
                let mut indices: Vec<usize> = (0..fields.len()).collect();
                if parallel_copy::reverse_emit_is_safe(&copies) {
                    indices.reverse();
                } else if !parallel_copy::forward_emit_is_safe(&copies) {
                    bail!("Unpack: neither reverse nor forward emit is overlap-safe");
                }
                for i in indices {
                    let (size, align) = field_widths[i];
                    self.emit_single_move(dst_offsets[i], SizedSlot {
                        offset: FrameOffset(src_info.offset.0 + fields[i].offset),
                        size,
                        align,
                    })?;
                }
            },

            // --- Closures ---
            Instr::PackClosure(dst, _fhi, mask, captured) => {
                // Target identity + captured-data descriptor were resolved in
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
            Instr::PackClosureGeneric(..) => bail!("generic closures not yet lowered"),
            Instr::CallClosure(rets, _sig_types, all_args) => {
                let ret_slots = &self.ctx.closure_call_sites[self.closure_call_cursor];
                self.closure_call_cursor += 1;
                // The destacker pushes the closure as the last operand;
                // everything before it is a provided (non-captured) argument.
                let Some((closure_slot, provided)) = all_args.split_last() else {
                    bail!("CallClosure has no closure operand");
                };
                let closure_src = self.slot(*closure_slot)?.offset;
                let provided_args = self.slots_to_sized_slots(provided)?;
                self.emit(MicroOp::CallClosure(Box::new(CallClosureOp {
                    closure_src,
                    provided_args,
                })))?;
                self.bind_call_returns(rets, ret_slots)?;
            },

            // --- Generic and variant field forms: not yet lowered ---
            Instr::ImmBorrowFieldGeneric(..)
            | Instr::MutBorrowFieldGeneric(..)
            | Instr::ReadFieldGeneric(..)
            | Instr::WriteFieldGeneric(..) => {
                bail!("generic field instruction not yet lowered")
            },

            _ => bail!("instruction {} not yet lowered", instr.opcode_name()),
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
    fn derive_native_abi(&self, cs: &CallSiteInfo) -> Result<NativeABI> {
        let callee_base = self.ctx.frame_data_size + FRAME_METADATA_SIZE as u32;
        let to_slot = |s: &TypedSlot| FrameSlot {
            offset: s.slot.offset.0 - callee_base,
            size: s.slot.size,
        };
        let args: Vec<FrameSlot> = cs.arg_slots.iter().map(to_slot).collect();
        let returns: Vec<FrameSlot> = cs.ret_slots.iter().map(to_slot).collect();
        Ok(NativeABI::new(args, returns)?)
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

    fn fixup_branches(&mut self) -> Result<()> {
        for &idx in &self.branch_fixups {
            // Extract the encoded label from the op, resolve it, then patch.
            let encoded = match &self.out_buf[idx] {
                MicroOp::Jump { target }
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
                other => bail!("unexpected non-branch op at fixup index {}: {}", idx, other),
            };
            let label = decode_label(encoded);
            let resolved = self.resolve_label(label)?;
            match &mut self.out_buf[idx] {
                MicroOp::Jump { target }
                | MicroOp::JumpNotZeroU64 { target, .. }
                | MicroOp::JumpNotZeroByte { target, .. }
                | MicroOp::JumpZeroByte { target, .. }
                | MicroOp::JumpGreaterEqualU64Imm { target, .. }
                | MicroOp::JumpLessU64Imm { target, .. }
                | MicroOp::JumpGreaterU64Imm { target, .. }
                | MicroOp::JumpLessEqualU64Imm { target, .. }
                | MicroOp::JumpLessU64 { target, .. }
                | MicroOp::JumpGreaterEqualU64 { target, .. }
                | MicroOp::JumpNotEqualU64 { target, .. } => target.0 = resolved,
                MicroOp::JumpIntCmp(op) => op.target.0 = resolved,
                MicroOp::JumpValueCmp(op) => op.target.0 = resolved,
                MicroOp::JumpValueRefCmp(op) => op.target.0 = resolved,
                other => bail!("unexpected non-branch op at fixup index {}: {}", idx, other),
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

/// Map an IR [`CmpOp`] to the micro-op [`CmpKind`].
///
/// TODO: `CmpOp` and `CmpKind` have identical variants; consider unifying on a
/// single shared type in core to drop this mapping.
fn cmp_kind(op: CmpOp) -> CmpKind {
    match op {
        CmpOp::Lt => CmpKind::Lt,
        CmpOp::Le => CmpKind::Le,
        CmpOp::Gt => CmpKind::Gt,
        CmpOp::Ge => CmpKind::Ge,
        CmpOp::Eq => CmpKind::Eq,
        CmpOp::Neq => CmpKind::Neq,
    }
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
fn eq_kind(ty: &Type) -> Result<EqKind> {
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
            bail!("equality is not supported for this operand type")
        },
    })
}

/// Map an equality [`CmpOp`] to the `negate` flag of the structural-equality
/// ops (`false` for `Eq`, `true` for `Neq`).
fn eq_negate(op: CmpOp) -> Result<bool> {
    match op {
        CmpOp::Eq => Ok(false),
        CmpOp::Neq => Ok(true),
        CmpOp::Lt | CmpOp::Le | CmpOp::Gt | CmpOp::Ge => {
            bail!("ordering comparison on a non-scalar operand is ill-typed")
        },
    }
}

/// Build an [`IntOperand`] for a comparison operand. Integer types delegate to
/// [`int_operand_from_slot`]. `bool` (1 byte), `address` and `signer` (both 32
/// bytes) are flat values with only `==`/`!=` (no ordering), and comparing
/// their bit patterns is exactly value equality, so they reuse the integer
/// compare ops at the matching width.
fn cmp_operand_from_slot(ty: &Type, off: FrameOffset) -> Result<IntOperand> {
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
        | Type::TypeParam { .. } => bail!("operand type has no comparison lowering"),
    }
}

/// Immediate counterpart of [`cmp_operand_from_slot`]: a bool immediate
/// compares as the 1-byte value `0`/`1`.
fn cmp_operand_from_imm(imm: &ImmValue) -> Result<IntOperand> {
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
