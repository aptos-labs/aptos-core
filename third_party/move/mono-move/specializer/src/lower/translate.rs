// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Lowers stackless exec IR to micro-ops.

use super::context::{LoweringContext, SlotInfo};
use crate::stackless_exec_ir::{BinaryOp, CmpOp, FunctionIR, ImmValue, Instr, Label, Slot};
use anyhow::{bail, Result};
use mono_move_core::{
    types::{view_type, InternedType, Type},
    CodeOffset, FrameOffset, MicroOp,
};

pub fn lower_function(func_ir: &FunctionIR, ctx: &LoweringContext) -> Result<Vec<MicroOp>> {
    let mut state = LoweringState::new(func_ir, ctx);
    for block in &func_ir.blocks {
        state.label_map[block.label.0 as usize] = Some(state.ops.len() as u32);
        for instr in &block.instrs {
            state.lower_instr(func_ir, instr)?;
        }
    }
    state.fixup_branches()?;
    Ok(state.ops)
}

struct LoweringState<'a> {
    ctx: &'a LoweringContext,
    ops: Vec<MicroOp>,
    /// Label(i) -> micro-op index. Dense: one entry per block; `None` until filled during lowering.
    label_map: Vec<Option<u32>>,
    /// Indices into ops that need target patching
    branch_fixups: Vec<usize>,
    call_site_cursor: usize,
    home_slot_types: &'a [InternedType],
    /// Per-position slot info for Xfer slots, updated lazily.
    active_xfer_slots: Vec<SlotInfo>,
    /// Per-position types for Xfer slots, updated lazily.
    active_xfer_types: Vec<InternedType>,
}

impl<'a> LoweringState<'a> {
    fn new(func_ir: &'a FunctionIR, ctx: &'a LoweringContext) -> Self {
        // `active_xfer_slots` / `active_xfer_types` track the *current* meaning
        // of each Xfer slot (which changes across call sites). Each position is
        // overwritten by `def_slot` before it's read in valid IR, so the initial
        // values below are benign placeholders that should never be observed.
        // TODO: revisit this design.
        let num_xfer_slots = ctx.num_xfer_slots as usize;
        let mut active_xfer_slots = vec![
            SlotInfo {
                offset: 0,
                size: 0,
                align: 1
            };
            num_xfer_slots
        ];
        let mut active_xfer_types = vec![mono_move_core::types::U64_TY; num_xfer_slots];
        if !ctx.call_sites.is_empty() {
            let first = &ctx.call_sites[0];
            let n = num_xfer_slots.min(first.arg_write_slots.len());
            active_xfer_slots[..n].copy_from_slice(&first.arg_write_slots[..n]);
            active_xfer_types[..n].clone_from_slice(&first.param_types[..n]);
        }

        LoweringState {
            ctx,
            ops: Vec::new(),
            label_map: vec![None; func_ir.blocks.len()],
            branch_fixups: Vec::new(),
            call_site_cursor: 0,
            home_slot_types: &func_ir.home_slot_types,
            active_xfer_slots,
            active_xfer_types,
        }
    }

    fn slot(&self, slot: Slot) -> SlotInfo {
        match slot {
            Slot::Home(i) => self.ctx.home_slots[i as usize],
            Slot::Xfer(j) => self.active_xfer_slots[j as usize],
            Slot::Vid(_) => unreachable!("Vid slot in post-allocation IR"),
        }
    }

    /// Resolve slot info for a destination slot. For Xfer slots, updates
    /// `active_xfer_slots` and `active_xfer_types` from the upcoming callsite
    /// before returning.
    fn def_slot(&mut self, slot: Slot) -> SlotInfo {
        match slot {
            Slot::Home(i) => self.ctx.home_slots[i as usize],
            Slot::Xfer(j) => {
                let cs = &self.ctx.call_sites[self.call_site_cursor];
                let slot = cs.arg_write_slots[j as usize];
                let ty = cs.param_types[j as usize];
                self.active_xfer_slots[j as usize] = slot;
                self.active_xfer_types[j as usize] = ty;
                slot
            },
            Slot::Vid(_) => unreachable!("Vid slot in post-allocation IR"),
        }
    }

    fn emit(&mut self, op: MicroOp) {
        self.ops.push(op);
    }

    fn emit_move(&mut self, dst: SlotInfo, src: SlotInfo) {
        if dst.offset == src.offset {
            return;
        }
        if src.size == 8 {
            self.emit(MicroOp::Move8 {
                dst: FrameOffset(dst.offset),
                src: FrameOffset(src.offset),
            });
        } else {
            self.emit(MicroOp::Move {
                dst: FrameOffset(dst.offset),
                src: FrameOffset(src.offset),
                size: src.size,
            });
        }
    }

    fn slot_type(&self, slot: Slot) -> &Type {
        let ptr = match slot {
            Slot::Home(i) => self.home_slot_types[i as usize],
            Slot::Xfer(j) => self.active_xfer_types[j as usize],
            Slot::Vid(_) => unreachable!("Vid slot in post-allocation IR"),
        };
        view_type(ptr)
    }

    /// Returns true if the type fits in 8 bytes or fewer (can use u64 micro-ops).
    /// TODO: this is only a temporary heuristic until we have proper type-specific
    /// micro-ops.
    fn fits_in_u64(ty: &Type) -> bool {
        match ty.size_and_align() {
            Some((size, _)) => size <= 8,
            None => false,
        }
    }

    /// Lower one IR instruction.
    fn lower_instr(&mut self, func_ir: &FunctionIR, instr: &Instr) -> Result<()> {
        match instr {
            // --- Loads ---
            Instr::LdU64(dst, v) => {
                let d = self.def_slot(*dst);
                self.emit(MicroOp::StoreImm8 {
                    dst: FrameOffset(d.offset),
                    imm: *v,
                });
            },
            Instr::LdTrue(dst) => {
                let d = self.def_slot(*dst);
                self.emit(MicroOp::StoreImm8 {
                    dst: FrameOffset(d.offset),
                    imm: 1,
                });
            },
            Instr::LdFalse(dst) => {
                let d = self.def_slot(*dst);
                self.emit(MicroOp::StoreImm8 {
                    dst: FrameOffset(d.offset),
                    imm: 0,
                });
            },
            Instr::LdU8(dst, v) => {
                let d = self.def_slot(*dst);
                self.emit(MicroOp::StoreImm8 {
                    dst: FrameOffset(d.offset),
                    imm: *v as u64,
                });
            },
            Instr::LdU16(dst, v) => {
                let d = self.def_slot(*dst);
                self.emit(MicroOp::StoreImm8 {
                    dst: FrameOffset(d.offset),
                    imm: *v as u64,
                });
            },
            Instr::LdU32(dst, v) => {
                let d = self.def_slot(*dst);
                self.emit(MicroOp::StoreImm8 {
                    dst: FrameOffset(d.offset),
                    imm: *v as u64,
                });
            },
            Instr::LdI8(dst, v) => {
                let d = self.def_slot(*dst);
                self.emit(MicroOp::StoreImm8 {
                    dst: FrameOffset(d.offset),
                    imm: *v as u64,
                });
            },
            Instr::LdI16(dst, v) => {
                let d = self.def_slot(*dst);
                self.emit(MicroOp::StoreImm8 {
                    dst: FrameOffset(d.offset),
                    imm: *v as u64,
                });
            },
            Instr::LdI32(dst, v) => {
                let d = self.def_slot(*dst);
                self.emit(MicroOp::StoreImm8 {
                    dst: FrameOffset(d.offset),
                    imm: *v as u64,
                });
            },
            Instr::LdI64(dst, v) => {
                let d = self.def_slot(*dst);
                self.emit(MicroOp::StoreImm8 {
                    dst: FrameOffset(d.offset),
                    imm: *v as u64,
                });
            },

            // --- Copy/Move ---
            Instr::Copy(dst, src) | Instr::Move(dst, src) => {
                let s = self.slot(*src);
                let d = self.def_slot(*dst);
                self.emit_move(d, s);
            },

            // --- Binary ops ---
            Instr::BinaryOp(dst, op, lhs, rhs) => {
                let lhs_ty = self.slot_type(*lhs);
                if Self::fits_in_u64(lhs_ty) {
                    let l = self.slot(*lhs);
                    let r = self.slot(*rhs);
                    let d = self.def_slot(*dst);
                    match op {
                        BinaryOp::Add => self.emit(MicroOp::AddU64 {
                            dst: FrameOffset(d.offset),
                            lhs: FrameOffset(l.offset),
                            rhs: FrameOffset(r.offset),
                        }),
                        _ => bail!("BinaryOp {:?} for u64-sized type not yet lowered", op),
                    }
                } else {
                    bail!("BinaryOp for non-u64 type not yet lowered");
                }
            },

            // --- Binary ops with immediate ---
            Instr::BinaryOpImm(dst, op, src, imm) => {
                let src_ty = self.slot_type(*src);
                if Self::fits_in_u64(src_ty) {
                    let s = self.slot(*src);
                    let d = self.def_slot(*dst);
                    let v = imm_to_u64(imm);
                    match op {
                        BinaryOp::Sub => self.emit(MicroOp::SubU64Imm {
                            dst: FrameOffset(d.offset),
                            src: FrameOffset(s.offset),
                            imm: v,
                        }),
                        BinaryOp::Add => self.emit(MicroOp::AddU64Imm {
                            dst: FrameOffset(d.offset),
                            src: FrameOffset(s.offset),
                            imm: v,
                        }),
                        _ => bail!("BinaryOpImm {:?} for u64-sized type not yet lowered", op),
                    }
                } else {
                    bail!("BinaryOpImm for non-u64 type not yet lowered");
                }
            },

            // --- Control flow ---
            Instr::Branch(Label(l)) => {
                let idx = self.ops.len();
                self.branch_fixups.push(idx);
                self.emit(MicroOp::Jump {
                    target: CodeOffset(encode_label(*l)),
                });
            },
            Instr::BrTrue(Label(l), cond) => {
                let s = self.slot(*cond);
                let idx = self.ops.len();
                self.branch_fixups.push(idx);
                // [TODO]: we are representing booleans as 0/1 in u64 slots here.
                // This needs to be updated with a more compact boolean representation.
                self.emit(MicroOp::JumpNotZeroU64 {
                    target: CodeOffset(encode_label(*l)),
                    src: FrameOffset(s.offset),
                });
            },
            Instr::BrFalse(Label(l), cond) => {
                let s = self.slot(*cond);
                let idx = self.ops.len();
                self.branch_fixups.push(idx);
                // [TODO]: we are representing booleans as 0/1 in u64 slots here.
                // This needs to be updated with a more compact boolean representation.
                self.emit(MicroOp::JumpLessU64Imm {
                    target: CodeOffset(encode_label(*l)),
                    src: FrameOffset(s.offset),
                    imm: 1,
                });
            },

            // --- Fused compare+branch ---
            Instr::BrCmp(Label(l), op, lhs, rhs) => {
                let lhs_ty = self.slot_type(*lhs);
                if Self::fits_in_u64(lhs_ty) {
                    let l_slot = self.slot(*lhs);
                    let r_slot = self.slot(*rhs);
                    let idx = self.ops.len();
                    self.branch_fixups.push(idx);
                    match op {
                        CmpOp::Lt => self.emit(MicroOp::JumpLessU64 {
                            target: CodeOffset(encode_label(*l)),
                            lhs: FrameOffset(l_slot.offset),
                            rhs: FrameOffset(r_slot.offset),
                        }),
                        CmpOp::Ge => self.emit(MicroOp::JumpGreaterEqualU64 {
                            target: CodeOffset(encode_label(*l)),
                            lhs: FrameOffset(l_slot.offset),
                            rhs: FrameOffset(r_slot.offset),
                        }),
                        // x > y ↔ y < x
                        CmpOp::Gt => self.emit(MicroOp::JumpLessU64 {
                            target: CodeOffset(encode_label(*l)),
                            lhs: FrameOffset(r_slot.offset),
                            rhs: FrameOffset(l_slot.offset),
                        }),
                        // x <= y ↔ y >= x
                        CmpOp::Le => self.emit(MicroOp::JumpGreaterEqualU64 {
                            target: CodeOffset(encode_label(*l)),
                            lhs: FrameOffset(r_slot.offset),
                            rhs: FrameOffset(l_slot.offset),
                        }),
                        CmpOp::Neq => self.emit(MicroOp::JumpNotEqualU64 {
                            target: CodeOffset(encode_label(*l)),
                            lhs: FrameOffset(l_slot.offset),
                            rhs: FrameOffset(r_slot.offset),
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
                let src_ty = self.slot_type(*src);
                if Self::fits_in_u64(src_ty) {
                    let s = self.slot(*src);
                    let v = imm_to_u64(imm);
                    let idx = self.ops.len();
                    self.branch_fixups.push(idx);
                    match op {
                        CmpOp::Ge => self.emit(MicroOp::JumpGreaterEqualU64Imm {
                            target: CodeOffset(encode_label(*l)),
                            src: FrameOffset(s.offset),
                            imm: v,
                        }),
                        CmpOp::Lt => self.emit(MicroOp::JumpLessU64Imm {
                            target: CodeOffset(encode_label(*l)),
                            src: FrameOffset(s.offset),
                            imm: v,
                        }),
                        CmpOp::Gt => self.emit(MicroOp::JumpGreaterU64Imm {
                            target: CodeOffset(encode_label(*l)),
                            src: FrameOffset(s.offset),
                            imm: v,
                        }),
                        CmpOp::Le => self.emit(MicroOp::JumpLessEqualU64Imm {
                            target: CodeOffset(encode_label(*l)),
                            src: FrameOffset(s.offset),
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
                self.lower_call(func_ir, args, rets);
            },
            Instr::CallGeneric(rets, _inst_idx, args) => {
                self.lower_call(func_ir, args, rets);
            },

            // --- Return ---
            Instr::Ret(slots) => {
                for (k, slot) in slots.iter().enumerate() {
                    let src = self.slot(*slot);
                    let dst = self.ctx.return_slots[k];
                    self.emit_move(dst, src);
                }
                self.emit(MicroOp::Return);
            },

            _ => bail!("instruction {} not yet lowered", instr.opcode_name()),
        }
        Ok(())
    }

    fn lower_call(&mut self, _func_ir: &FunctionIR, args: &[Slot], rets: &[Slot]) {
        let call_site = &self.ctx.call_sites[self.call_site_cursor];

        // Copy arguments into callee's parameter slots.
        // active_xfer_slots already set for this call's args by prior def_slot calls.
        for (k, arg_slot) in args.iter().enumerate() {
            let src = self.slot(*arg_slot);
            let dst = call_site.arg_write_slots[k];
            self.emit_move(dst, src);
        }

        self.emit(MicroOp::CallFunc {
            func_id: call_site.callee_func_id,
        });
        self.call_site_cursor += 1;

        // Update active_xfer_slots for ret Xfer positions before doing ret copies.
        // For each ret that targets a Xfer(j), resolve its real slot:
        //  - If there's a next callsite with a param at position j, use that
        //    callsite's arg_write_slots[j] so the later xfer copy is elided.
        //  - Otherwise fall back to ret_read_slots[k] (no-copy).
        let prev = self.call_site_cursor - 1;
        let prev_cs = &self.ctx.call_sites[prev];
        for (k, ret_slot) in rets.iter().enumerate() {
            if let Slot::Xfer(j) = ret_slot {
                let j = *j as usize;
                let mut resolved = false;
                if self.call_site_cursor < self.ctx.call_sites.len() {
                    let next_cs = &self.ctx.call_sites[self.call_site_cursor];
                    if j < next_cs.arg_write_slots.len() {
                        self.active_xfer_slots[j] = next_cs.arg_write_slots[j];
                        self.active_xfer_types[j] = next_cs.param_types[j];
                        resolved = true;
                    }
                }
                if !resolved && k < prev_cs.ret_read_slots.len() {
                    self.active_xfer_slots[j] = prev_cs.ret_read_slots[k];
                    self.active_xfer_types[j] = prev_cs.ret_types[k];
                }
            }
        }

        // Move return values to destination slots.
        for (k, ret_slot) in rets.iter().enumerate() {
            let src = prev_cs.ret_read_slots[k];
            let dst = self.slot(*ret_slot);
            self.emit_move(dst, src);
        }
    }

    fn fixup_branches(&mut self) -> Result<()> {
        for &idx in &self.branch_fixups {
            // Extract the encoded label from the op, resolve it, then patch.
            let encoded = match &self.ops[idx] {
                MicroOp::Jump { target }
                | MicroOp::JumpNotZeroU64 { target, .. }
                | MicroOp::JumpGreaterEqualU64Imm { target, .. }
                | MicroOp::JumpLessU64Imm { target, .. }
                | MicroOp::JumpGreaterU64Imm { target, .. }
                | MicroOp::JumpLessEqualU64Imm { target, .. }
                | MicroOp::JumpLessU64 { target, .. }
                | MicroOp::JumpGreaterEqualU64 { target, .. }
                | MicroOp::JumpNotEqualU64 { target, .. } => target.0,
                other => bail!(
                    "unexpected non-branch op at fixup index {}: {:?}",
                    idx,
                    other
                ),
            };
            let label = decode_label(encoded);
            let resolved = self.resolve_label(label)?;
            match &mut self.ops[idx] {
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
