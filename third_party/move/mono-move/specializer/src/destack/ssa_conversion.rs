// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Bytecode-to-SSA conversion.
//!
//! Simulates the operand stack, assigning fresh sequential value IDs
//! (pure SSA within each basic block). Locals (params + declared locals)
//! are mutable across blocks and keep their original slot indices.

use super::{
    ssa_function::SSAFunction,
    type_conversion::{convert_sig_token, convert_sig_tokens},
};
use crate::stackless_exec_ir::{BasicBlock, BinaryOp, CmpOp, Instr, Label, Slot, UnaryOp};
use anyhow::{bail, ensure, Context, Result};
use mono_move_core::types::{self as ty, InternedType, InternedTypeList};
use mono_move_global_context::ExecutionGuard;
use move_binary_format::{
    access::ModuleAccess,
    file_format::{
        Bytecode, CodeOffset, FieldHandleIndex, FieldInstantiationIndex, SignatureToken,
        StructDefInstantiationIndex, StructDefinitionIndex, StructFieldInformation,
        StructVariantInstantiationIndex, VariantFieldHandleIndex, VariantFieldInstantiationIndex,
        VariantIndex,
    },
    CompiledModule,
};
use shared_dsa::{Entry, UnorderedMap};
use std::ops::Range;

// ================================================================================================
// Pass: Bytecode -> Intra-Block SSA
// ================================================================================================

/// Split bytecode into basic blocks, returning half-open `start..end` ranges.
///
/// A new block starts at every branch target (present in `label_map`) and after
/// every terminator (`Branch`, `BrTrue`, `BrFalse`, `Ret`, `Abort`, `AbortMsg`).
fn split_bytecode_into_blocks(
    code: &[Bytecode],
    label_map: &UnorderedMap<CodeOffset, Label>,
) -> Result<Vec<Range<usize>>> {
    let mut blocks = Vec::new();
    let mut start = 0;

    for (offset, bc) in code.iter().enumerate() {
        if offset > start && label_map.contains_key(&(offset as CodeOffset)) {
            blocks.push(start..offset);
            start = offset;
        }
        match bc {
            Bytecode::Branch(_)
            | Bytecode::BrTrue(_)
            | Bytecode::BrFalse(_)
            | Bytecode::Ret
            | Bytecode::Abort
            | Bytecode::AbortMsg => {
                blocks.push(start..offset + 1);
                start = offset + 1;
            },
            _ => {},
        }
    }
    // The bytecode verifier's `verify_fallthrough` rejects code whose last
    // instruction is not an unconditional branch (Ret/Abort/Branch), so every
    // block-ending terminator in the loop above will have consumed all code.
    ensure!(
        start >= code.len(),
        "verified bytecode must end with a terminator"
    );
    Ok(blocks)
}

pub(crate) struct SsaConverter<'a> {
    /// Next value ID number (0-based, monotonically increasing across blocks).
    next_vid: u16,
    /// Simulated operand stack.
    stack: Vec<Slot>,
    /// Types of all locals (params ++ declared locals).
    local_types: Vec<InternedType>,
    /// Types of value IDs, indexed directly by value ID number.
    vid_types: Vec<InternedType>,
    /// Execution guard for interning types.
    guard: &'a ExecutionGuard<'a>,
    /// Pre-resolved struct types, indexed by StructHandleIndex ordinal.
    /// `None` entries denote handles the orchestrator could not resolve
    /// (imported or generic); the converter must reject any code that
    /// references them.
    struct_types: &'a [Option<InternedType>],
    /// Completed basic blocks.
    blocks: Vec<BasicBlock>,
    /// Instructions for the current block being built.
    current_block_instrs: Vec<Instr>,
    /// Label for the current block being built. `None` before the first block starts.
    current_block_label: Option<Label>,
    /// Map from bytecode offset to label
    label_map: UnorderedMap<CodeOffset, Label>,
    /// Next label index
    next_label: u16,
}

impl<'a> SsaConverter<'a> {
    pub(crate) fn new(
        local_types: Vec<InternedType>,
        guard: &'a ExecutionGuard<'a>,
        struct_types: &'a [Option<InternedType>],
    ) -> Self {
        Self {
            next_vid: 0,
            stack: Vec::new(),
            local_types,
            vid_types: Vec::new(),
            guard,
            struct_types,
            blocks: Vec::new(),
            current_block_instrs: Vec::new(),
            current_block_label: None,
            label_map: UnorderedMap::new(),
            next_label: 0,
        }
    }

    fn alloc_vid(&mut self, ty: InternedType) -> Result<Slot> {
        let vid = Slot::Vid(self.next_vid);
        self.next_vid = self
            .next_vid
            .checked_add(1)
            .context("too many SSA values (Vid overflow)")?;
        self.vid_types.push(ty);
        Ok(vid)
    }

    fn push_slot(&mut self, r: Slot) {
        debug_assert!(r.is_vid(), "only Vid slots belong on the operand stack");
        self.stack.push(r);
    }

    fn pop_slot(&mut self) -> Result<Slot> {
        self.stack.pop().context("stack underflow")
    }

    fn pop_n_reverse(&mut self, n: usize) -> Result<Vec<Slot>> {
        ensure!(self.stack.len() >= n, "stack underflow");
        let start = self.stack.len() - n;
        Ok(self.stack.drain(start..).collect())
    }

    /// Returns the type of a Vid slot by looking it up in `vid_types`.
    /// Invariant: only Vid slots appear on the operand stack.
    fn vid_type(&self, slot: Slot) -> Result<InternedType> {
        match slot {
            Slot::Vid(id) => self
                .vid_types
                .get(id as usize)
                .copied()
                .with_context(|| format!("Vid id {} out of range", id)),
            other => bail!("expected Vid slot on operand stack, got {:?}", other),
        }
    }

    fn get_or_create_label(&mut self, offset: CodeOffset) -> Label {
        match self.label_map.entry(offset) {
            Entry::Occupied(e) => *e.get(),
            Entry::Vacant(e) => {
                let label = Label(self.next_label);
                self.next_label += 1;
                *e.insert(label)
            },
        }
    }

    fn assign_labels(&mut self, code: &[Bytecode]) {
        for (offset, bc) in code.iter().enumerate() {
            match bc {
                Bytecode::Branch(target) | Bytecode::BrTrue(target) | Bytecode::BrFalse(target) => {
                    self.get_or_create_label(*target);
                },
                _ => {},
            }
            // Conditional branches also need a label for the fall-through target.
            if matches!(bc, Bytecode::BrTrue(_) | Bytecode::BrFalse(_)) {
                self.get_or_create_label((offset + 1) as CodeOffset);
            }
        }
    }

    // --------------------------------------------------------------------------------------------
    // Type helpers
    // --------------------------------------------------------------------------------------------

    fn struct_type(
        &self,
        module: &CompiledModule,
        idx: StructDefinitionIndex,
    ) -> Result<InternedType> {
        let handle = module.struct_defs[idx.0 as usize].struct_handle;
        match self.struct_types.get(handle.0 as usize) {
            Some(Some(ty)) => Ok(*ty),
            Some(None) => bail!(
                "unresolved struct handle {} for local struct definition {}",
                handle.0,
                idx.0
            ),
            None => bail!("struct handle index {} out of bounds", handle.0),
        }
    }

    fn struct_inst_type(
        &self,
        module: &CompiledModule,
        idx: StructDefInstantiationIndex,
    ) -> Result<InternedType> {
        // TODO: when generic-instantiation interning lands, return a
        // distinct type for the instantiation. For now, the base struct
        // type is what the rest of the pipeline uses.
        let inst = &module.struct_def_instantiations[idx.0 as usize];
        self.struct_type(module, inst.def)
    }

    /// Returns `(base_struct_ty, ty_args_list)` for a generic struct
    /// instantiation. `base_struct_ty` is the (uninstantiated) struct type;
    /// `ty_args_list` is the interned list of type arguments.
    ///
    /// Because proper generic-instantiation interning is not yet implemented,
    /// this pair preserves the information needed for later monomorphization
    /// without producing a distinct instantiated struct type.
    fn struct_inst_parts(
        &self,
        module: &CompiledModule,
        idx: StructDefInstantiationIndex,
    ) -> Result<(InternedType, InternedTypeList)> {
        let inst = &module.struct_def_instantiations[idx.0 as usize];
        let base_ty = self.struct_type(module, inst.def)?;
        let ty_args_toks = &module.signature_at(inst.type_parameters).0;
        let ty_args = convert_sig_tokens(ty_args_toks, self.guard, self.struct_types)?;
        let ty_args_ptr = self.guard.intern_type_list(&ty_args);
        Ok((base_ty, ty_args_ptr))
    }

    /// Returns `(enum_ty, variant_ordinal, ty_args_list)` for a generic
    /// enum-variant instantiation.
    fn variant_inst_parts(
        &self,
        module: &CompiledModule,
        idx: StructVariantInstantiationIndex,
    ) -> Result<(InternedType, u16, InternedTypeList)> {
        let inst = &module.struct_variant_instantiations[idx.0 as usize];
        let handle = &module.struct_variant_handles[inst.handle.0 as usize];
        let enum_ty = self.struct_type(module, handle.struct_index)?;
        let ty_args_toks = &module.signature_at(inst.type_parameters).0;
        let ty_args = convert_sig_tokens(ty_args_toks, self.guard, self.struct_types)?;
        let ty_args_ptr = self.guard.intern_type_list(&ty_args);
        Ok((enum_ty, handle.variant, ty_args_ptr))
    }

    // TODO: reconsider field-type resolution.
    //
    // Current (walk on demand):
    //   + no extra builder pass or storage
    //   + pays cost only for fields actually accessed
    //   - re-walks the same field's signature on each access
    //   - field resolution scattered across destack; not a single-step lookup
    //
    // Alternative (pre-resolve into a table, indexed by field handle):
    //   + O(1) lookup here; mirrors how `struct_types` is handled
    //   + single authoritative resolution pass in the builder
    //   - extra builder pass + `Vec<InternedType>` storage per module
    //   - pays cost upfront, including for never-accessed fields
    fn field_type(&self, module: &CompiledModule, idx: FieldHandleIndex) -> Result<InternedType> {
        let handle = &module.field_handles[idx.0 as usize];
        let tok = match &module.struct_defs[handle.owner.0 as usize].field_information {
            StructFieldInformation::Declared(fields) => {
                fields[handle.field as usize].signature.0.clone()
            },
            _ => bail!("field access on native/variant struct"),
        };
        convert_sig_token(&tok, self.guard, self.struct_types)
    }

    fn field_inst_type(
        &self,
        module: &CompiledModule,
        idx: FieldInstantiationIndex,
    ) -> Result<InternedType> {
        let inst = &module.field_instantiations[idx.0 as usize];
        let handle = &module.field_handles[inst.handle.0 as usize];
        let base_tok = match &module.struct_defs[handle.owner.0 as usize].field_information {
            StructFieldInformation::Declared(fields) => {
                fields[handle.field as usize].signature.0.clone()
            },
            _ => bail!("field access on native/variant struct"),
        };
        let type_params = &module.signature_at(inst.type_parameters).0;
        let tok = substitute_type_params(&base_tok, type_params);
        convert_sig_token(&tok, self.guard, self.struct_types)
    }

    fn variant_field_handle_type(
        &self,
        module: &CompiledModule,
        idx: VariantFieldHandleIndex,
    ) -> Result<InternedType> {
        let handle = &module.variant_field_handles[idx.0 as usize];
        let tok = match &module.struct_defs[handle.struct_index.0 as usize].field_information {
            StructFieldInformation::DeclaredVariants(variants) => {
                variants[handle.variants[0] as usize].fields[handle.field as usize]
                    .signature
                    .0
                    .clone()
            },
            _ => bail!("variant field access on non-variant struct"),
        };
        convert_sig_token(&tok, self.guard, self.struct_types)
    }

    fn variant_field_inst_type(
        &self,
        module: &CompiledModule,
        idx: VariantFieldInstantiationIndex,
    ) -> Result<InternedType> {
        let inst = &module.variant_field_instantiations[idx.0 as usize];
        let handle = &module.variant_field_handles[inst.handle.0 as usize];
        let base_tok = match &module.struct_defs[handle.struct_index.0 as usize].field_information {
            StructFieldInformation::DeclaredVariants(variants) => {
                variants[handle.variants[0] as usize].fields[handle.field as usize]
                    .signature
                    .0
                    .clone()
            },
            _ => bail!("variant field access on non-variant struct"),
        };
        let type_params = &module.signature_at(inst.type_parameters).0;
        let tok = substitute_type_params(&base_tok, type_params);
        convert_sig_token(&tok, self.guard, self.struct_types)
    }

    // --------------------------------------------------------------------------------------------
    // Function conversion
    // --------------------------------------------------------------------------------------------

    /// Finalize the current block and start a new one with the given label.
    fn start_new_block(&mut self, label: Label) {
        self.finalize_current_block();
        self.current_block_label = Some(label);
    }

    /// Push the current block onto the completed blocks list.
    fn finalize_current_block(&mut self) {
        if let Some(label) = self.current_block_label.take() {
            self.blocks.push(BasicBlock {
                label,
                instrs: std::mem::take(&mut self.current_block_instrs),
            });
        }
    }

    /// Converts the function's bytecode into SSA form, consuming the converter.
    pub(crate) fn convert_function(
        mut self,
        module: &CompiledModule,
        code: &[Bytecode],
    ) -> Result<SSAFunction> {
        self.assign_labels(code);

        let block_boundaries = split_bytecode_into_blocks(code, &self.label_map)?;

        for block in block_boundaries {
            ensure!(
                self.stack.is_empty(),
                "stack must be empty at block boundary"
            );

            // Every block gets a label (assigned on-demand if not already a branch target).
            let label = self.get_or_create_label(block.start as CodeOffset);
            self.start_new_block(label);

            for bc in &code[block] {
                self.convert_bytecode(module, bc)?;
            }
        }
        self.finalize_current_block();

        Ok(SSAFunction {
            blocks: self.blocks,
            vid_types: self.vid_types,
            local_types: self.local_types,
        })
    }

    /// Converts a single stack-based bytecode into slot-based SSA instruction(s).
    ///
    /// Each bytecode pops its operands from the simulated stack, allocates a fresh
    /// value ID for each result, emits the corresponding slot-based instruction, and
    /// pushes the results back. The stack is only a compile-time simulation — the
    /// emitted IR is purely slot-based.
    fn convert_bytecode(&mut self, module: &CompiledModule, bc: &Bytecode) -> Result<()> {
        use Bytecode as B;
        match bc {
            // --- Loads ---
            B::LdU8(v) => {
                let dst = self.alloc_vid(ty::U8_TY)?;
                self.current_block_instrs.push(Instr::LdU8(dst, *v));
                self.push_slot(dst);
            },
            B::LdU16(v) => {
                let dst = self.alloc_vid(ty::U16_TY)?;
                self.current_block_instrs.push(Instr::LdU16(dst, *v));
                self.push_slot(dst);
            },
            B::LdU32(v) => {
                let dst = self.alloc_vid(ty::U32_TY)?;
                self.current_block_instrs.push(Instr::LdU32(dst, *v));
                self.push_slot(dst);
            },
            B::LdU64(v) => {
                let dst = self.alloc_vid(ty::U64_TY)?;
                self.current_block_instrs.push(Instr::LdU64(dst, *v));
                self.push_slot(dst);
            },
            B::LdU128(v) => {
                let dst = self.alloc_vid(ty::U128_TY)?;
                self.current_block_instrs.push(Instr::LdU128(dst, *v));
                self.push_slot(dst);
            },
            B::LdU256(v) => {
                let dst = self.alloc_vid(ty::U256_TY)?;
                self.current_block_instrs.push(Instr::LdU256(dst, *v));
                self.push_slot(dst);
            },
            B::LdI8(v) => {
                let dst = self.alloc_vid(ty::I8_TY)?;
                self.current_block_instrs.push(Instr::LdI8(dst, *v));
                self.push_slot(dst);
            },
            B::LdI16(v) => {
                let dst = self.alloc_vid(ty::I16_TY)?;
                self.current_block_instrs.push(Instr::LdI16(dst, *v));
                self.push_slot(dst);
            },
            B::LdI32(v) => {
                let dst = self.alloc_vid(ty::I32_TY)?;
                self.current_block_instrs.push(Instr::LdI32(dst, *v));
                self.push_slot(dst);
            },
            B::LdI64(v) => {
                let dst = self.alloc_vid(ty::I64_TY)?;
                self.current_block_instrs.push(Instr::LdI64(dst, *v));
                self.push_slot(dst);
            },
            B::LdI128(v) => {
                let dst = self.alloc_vid(ty::I128_TY)?;
                self.current_block_instrs.push(Instr::LdI128(dst, *v));
                self.push_slot(dst);
            },
            B::LdI256(v) => {
                let dst = self.alloc_vid(ty::I256_TY)?;
                self.current_block_instrs.push(Instr::LdI256(dst, *v));
                self.push_slot(dst);
            },
            B::LdConst(idx) => {
                let tok = &module.constant_pool[idx.0 as usize].type_;
                let ty = convert_sig_token(tok, self.guard, self.struct_types)?;
                let dst = self.alloc_vid(ty)?;
                self.current_block_instrs.push(Instr::LdConst(dst, *idx));
                self.push_slot(dst);
            },
            B::LdTrue => {
                let dst = self.alloc_vid(ty::BOOL_TY)?;
                self.current_block_instrs.push(Instr::LdTrue(dst));
                self.push_slot(dst);
            },
            B::LdFalse => {
                let dst = self.alloc_vid(ty::BOOL_TY)?;
                self.current_block_instrs.push(Instr::LdFalse(dst));
                self.push_slot(dst);
            },

            // --- Locals ---
            B::CopyLoc(idx) => {
                let src = Slot::Home(*idx as u16);
                let ty = self.local_types[*idx as usize];
                let dst = self.alloc_vid(ty)?;
                self.current_block_instrs.push(Instr::Copy(dst, src));
                self.push_slot(dst);
            },
            B::MoveLoc(idx) => {
                let src = Slot::Home(*idx as u16);
                let ty = self.local_types[*idx as usize];
                let dst = self.alloc_vid(ty)?;
                self.current_block_instrs.push(Instr::Move(dst, src));
                self.push_slot(dst);
            },
            B::StLoc(idx) => {
                let src = self.pop_slot()?;
                let dst = Slot::Home(*idx as u16);
                self.current_block_instrs.push(Instr::Move(dst, src));
            },

            // --- Pop ---
            B::Pop => {
                let _ = self.pop_slot()?;
            },

            // --- Binary ops (result type = operand type) ---
            B::Add => self.convert_binop(BinaryOp::Add, false)?,
            B::Sub => self.convert_binop(BinaryOp::Sub, false)?,
            B::Mul => self.convert_binop(BinaryOp::Mul, false)?,
            B::Div => self.convert_binop(BinaryOp::Div, false)?,
            B::Mod => self.convert_binop(BinaryOp::Mod, false)?,
            B::BitOr => self.convert_binop(BinaryOp::BitOr, false)?,
            B::BitAnd => self.convert_binop(BinaryOp::BitAnd, false)?,
            B::Xor => self.convert_binop(BinaryOp::Xor, false)?,
            B::Shl => self.convert_binop(BinaryOp::Shl, false)?,
            B::Shr => self.convert_binop(BinaryOp::Shr, false)?,
            // --- Comparisons / logical (result type = bool) ---
            B::Lt => self.convert_binop(BinaryOp::Cmp(CmpOp::Lt), true)?,
            B::Gt => self.convert_binop(BinaryOp::Cmp(CmpOp::Gt), true)?,
            B::Le => self.convert_binop(BinaryOp::Cmp(CmpOp::Le), true)?,
            B::Ge => self.convert_binop(BinaryOp::Cmp(CmpOp::Ge), true)?,
            B::Eq => self.convert_binop(BinaryOp::Cmp(CmpOp::Eq), true)?,
            B::Neq => self.convert_binop(BinaryOp::Cmp(CmpOp::Neq), true)?,
            B::Or => self.convert_binop(BinaryOp::Or, true)?,
            B::And => self.convert_binop(BinaryOp::And, true)?,

            // --- Unary ops (result type specified) ---
            B::CastU8 => self.convert_unop(UnaryOp::CastU8, ty::U8_TY)?,
            B::CastU16 => self.convert_unop(UnaryOp::CastU16, ty::U16_TY)?,
            B::CastU32 => self.convert_unop(UnaryOp::CastU32, ty::U32_TY)?,
            B::CastU64 => self.convert_unop(UnaryOp::CastU64, ty::U64_TY)?,
            B::CastU128 => self.convert_unop(UnaryOp::CastU128, ty::U128_TY)?,
            B::CastU256 => self.convert_unop(UnaryOp::CastU256, ty::U256_TY)?,
            B::CastI8 => self.convert_unop(UnaryOp::CastI8, ty::I8_TY)?,
            B::CastI16 => self.convert_unop(UnaryOp::CastI16, ty::I16_TY)?,
            B::CastI32 => self.convert_unop(UnaryOp::CastI32, ty::I32_TY)?,
            B::CastI64 => self.convert_unop(UnaryOp::CastI64, ty::I64_TY)?,
            B::CastI128 => self.convert_unop(UnaryOp::CastI128, ty::I128_TY)?,
            B::CastI256 => self.convert_unop(UnaryOp::CastI256, ty::I256_TY)?,
            B::Not => self.convert_unop(UnaryOp::Not, ty::BOOL_TY)?,
            // --- Unary ops (result type derived from operand) ---
            B::Negate => {
                let src_ty = self.vid_type(*self.stack.last().context("stack underflow")?)?;
                self.convert_unop(UnaryOp::Negate, src_ty)?;
            },
            B::FreezeRef => {
                let src_ty = self.vid_type(*self.stack.last().context("stack underflow")?)?;
                // The bytecode verifier guarantees the operand is &mut T.
                let result_ty = self.guard.convert_mut_to_immut_ref(src_ty)?;
                self.convert_unop(UnaryOp::FreezeRef, result_ty)?;
            },

            // --- Struct ops ---
            B::Pack(idx) => {
                let n = struct_field_count(module, *idx);
                let fields = self.pop_n_reverse(n)?;
                let result_ty = self.struct_type(module, *idx)?;
                let dst = self.alloc_vid(result_ty)?;
                self.current_block_instrs
                    .push(Instr::Pack(dst, result_ty, fields));
                self.push_slot(dst);
            },
            B::PackGeneric(idx) => {
                let inst = &module.struct_def_instantiations[idx.0 as usize];
                let n = struct_field_count(module, inst.def);
                let fields = self.pop_n_reverse(n)?;
                let result_ty = self.struct_inst_type(module, *idx)?;
                let (base_ty, ty_args) = self.struct_inst_parts(module, *idx)?;
                let dst = self.alloc_vid(result_ty)?;
                self.current_block_instrs
                    .push(Instr::PackGeneric(dst, base_ty, ty_args, fields));
                self.push_slot(dst);
            },
            B::Unpack(idx) => {
                let src = self.pop_slot()?;
                let n = struct_field_count(module, *idx);
                let ftypes = struct_field_type_toks(module, *idx);
                let ftypes: Vec<InternedType> =
                    convert_sig_tokens(&ftypes, self.guard, self.struct_types)?;
                let mut dsts = Vec::with_capacity(n);
                for i in 0..n {
                    let fty = ftypes
                        .get(i)
                        .copied()
                        .context("field type index out of bounds")?;
                    dsts.push(self.alloc_vid(fty)?);
                }
                let struct_ty = self.struct_type(module, *idx)?;
                self.current_block_instrs
                    .push(Instr::Unpack(dsts.clone(), struct_ty, src));
                for dst in dsts {
                    self.push_slot(dst);
                }
            },
            B::UnpackGeneric(idx) => {
                let src = self.pop_slot()?;
                let inst = &module.struct_def_instantiations[idx.0 as usize];
                let n = struct_field_count(module, inst.def);
                let type_params = module.signature_at(inst.type_parameters).0.clone();
                let raw_ftypes = struct_field_type_toks(module, inst.def);
                let ftypes_tok: Vec<SignatureToken> = raw_ftypes
                    .iter()
                    .map(|ft| substitute_type_params(ft, &type_params))
                    .collect();
                let ftypes: Vec<InternedType> =
                    convert_sig_tokens(&ftypes_tok, self.guard, self.struct_types)?;
                let mut dsts = Vec::with_capacity(n);
                for i in 0..n {
                    let fty = ftypes
                        .get(i)
                        .copied()
                        .context("field type index out of bounds")?;
                    dsts.push(self.alloc_vid(fty)?);
                }
                let (base_ty, ty_args) = self.struct_inst_parts(module, *idx)?;
                self.current_block_instrs.push(Instr::UnpackGeneric(
                    dsts.clone(),
                    base_ty,
                    ty_args,
                    src,
                ));
                for dst in dsts {
                    self.push_slot(dst);
                }
            },

            // --- Variant ops ---
            B::PackVariant(idx) => {
                let handle = &module.struct_variant_handles[idx.0 as usize];
                let variant = handle.variant;
                let n = variant_field_count(module, handle.struct_index, variant);
                let fields = self.pop_n_reverse(n)?;
                let result_ty = self.struct_type(module, handle.struct_index)?;
                let dst = self.alloc_vid(result_ty)?;
                self.current_block_instrs
                    .push(Instr::PackVariant(dst, result_ty, variant, fields));
                self.push_slot(dst);
            },
            B::PackVariantGeneric(idx) => {
                let inst = &module.struct_variant_instantiations[idx.0 as usize];
                let handle = &module.struct_variant_handles[inst.handle.0 as usize];
                let n = variant_field_count(module, handle.struct_index, handle.variant);
                let fields = self.pop_n_reverse(n)?;
                let type_params = module.signature_at(inst.type_parameters).0.clone();
                let def = &module.struct_defs[handle.struct_index.0 as usize];
                let tok = SignatureToken::StructInstantiation(def.struct_handle, type_params);
                let result_ty = convert_sig_token(&tok, self.guard, self.struct_types)?;
                let (enum_ty, variant, ty_args) = self.variant_inst_parts(module, *idx)?;
                let dst = self.alloc_vid(result_ty)?;
                self.current_block_instrs.push(Instr::PackVariantGeneric(
                    dst, enum_ty, variant, ty_args, fields,
                ));
                self.push_slot(dst);
            },
            B::UnpackVariant(idx) => {
                let src = self.pop_slot()?;
                let handle = &module.struct_variant_handles[idx.0 as usize];
                let variant = handle.variant;
                let n = variant_field_count(module, handle.struct_index, variant);
                let ftypes_tok = variant_field_type_toks(module, handle.struct_index, variant);
                let ftypes: Vec<InternedType> =
                    convert_sig_tokens(&ftypes_tok, self.guard, self.struct_types)?;
                let mut dsts = Vec::with_capacity(n);
                for i in 0..n {
                    let fty = ftypes
                        .get(i)
                        .copied()
                        .context("field type index out of bounds")?;
                    dsts.push(self.alloc_vid(fty)?);
                }
                let enum_ty = self.struct_type(module, handle.struct_index)?;
                self.current_block_instrs.push(Instr::UnpackVariant(
                    dsts.clone(),
                    enum_ty,
                    variant,
                    src,
                ));
                for dst in dsts {
                    self.push_slot(dst);
                }
            },
            B::UnpackVariantGeneric(idx) => {
                let src = self.pop_slot()?;
                let inst = &module.struct_variant_instantiations[idx.0 as usize];
                let handle = &module.struct_variant_handles[inst.handle.0 as usize];
                let n = variant_field_count(module, handle.struct_index, handle.variant);
                let type_params = module.signature_at(inst.type_parameters).0.clone();
                let raw_ftypes =
                    variant_field_type_toks(module, handle.struct_index, handle.variant);
                let ftypes_tok: Vec<SignatureToken> = raw_ftypes
                    .iter()
                    .map(|ft| substitute_type_params(ft, &type_params))
                    .collect();
                let ftypes: Vec<InternedType> =
                    convert_sig_tokens(&ftypes_tok, self.guard, self.struct_types)?;
                let mut dsts = Vec::with_capacity(n);
                for i in 0..n {
                    let fty = ftypes
                        .get(i)
                        .copied()
                        .context("field type index out of bounds")?;
                    dsts.push(self.alloc_vid(fty)?);
                }
                let (enum_ty, variant, ty_args) = self.variant_inst_parts(module, *idx)?;
                self.current_block_instrs.push(Instr::UnpackVariantGeneric(
                    dsts.clone(),
                    enum_ty,
                    variant,
                    ty_args,
                    src,
                ));
                for dst in dsts {
                    self.push_slot(dst);
                }
            },
            B::TestVariant(idx) => {
                let src = self.pop_slot()?;
                let handle = &module.struct_variant_handles[idx.0 as usize];
                let variant = handle.variant;
                let enum_ty = self.struct_type(module, handle.struct_index)?;
                let dst = self.alloc_vid(ty::BOOL_TY)?;
                self.current_block_instrs
                    .push(Instr::TestVariant(dst, enum_ty, variant, src));
                self.push_slot(dst);
            },
            B::TestVariantGeneric(idx) => {
                let src = self.pop_slot()?;
                let (enum_ty, variant, ty_args) = self.variant_inst_parts(module, *idx)?;
                let dst = self.alloc_vid(ty::BOOL_TY)?;
                self.current_block_instrs.push(Instr::TestVariantGeneric(
                    dst, enum_ty, variant, ty_args, src,
                ));
                self.push_slot(dst);
            },

            // --- References ---
            B::ImmBorrowLoc(idx) => {
                let src = Slot::Home(*idx as u16);
                let inner = self.local_types[*idx as usize];
                let ty = self.guard.intern_immut_ref(inner);
                let dst = self.alloc_vid(ty)?;
                self.current_block_instrs
                    .push(Instr::ImmBorrowLoc(dst, src));
                self.push_slot(dst);
            },
            B::MutBorrowLoc(idx) => {
                let src = Slot::Home(*idx as u16);
                let inner = self.local_types[*idx as usize];
                let ty = self.guard.intern_mut_ref(inner);
                let dst = self.alloc_vid(ty)?;
                self.current_block_instrs
                    .push(Instr::MutBorrowLoc(dst, src));
                self.push_slot(dst);
            },
            B::ImmBorrowField(idx) => {
                let src = self.pop_slot()?;
                let fty = self.field_type(module, *idx)?;
                let ty = self.guard.intern_immut_ref(fty);
                let dst = self.alloc_vid(ty)?;
                self.current_block_instrs
                    .push(Instr::ImmBorrowField(dst, *idx, src));
                self.push_slot(dst);
            },
            B::MutBorrowField(idx) => {
                let src = self.pop_slot()?;
                let fty = self.field_type(module, *idx)?;
                let ty = self.guard.intern_mut_ref(fty);
                let dst = self.alloc_vid(ty)?;
                self.current_block_instrs
                    .push(Instr::MutBorrowField(dst, *idx, src));
                self.push_slot(dst);
            },
            B::ImmBorrowFieldGeneric(idx) => {
                let src = self.pop_slot()?;
                let fty = self.field_inst_type(module, *idx)?;
                let ty = self.guard.intern_immut_ref(fty);
                let dst = self.alloc_vid(ty)?;
                self.current_block_instrs
                    .push(Instr::ImmBorrowFieldGeneric(dst, *idx, src));
                self.push_slot(dst);
            },
            B::MutBorrowFieldGeneric(idx) => {
                let src = self.pop_slot()?;
                let fty = self.field_inst_type(module, *idx)?;
                let ty = self.guard.intern_mut_ref(fty);
                let dst = self.alloc_vid(ty)?;
                self.current_block_instrs
                    .push(Instr::MutBorrowFieldGeneric(dst, *idx, src));
                self.push_slot(dst);
            },
            B::ImmBorrowVariantField(idx) => {
                let src = self.pop_slot()?;
                let fty = self.variant_field_handle_type(module, *idx)?;
                let ty = self.guard.intern_immut_ref(fty);
                let dst = self.alloc_vid(ty)?;
                self.current_block_instrs
                    .push(Instr::ImmBorrowVariantField(dst, *idx, src));
                self.push_slot(dst);
            },
            B::MutBorrowVariantField(idx) => {
                let src = self.pop_slot()?;
                let fty = self.variant_field_handle_type(module, *idx)?;
                let ty = self.guard.intern_mut_ref(fty);
                let dst = self.alloc_vid(ty)?;
                self.current_block_instrs
                    .push(Instr::MutBorrowVariantField(dst, *idx, src));
                self.push_slot(dst);
            },
            B::ImmBorrowVariantFieldGeneric(idx) => {
                let src = self.pop_slot()?;
                let fty = self.variant_field_inst_type(module, *idx)?;
                let ty = self.guard.intern_immut_ref(fty);
                let dst = self.alloc_vid(ty)?;
                self.current_block_instrs
                    .push(Instr::ImmBorrowVariantFieldGeneric(dst, *idx, src));
                self.push_slot(dst);
            },
            B::MutBorrowVariantFieldGeneric(idx) => {
                let src = self.pop_slot()?;
                let fty = self.variant_field_inst_type(module, *idx)?;
                let ty = self.guard.intern_mut_ref(fty);
                let dst = self.alloc_vid(ty)?;
                self.current_block_instrs
                    .push(Instr::MutBorrowVariantFieldGeneric(dst, *idx, src));
                self.push_slot(dst);
            },
            B::ReadRef => {
                let src = self.pop_slot()?;
                let src_ty = self.vid_type(src)?;
                // The bytecode verifier guarantees the operand is `&T` or `&mut T`.
                let ty = self.guard.strip_ref(src_ty)?;
                let dst = self.alloc_vid(ty)?;
                self.current_block_instrs.push(Instr::ReadRef(dst, src));
                self.push_slot(dst);
            },
            B::WriteRef => {
                let ref_r = self.pop_slot()?;
                let val = self.pop_slot()?;
                self.current_block_instrs.push(Instr::WriteRef(ref_r, val));
            },

            // --- Globals ---
            B::Exists(idx) => {
                let addr = self.pop_slot()?;
                let struct_ty = self.struct_type(module, *idx)?;
                let dst = self.alloc_vid(ty::BOOL_TY)?;
                self.current_block_instrs
                    .push(Instr::Exists(dst, struct_ty, addr));
                self.push_slot(dst);
            },
            B::ExistsGeneric(idx) => {
                let addr = self.pop_slot()?;
                let (base_ty, ty_args) = self.struct_inst_parts(module, *idx)?;
                let dst = self.alloc_vid(ty::BOOL_TY)?;
                self.current_block_instrs
                    .push(Instr::ExistsGeneric(dst, base_ty, ty_args, addr));
                self.push_slot(dst);
            },
            B::MoveFrom(idx) => {
                let addr = self.pop_slot()?;
                let ty = self.struct_type(module, *idx)?;
                let dst = self.alloc_vid(ty)?;
                self.current_block_instrs
                    .push(Instr::MoveFrom(dst, ty, addr));
                self.push_slot(dst);
            },
            B::MoveFromGeneric(idx) => {
                let addr = self.pop_slot()?;
                let ty = self.struct_inst_type(module, *idx)?;
                let (base_ty, ty_args) = self.struct_inst_parts(module, *idx)?;
                let dst = self.alloc_vid(ty)?;
                self.current_block_instrs
                    .push(Instr::MoveFromGeneric(dst, base_ty, ty_args, addr));
                self.push_slot(dst);
            },
            B::MoveTo(idx) => {
                let val = self.pop_slot()?;
                let signer = self.pop_slot()?;
                let struct_ty = self.struct_type(module, *idx)?;
                self.current_block_instrs
                    .push(Instr::MoveTo(struct_ty, signer, val));
            },
            B::MoveToGeneric(idx) => {
                let val = self.pop_slot()?;
                let signer = self.pop_slot()?;
                let (base_ty, ty_args) = self.struct_inst_parts(module, *idx)?;
                self.current_block_instrs
                    .push(Instr::MoveToGeneric(base_ty, ty_args, signer, val));
            },
            B::ImmBorrowGlobal(idx) => {
                let addr = self.pop_slot()?;
                let inner = self.struct_type(module, *idx)?;
                let ty = self.guard.intern_immut_ref(inner);
                let dst = self.alloc_vid(ty)?;
                self.current_block_instrs
                    .push(Instr::ImmBorrowGlobal(dst, inner, addr));
                self.push_slot(dst);
            },
            B::ImmBorrowGlobalGeneric(idx) => {
                let addr = self.pop_slot()?;
                let inner = self.struct_inst_type(module, *idx)?;
                let ty = self.guard.intern_immut_ref(inner);
                let (base_ty, ty_args) = self.struct_inst_parts(module, *idx)?;
                let dst = self.alloc_vid(ty)?;
                self.current_block_instrs
                    .push(Instr::ImmBorrowGlobalGeneric(dst, base_ty, ty_args, addr));
                self.push_slot(dst);
            },
            B::MutBorrowGlobal(idx) => {
                let addr = self.pop_slot()?;
                let inner = self.struct_type(module, *idx)?;
                let ty = self.guard.intern_mut_ref(inner);
                let dst = self.alloc_vid(ty)?;
                self.current_block_instrs
                    .push(Instr::MutBorrowGlobal(dst, inner, addr));
                self.push_slot(dst);
            },
            B::MutBorrowGlobalGeneric(idx) => {
                let addr = self.pop_slot()?;
                let inner = self.struct_inst_type(module, *idx)?;
                let ty = self.guard.intern_mut_ref(inner);
                let (base_ty, ty_args) = self.struct_inst_parts(module, *idx)?;
                let dst = self.alloc_vid(ty)?;
                self.current_block_instrs
                    .push(Instr::MutBorrowGlobalGeneric(dst, base_ty, ty_args, addr));
                self.push_slot(dst);
            },

            // --- Calls ---
            B::Call(idx) => {
                let handle = module.function_handle_at(*idx);
                let num_args = module.signature_at(handle.parameters).0.len();
                let ret_toks = &module.signature_at(handle.return_).0;
                let ret_types = convert_sig_tokens(ret_toks, self.guard, self.struct_types)?;
                let args = self.pop_n_reverse(num_args)?;
                let mut rets = Vec::with_capacity(ret_types.len());
                for rty in ret_types {
                    rets.push(self.alloc_vid(rty)?);
                }
                self.current_block_instrs
                    .push(Instr::Call(rets.clone(), *idx, args));
                for r in rets {
                    self.push_slot(r);
                }
            },
            B::CallGeneric(idx) => {
                let inst = &module.function_instantiations[idx.0 as usize];
                let handle = module.function_handle_at(inst.handle);
                let num_args = module.signature_at(handle.parameters).0.len();
                let type_params = module.signature_at(inst.type_parameters).0.clone();
                let raw_ret_toks = &module.signature_at(handle.return_).0;
                let ret_toks: Vec<SignatureToken> = raw_ret_toks
                    .iter()
                    .map(|t| substitute_type_params(t, &type_params))
                    .collect();
                let ret_types = convert_sig_tokens(&ret_toks, self.guard, self.struct_types)?;
                let args = self.pop_n_reverse(num_args)?;
                let mut rets = Vec::with_capacity(ret_types.len());
                for rty in ret_types {
                    rets.push(self.alloc_vid(rty)?);
                }
                self.current_block_instrs
                    .push(Instr::CallGeneric(rets.clone(), *idx, args));
                for r in rets {
                    self.push_slot(r);
                }
            },

            // --- Closures ---
            B::PackClosure(fhi, mask) => {
                let captured_count = mask.captured_count() as usize;
                let captured = self.pop_n_reverse(captured_count)?;
                let handle = module.function_handle_at(*fhi);
                let params = &module.signature_at(handle.parameters).0;
                let returns = &module.signature_at(handle.return_).0;
                let tok = SignatureToken::Function(
                    params.clone(),
                    returns.clone(),
                    move_core_types::ability::AbilitySet::EMPTY,
                );
                let ty = convert_sig_token(&tok, self.guard, self.struct_types)?;
                let dst = self.alloc_vid(ty)?;
                self.current_block_instrs
                    .push(Instr::PackClosure(dst, *fhi, *mask, captured));
                self.push_slot(dst);
            },
            B::PackClosureGeneric(fii, mask) => {
                let captured_count = mask.captured_count() as usize;
                let captured = self.pop_n_reverse(captured_count)?;
                let inst = &module.function_instantiations[fii.0 as usize];
                let handle = module.function_handle_at(inst.handle);
                let params = &module.signature_at(handle.parameters).0;
                let returns = &module.signature_at(handle.return_).0;
                let tok = SignatureToken::Function(
                    params.clone(),
                    returns.clone(),
                    move_core_types::ability::AbilitySet::EMPTY,
                );
                let ty = convert_sig_token(&tok, self.guard, self.struct_types)?;
                let dst = self.alloc_vid(ty)?;
                self.current_block_instrs
                    .push(Instr::PackClosureGeneric(dst, *fii, *mask, captured));
                self.push_slot(dst);
            },
            B::CallClosure(sig_idx) => {
                let sig = module.signature_at(*sig_idx);
                let (num_args, ret_toks) =
                    if let Some(SignatureToken::Function(params, results, _)) = sig.0.first() {
                        (params.len(), results.clone())
                    } else {
                        bail!("CallClosure signature must start with a Function token")
                    };
                let ret_types = convert_sig_tokens(&ret_toks, self.guard, self.struct_types)?;
                let closure = self.pop_slot()?;
                let mut all_args = self.pop_n_reverse(num_args)?;
                all_args.push(closure);
                let mut rets = Vec::with_capacity(ret_types.len());
                for rty in &ret_types {
                    rets.push(self.alloc_vid(*rty)?);
                }
                let sig_types_vec = convert_sig_tokens(&sig.0, self.guard, self.struct_types)?;
                let signature_types = self.guard.intern_type_list(&sig_types_vec);
                self.current_block_instrs.push(Instr::CallClosure(
                    rets.clone(),
                    signature_types,
                    all_args,
                ));
                for r in rets {
                    self.push_slot(r);
                }
            },

            // --- Vector ops ---
            B::VecPack(sig_idx, count) => {
                let count = *count as u16;
                let elems = self.pop_n_reverse(count as usize)?;
                let elem_tok = &module.signature_at(*sig_idx).0[0];
                let elem_ty = convert_sig_token(elem_tok, self.guard, self.struct_types)?;
                let ty = self.guard.intern_vector(elem_ty);
                let dst = self.alloc_vid(ty)?;
                self.current_block_instrs
                    .push(Instr::VecPack(dst, elem_ty, count, elems));
                self.push_slot(dst);
            },
            B::VecLen(sig_idx) => {
                let vec_ref = self.pop_slot()?;
                let elem_tok = &module.signature_at(*sig_idx).0[0];
                let elem_ty = convert_sig_token(elem_tok, self.guard, self.struct_types)?;
                let dst = self.alloc_vid(ty::U64_TY)?;
                self.current_block_instrs
                    .push(Instr::VecLen(dst, elem_ty, vec_ref));
                self.push_slot(dst);
            },
            B::VecImmBorrow(sig_idx) => {
                let idx_r = self.pop_slot()?;
                let vec_ref = self.pop_slot()?;
                let elem_tok = &module.signature_at(*sig_idx).0[0];
                let elem_ty = convert_sig_token(elem_tok, self.guard, self.struct_types)?;
                let ty = self.guard.intern_immut_ref(elem_ty);
                let dst = self.alloc_vid(ty)?;
                self.current_block_instrs
                    .push(Instr::VecImmBorrow(dst, elem_ty, vec_ref, idx_r));
                self.push_slot(dst);
            },
            B::VecMutBorrow(sig_idx) => {
                let idx_r = self.pop_slot()?;
                let vec_ref = self.pop_slot()?;
                let elem_tok = &module.signature_at(*sig_idx).0[0];
                let elem_ty = convert_sig_token(elem_tok, self.guard, self.struct_types)?;
                let ty = self.guard.intern_mut_ref(elem_ty);
                let dst = self.alloc_vid(ty)?;
                self.current_block_instrs
                    .push(Instr::VecMutBorrow(dst, elem_ty, vec_ref, idx_r));
                self.push_slot(dst);
            },
            B::VecPushBack(sig_idx) => {
                let val = self.pop_slot()?;
                let vec_ref = self.pop_slot()?;
                let elem_tok = &module.signature_at(*sig_idx).0[0];
                let elem_ty = convert_sig_token(elem_tok, self.guard, self.struct_types)?;
                self.current_block_instrs
                    .push(Instr::VecPushBack(elem_ty, vec_ref, val));
            },
            B::VecPopBack(sig_idx) => {
                let vec_ref = self.pop_slot()?;
                let elem_tok = &module.signature_at(*sig_idx).0[0];
                let elem_ty = convert_sig_token(elem_tok, self.guard, self.struct_types)?;
                let dst = self.alloc_vid(elem_ty)?;
                self.current_block_instrs
                    .push(Instr::VecPopBack(dst, elem_ty, vec_ref));
                self.push_slot(dst);
            },
            B::VecUnpack(sig_idx, count) => {
                let count = *count as u16;
                let src = self.pop_slot()?;
                let elem_tok = &module.signature_at(*sig_idx).0[0];
                let elem_ty = convert_sig_token(elem_tok, self.guard, self.struct_types)?;
                let mut dsts = Vec::with_capacity(count as usize);
                for _ in 0..count {
                    dsts.push(self.alloc_vid(elem_ty)?);
                }
                self.current_block_instrs
                    .push(Instr::VecUnpack(dsts.clone(), elem_ty, count, src));
                for dst in dsts {
                    self.push_slot(dst);
                }
            },
            B::VecSwap(sig_idx) => {
                let j = self.pop_slot()?;
                let i = self.pop_slot()?;
                let vec_ref = self.pop_slot()?;
                let elem_tok = &module.signature_at(*sig_idx).0[0];
                let elem_ty = convert_sig_token(elem_tok, self.guard, self.struct_types)?;
                self.current_block_instrs
                    .push(Instr::VecSwap(elem_ty, vec_ref, i, j));
            },

            // --- Control flow ---
            B::Branch(target) => {
                let label = *self.label_map.get(target).expect("branch target label");
                self.current_block_instrs.push(Instr::Branch(label));
            },
            B::BrTrue(target) => {
                let cond = self.pop_slot()?;
                let label = *self.label_map.get(target).expect("branch target label");
                self.current_block_instrs.push(Instr::BrTrue(label, cond));
            },
            B::BrFalse(target) => {
                let cond = self.pop_slot()?;
                let label = *self.label_map.get(target).expect("branch target label");
                self.current_block_instrs.push(Instr::BrFalse(label, cond));
            },
            B::Ret => {
                let rets: Vec<Slot> = self.stack.drain(..).collect();
                self.current_block_instrs.push(Instr::Ret(rets));
            },
            B::Abort => {
                let code = self.pop_slot()?;
                self.current_block_instrs.push(Instr::Abort(code));
            },
            B::AbortMsg => {
                let msg = self.pop_slot()?;
                let code = self.pop_slot()?;
                self.current_block_instrs.push(Instr::AbortMsg(code, msg));
            },

            B::Nop => {},
        }
        Ok(())
    }

    fn convert_binop(&mut self, op: BinaryOp, result_is_bool: bool) -> Result<()> {
        let rhs = self.pop_slot()?;
        let lhs = self.pop_slot()?;
        let result_ty = if result_is_bool {
            ty::BOOL_TY
        } else {
            self.vid_type(lhs)?
        };
        let dst = self.alloc_vid(result_ty)?;
        self.current_block_instrs
            .push(Instr::BinaryOp(dst, op, lhs, rhs));
        self.push_slot(dst);
        Ok(())
    }

    fn convert_unop(&mut self, op: UnaryOp, result_ty: InternedType) -> Result<()> {
        let src = self.pop_slot()?;
        let dst = self.alloc_vid(result_ty)?;
        self.current_block_instrs.push(Instr::UnaryOp(dst, op, src));
        self.push_slot(dst);
        Ok(())
    }
}

// ================================================================================================
// Type/field helpers
// ================================================================================================

fn struct_field_count(module: &CompiledModule, idx: StructDefinitionIndex) -> usize {
    match &module.struct_defs[idx.0 as usize].field_information {
        StructFieldInformation::Declared(fields) => fields.len(),
        other => unreachable!("struct_field_count on {:?}", other),
    }
}

fn struct_field_type_toks(
    module: &CompiledModule,
    idx: StructDefinitionIndex,
) -> Vec<SignatureToken> {
    match &module.struct_defs[idx.0 as usize].field_information {
        StructFieldInformation::Declared(fields) => {
            fields.iter().map(|f| f.signature.0.clone()).collect()
        },
        other => unreachable!("struct_field_type_toks on {:?}", other),
    }
}

fn variant_field_count(
    module: &CompiledModule,
    struct_idx: StructDefinitionIndex,
    variant: VariantIndex,
) -> usize {
    match &module.struct_defs[struct_idx.0 as usize].field_information {
        StructFieldInformation::DeclaredVariants(variants) => {
            variants[variant as usize].fields.len()
        },
        other => unreachable!("variant_field_count on {:?}", other),
    }
}

fn variant_field_type_toks(
    module: &CompiledModule,
    struct_idx: StructDefinitionIndex,
    variant: VariantIndex,
) -> Vec<SignatureToken> {
    match &module.struct_defs[struct_idx.0 as usize].field_information {
        StructFieldInformation::DeclaredVariants(variants) => variants[variant as usize]
            .fields
            .iter()
            .map(|f| f.signature.0.clone())
            .collect(),
        other => unreachable!("variant_field_type_toks on {:?}", other),
    }
}

/// Substitute TypeParameter tokens with concrete types.
fn substitute_type_params(ty: &SignatureToken, params: &[SignatureToken]) -> SignatureToken {
    match ty {
        SignatureToken::TypeParameter(idx) => {
            params.get(*idx as usize).cloned().unwrap_or(ty.clone())
        },
        SignatureToken::Vector(inner) => {
            SignatureToken::Vector(Box::new(substitute_type_params(inner, params)))
        },
        SignatureToken::Reference(inner) => {
            SignatureToken::Reference(Box::new(substitute_type_params(inner, params)))
        },
        SignatureToken::MutableReference(inner) => {
            SignatureToken::MutableReference(Box::new(substitute_type_params(inner, params)))
        },
        SignatureToken::StructInstantiation(handle, tps) => {
            let new_tps: Vec<_> = tps
                .iter()
                .map(|p| substitute_type_params(p, params))
                .collect();
            SignatureToken::StructInstantiation(*handle, new_tps)
        },
        // [TODO] Function types with type parameters not yet substituted.
        _ => ty.clone(),
    }
}
