// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Lowers stackless exec IR to micro-ops.

use super::{
    context::{LoweringContext, SlotInfo, TypedSlot},
    parallel_copy,
};
use crate::stackless_exec_ir::{BinaryOp, CmpOp, FunctionIR, ImmValue, Instr, Label, Slot};
use anyhow::{bail, Context, Result};
use mono_move_core::{
    types::{strip_ref, view_type, InternedType, Type},
    CodeOffset, FrameOffset, MicroOp,
};

pub fn lower_function(func_ir: &FunctionIR, ctx: &LoweringContext) -> Result<Vec<MicroOp>> {
    let mut state = LoweringState::new(func_ir, ctx);
    for block in &func_ir.blocks {
        // Reset Xfer bindings: Vids are block-local in the post-allocation
        // IR, so a binding from a previous block can never be a legitimate
        // read in this one. Wiping makes any accidental cross-block read
        // surface as an error from `slot()` (via `xfer_binding`) instead
        // of silently using a stale binding.
        state.xfer_bindings.fill(None);
        state.label_map[block.label.0 as usize] = Some(state.out_buf.len() as u32);
        for instr in &block.instrs {
            state.lower_instr(func_ir, instr)?;
        }
    }
    state.fixup_branches()?;
    Ok(state.out_buf)
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
    /// Where `Slot::Xfer(j)` currently lives in the caller's frame
    /// (an arg slot before a call; a ret slot after). `None` means no
    /// value lives at `j` right now. Length is fixed at
    /// `ctx.num_xfer_positions`. Each `j` is rebound many times within
    /// a block as successive calls reuse it; every binding is wiped to
    /// `None` at block boundaries by `lower_function`, so stale
    /// cross-block reads error out via `xfer_binding`.
    xfer_bindings: Vec<Option<TypedSlot>>,
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

    /// Resolve slot info for a destination slot. For Xfer slots, binds
    /// the position to the upcoming call's `arg_slots[j]`.
    fn def_slot(&mut self, slot: Slot) -> Result<SlotInfo> {
        Ok(match slot {
            Slot::Home(i) => self.ctx.home_slots[i as usize],
            Slot::Xfer(j) => {
                let cs = &self.ctx.call_sites[self.call_site_cursor];
                let ts = cs.arg_slots[j as usize];
                self.xfer_bindings[j as usize] = Some(ts);
                ts.slot
            },
            Slot::Vid(i) => bail!("Vid({}) in post-allocation IR", i),
        })
    }

    fn emit(&mut self, op: MicroOp) {
        self.out_buf.push(op);
    }

    fn slot_type(&self, slot: Slot) -> Result<&Type> {
        let ptr = match slot {
            Slot::Home(i) => self.home_slot_types[i as usize],
            Slot::Xfer(j) => self.xfer_binding(j)?.ty,
            Slot::Vid(i) => bail!("Vid({}) in post-allocation IR", i),
        };
        Ok(view_type(ptr))
    }

    /// Size in bytes of `ref_slot`'s pointee.
    fn ref_pointee_size(&self, ref_slot: Slot) -> Result<u32> {
        let ref_ty = match ref_slot {
            Slot::Home(i) => self.home_slot_types[i as usize],
            Slot::Xfer(j) => self.xfer_binding(j)?.ty,
            Slot::Vid(i) => bail!("Vid({}) in post-allocation IR", i),
        };
        let pointee = strip_ref(ref_ty)?;
        let (size, _) = view_type(pointee)
            .size_and_align()
            .ok_or_else(|| anyhow::anyhow!("ref pointee type has no concrete size"))?;
        Ok(size)
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
    fn emit_single_move(&mut self, dst_offset: FrameOffset, src: SlotInfo) {
        if dst_offset == src.offset {
            return;
        }
        if src.size == 8 {
            self.emit(MicroOp::Move8 {
                dst: dst_offset,
                src: src.offset,
            });
        } else {
            self.emit(MicroOp::Move {
                dst: dst_offset,
                src: src.offset,
                size: src.size,
            });
        }
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
                });
            },
            Instr::LdTrue(dst) => {
                let dst_info = self.def_slot(*dst)?;
                self.emit(MicroOp::StoreImm8 {
                    dst: dst_info.offset,
                    imm: 1,
                });
            },
            Instr::LdFalse(dst) => {
                let dst_info = self.def_slot(*dst)?;
                self.emit(MicroOp::StoreImm8 {
                    dst: dst_info.offset,
                    imm: 0,
                });
            },
            Instr::LdU8(dst, v) => {
                let dst_info = self.def_slot(*dst)?;
                self.emit(MicroOp::StoreImm8 {
                    dst: dst_info.offset,
                    imm: *v as u64,
                });
            },
            Instr::LdU16(dst, v) => {
                let dst_info = self.def_slot(*dst)?;
                self.emit(MicroOp::StoreImm8 {
                    dst: dst_info.offset,
                    imm: *v as u64,
                });
            },
            Instr::LdU32(dst, v) => {
                let dst_info = self.def_slot(*dst)?;
                self.emit(MicroOp::StoreImm8 {
                    dst: dst_info.offset,
                    imm: *v as u64,
                });
            },
            Instr::LdI8(dst, v) => {
                let dst_info = self.def_slot(*dst)?;
                self.emit(MicroOp::StoreImm8 {
                    dst: dst_info.offset,
                    imm: *v as u64,
                });
            },
            Instr::LdI16(dst, v) => {
                let dst_info = self.def_slot(*dst)?;
                self.emit(MicroOp::StoreImm8 {
                    dst: dst_info.offset,
                    imm: *v as u64,
                });
            },
            Instr::LdI32(dst, v) => {
                let dst_info = self.def_slot(*dst)?;
                self.emit(MicroOp::StoreImm8 {
                    dst: dst_info.offset,
                    imm: *v as u64,
                });
            },
            Instr::LdI64(dst, v) => {
                let dst_info = self.def_slot(*dst)?;
                self.emit(MicroOp::StoreImm8 {
                    dst: dst_info.offset,
                    imm: *v as u64,
                });
            },

            // --- Copy/Move ---
            Instr::Copy(dst, src) | Instr::Move(dst, src) => {
                let src_info = self.slot(*src)?;
                let dst_info = self.def_slot(*dst)?;
                self.emit_single_move(dst_info.offset, src_info);
            },

            // --- Binary ops ---
            Instr::BinaryOp(dst, op, lhs, rhs) => {
                let lhs_ty = self.slot_type(*lhs)?;
                if Self::is_u64_sized(lhs_ty) {
                    let lhs_info = self.slot(*lhs)?;
                    let rhs_info = self.slot(*rhs)?;
                    let dst_info = self.def_slot(*dst)?;
                    let dst = dst_info.offset;
                    let lhs = lhs_info.offset;
                    let rhs = rhs_info.offset;
                    match op {
                        BinaryOp::Add => self.emit(MicroOp::AddU64 { dst, lhs, rhs }),
                        BinaryOp::Sub => self.emit(MicroOp::SubU64 { dst, lhs, rhs }),
                        BinaryOp::Mul => self.emit(MicroOp::MulU64 { dst, lhs, rhs }),
                        BinaryOp::Div => self.emit(MicroOp::DivU64 { dst, lhs, rhs }),
                        BinaryOp::Mod => self.emit(MicroOp::ModU64 { dst, lhs, rhs }),
                        BinaryOp::BitAnd => self.emit(MicroOp::BitAndU64 { dst, lhs, rhs }),
                        BinaryOp::BitOr => self.emit(MicroOp::BitOrU64 { dst, lhs, rhs }),
                        BinaryOp::BitXor => self.emit(MicroOp::BitXorU64 { dst, lhs, rhs }),
                        BinaryOp::Shl => self.emit(MicroOp::ShlU64 { dst, lhs, rhs }),
                        BinaryOp::Shr => self.emit(MicroOp::ShrU64 { dst, lhs, rhs }),
                        BinaryOp::Cmp(_) | BinaryOp::Or | BinaryOp::And => {
                            bail!("BinaryOp {:?} for u64-sized type not yet lowered", op)
                        },
                    }
                } else {
                    bail!("BinaryOp for non-u64 type not yet lowered");
                }
            },

            // --- Binary ops with immediate ---
            Instr::BinaryOpImm(dst, op, src, imm) => {
                let src_ty = self.slot_type(*src)?;
                if Self::is_u64_sized(src_ty) {
                    let src_info = self.slot(*src)?;
                    let dst_info = self.def_slot(*dst)?;
                    let v = imm_to_u64(imm);
                    let dst = dst_info.offset;
                    let src = src_info.offset;
                    match op {
                        BinaryOp::Add => self.emit(MicroOp::AddU64Imm { dst, src, imm: v }),
                        BinaryOp::Sub => self.emit(MicroOp::SubU64Imm { dst, src, imm: v }),
                        BinaryOp::Mul => self.emit(MicroOp::MulU64Imm { dst, src, imm: v }),
                        BinaryOp::Div => self.emit(MicroOp::DivU64Imm { dst, src, imm: v }),
                        BinaryOp::Mod => self.emit(MicroOp::ModU64Imm { dst, src, imm: v }),
                        BinaryOp::Shl => self.emit(MicroOp::ShlU64Imm {
                            dst,
                            src,
                            imm: shift_imm_u8(imm)?,
                        }),
                        BinaryOp::Shr => self.emit(MicroOp::ShrU64Imm {
                            dst,
                            src,
                            imm: shift_imm_u8(imm)?,
                        }),
                        // No immediate forms today: BitAnd/BitOr/BitXor and the
                        // Cmp/Or/And ops.
                        BinaryOp::BitAnd
                        | BinaryOp::BitOr
                        | BinaryOp::BitXor
                        | BinaryOp::Cmp(_)
                        | BinaryOp::Or
                        | BinaryOp::And => {
                            bail!("BinaryOpImm {:?} for u64-sized type not yet lowered", op)
                        },
                    }
                } else {
                    bail!("BinaryOpImm for non-u64 type not yet lowered");
                }
            },

            // --- References ---
            Instr::ImmBorrowLoc(dst, src) | Instr::MutBorrowLoc(dst, src) => {
                let src_info = self.slot(*src)?;
                let dst_info = self.def_slot(*dst)?;
                self.emit(MicroOp::SlotBorrow {
                    dst: dst_info.offset,
                    local: src_info.offset,
                });
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
                });
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
                });
            },

            // --- Control flow ---
            Instr::Branch(Label(l)) => {
                let idx = self.out_buf.len();
                self.branch_fixups.push(idx);
                self.emit(MicroOp::Jump {
                    target: CodeOffset(encode_label(*l)),
                });
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
                });
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
                });
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
                        }),
                        CmpOp::Ge => self.emit(MicroOp::JumpGreaterEqualU64 {
                            target: CodeOffset(encode_label(*l)),
                            lhs: lhs_info.offset,
                            rhs: rhs_info.offset,
                        }),
                        // x > y ↔ y < x
                        CmpOp::Gt => self.emit(MicroOp::JumpLessU64 {
                            target: CodeOffset(encode_label(*l)),
                            lhs: rhs_info.offset,
                            rhs: lhs_info.offset,
                        }),
                        // x <= y ↔ y >= x
                        CmpOp::Le => self.emit(MicroOp::JumpGreaterEqualU64 {
                            target: CodeOffset(encode_label(*l)),
                            lhs: rhs_info.offset,
                            rhs: lhs_info.offset,
                        }),
                        CmpOp::Neq => self.emit(MicroOp::JumpNotEqualU64 {
                            target: CodeOffset(encode_label(*l)),
                            lhs: lhs_info.offset,
                            rhs: rhs_info.offset,
                        }),
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
                    let v = imm_to_u64(imm);
                    let idx = self.out_buf.len();
                    self.branch_fixups.push(idx);
                    match op {
                        CmpOp::Ge => self.emit(MicroOp::JumpGreaterEqualU64Imm {
                            target: CodeOffset(encode_label(*l)),
                            src: src_info.offset,
                            imm: v,
                        }),
                        CmpOp::Lt => self.emit(MicroOp::JumpLessU64Imm {
                            target: CodeOffset(encode_label(*l)),
                            src: src_info.offset,
                            imm: v,
                        }),
                        CmpOp::Gt => self.emit(MicroOp::JumpGreaterU64Imm {
                            target: CodeOffset(encode_label(*l)),
                            src: src_info.offset,
                            imm: v,
                        }),
                        CmpOp::Le => self.emit(MicroOp::JumpLessEqualU64Imm {
                            target: CodeOffset(encode_label(*l)),
                            src: src_info.offset,
                            imm: v,
                        }),
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
                parallel_copy::emit_parallel_copy(
                    &mut self.out_buf,
                    &copies,
                    self.ctx.scratch_offset,
                );
                self.emit(MicroOp::Return);
            },

            _ => bail!("instruction {} not yet lowered", instr.opcode_name()),
        }
        Ok(())
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
                });
            } else {
                self.emit(MicroOp::Move {
                    dst: dst_off,
                    src: arg_info.offset,
                    size: arg_info.size,
                });
            }
        }

        self.emit(MicroOp::CallIndirect {
            executable_id: cs.callee_module_id,
            func_name: cs.callee_func_name,
        });
        self.call_site_cursor += 1;

        // Place each ret. Xfer rets just bind the slot info — the next
        // consumer reads from `ret_slots[k]` directly. Home rets copy
        // out of the ret region into their Home slot (disjoint regions,
        // never conflict with future arg setup).
        for (k, ret_slot) in rets.iter().enumerate() {
            match *ret_slot {
                Slot::Xfer(j) => {
                    self.xfer_bindings[j as usize] = Some(cs.ret_slots[k]);
                },
                Slot::Home(i) => {
                    let src = cs.ret_slots[k].slot;
                    let dst = self.ctx.home_slots[i as usize];
                    self.emit_single_move(dst.offset, src);
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

fn imm_to_u64(imm: &ImmValue) -> u64 {
    match imm {
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
    }
}

/// Extract a u8 shift amount. The rhs of a Move `Shl`/`Shr` is always u8
/// by language spec; anything else is an upstream invariant violation.
fn shift_imm_u8(imm: &ImmValue) -> Result<u8> {
    match imm {
        ImmValue::U8(v) => Ok(*v),
        other => bail!("shift immediate must be u8, got {:?}", other),
    }
}
