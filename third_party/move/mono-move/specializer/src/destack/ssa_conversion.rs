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
use crate::stackless_exec_ir::{BasicBlock, BinaryOp, Instr, Label, Slot, UnaryOp};
use anyhow::{bail, ensure, Context, Result};
use move_binary_format::{
    access::ModuleAccess,
    file_format::{
        Bytecode, CodeOffset, SignatureToken, StructDefInstantiationIndex, StructDefinitionIndex,
        StructFieldInformation, VariantIndex,
    },
    CompiledModule,
};
use move_vm_types::loaded_data::{runtime_types::Type, struct_name_indexing::StructNameIndex};
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
    /// Simulated operand stack with type information.
    stack: Vec<(Slot, Type)>,
    /// Types of all locals (params ++ declared locals).
    local_types: Vec<Type>,
    /// Types of value IDs, indexed directly by value ID number.
    vid_types: Vec<Type>,
    /// Struct name table for type conversion.
    struct_name_table: &'a [StructNameIndex],
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
    pub(crate) fn new(local_types: Vec<Type>, struct_name_table: &'a [StructNameIndex]) -> Self {
        Self {
            next_vid: 0,
            stack: Vec::new(),
            local_types,
            vid_types: Vec::new(),
            struct_name_table,
            blocks: Vec::new(),
            current_block_instrs: Vec::new(),
            current_block_label: None,
            label_map: UnorderedMap::new(),
            next_label: 0,
        }
    }

    fn alloc_vid(&mut self, ty: Type) -> Result<Slot> {
        let vid = Slot::Vid(self.next_vid);
        self.next_vid = self
            .next_vid
            .checked_add(1)
            .context("too many SSA values (Vid overflow)")?;
        self.vid_types.push(ty);
        Ok(vid)
    }

    fn push_typed_slot(&mut self, r: Slot, ty: Type) {
        self.stack.push((r, ty));
    }

    fn pop_typed_slot(&mut self) -> Result<(Slot, Type)> {
        self.stack.pop().context("stack underflow")
    }

    fn pop_n_reverse(&mut self, n: usize) -> Result<Vec<(Slot, Type)>> {
        ensure!(self.stack.len() >= n, "stack underflow");
        let start = self.stack.len() - n;
        Ok(self.stack.drain(start..).collect())
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
    // Type helpers: these will all be replaced by cached type representations.
    // --------------------------------------------------------------------------------------------

    fn struct_type(&self, module: &CompiledModule, idx: StructDefinitionIndex) -> Type {
        let def = &module.struct_defs[idx.0 as usize];
        let tok = SignatureToken::Struct(def.struct_handle);
        convert_sig_token(module, &tok, self.struct_name_table)
    }

    fn struct_inst_type(&self, module: &CompiledModule, idx: StructDefInstantiationIndex) -> Type {
        let inst = &module.struct_def_instantiations[idx.0 as usize];
        let def = &module.struct_defs[inst.def.0 as usize];
        let type_params = module.signature_at(inst.type_parameters).0.clone();
        let tok = SignatureToken::StructInstantiation(def.struct_handle, type_params);
        convert_sig_token(module, &tok, self.struct_name_table)
    }

    fn field_type(
        &self,
        module: &CompiledModule,
        idx: move_binary_format::file_format::FieldHandleIndex,
    ) -> Result<Type> {
        let handle = &module.field_handles[idx.0 as usize];
        let tok = match &module.struct_defs[handle.owner.0 as usize].field_information {
            StructFieldInformation::Declared(fields) => {
                fields[handle.field as usize].signature.0.clone()
            },
            _ => bail!("field access on native/variant struct"),
        };
        Ok(convert_sig_token(module, &tok, self.struct_name_table))
    }

    fn field_inst_type(
        &self,
        module: &CompiledModule,
        idx: move_binary_format::file_format::FieldInstantiationIndex,
    ) -> Result<Type> {
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
        Ok(convert_sig_token(module, &tok, self.struct_name_table))
    }

    fn variant_field_handle_type(
        &self,
        module: &CompiledModule,
        idx: move_binary_format::file_format::VariantFieldHandleIndex,
    ) -> Result<Type> {
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
        Ok(convert_sig_token(module, &tok, self.struct_name_table))
    }

    fn variant_field_inst_type(
        &self,
        module: &CompiledModule,
        idx: move_binary_format::file_format::VariantFieldInstantiationIndex,
    ) -> Result<Type> {
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
        Ok(convert_sig_token(module, &tok, self.struct_name_table))
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
    /// [TODO] check if we need to have types in the simulated stack
    fn convert_bytecode(&mut self, module: &CompiledModule, bc: &Bytecode) -> Result<()> {
        use Bytecode as B;
        match bc {
            // --- Loads ---
            B::LdU8(v) => {
                let ty = Type::U8;
                let dst = self.alloc_vid(ty.clone())?;
                self.current_block_instrs.push(Instr::LdU8(dst, *v));
                self.push_typed_slot(dst, ty);
            },
            B::LdU16(v) => {
                let ty = Type::U16;
                let dst = self.alloc_vid(ty.clone())?;
                self.current_block_instrs.push(Instr::LdU16(dst, *v));
                self.push_typed_slot(dst, ty);
            },
            B::LdU32(v) => {
                let ty = Type::U32;
                let dst = self.alloc_vid(ty.clone())?;
                self.current_block_instrs.push(Instr::LdU32(dst, *v));
                self.push_typed_slot(dst, ty);
            },
            B::LdU64(v) => {
                let ty = Type::U64;
                let dst = self.alloc_vid(ty.clone())?;
                self.current_block_instrs.push(Instr::LdU64(dst, *v));
                self.push_typed_slot(dst, ty);
            },
            B::LdU128(v) => {
                let ty = Type::U128;
                let dst = self.alloc_vid(ty.clone())?;
                self.current_block_instrs.push(Instr::LdU128(dst, *v));
                self.push_typed_slot(dst, ty);
            },
            B::LdU256(v) => {
                let ty = Type::U256;
                let dst = self.alloc_vid(ty.clone())?;
                self.current_block_instrs.push(Instr::LdU256(dst, *v));
                self.push_typed_slot(dst, ty);
            },
            B::LdI8(v) => {
                let ty = Type::I8;
                let dst = self.alloc_vid(ty.clone())?;
                self.current_block_instrs.push(Instr::LdI8(dst, *v));
                self.push_typed_slot(dst, ty);
            },
            B::LdI16(v) => {
                let ty = Type::I16;
                let dst = self.alloc_vid(ty.clone())?;
                self.current_block_instrs.push(Instr::LdI16(dst, *v));
                self.push_typed_slot(dst, ty);
            },
            B::LdI32(v) => {
                let ty = Type::I32;
                let dst = self.alloc_vid(ty.clone())?;
                self.current_block_instrs.push(Instr::LdI32(dst, *v));
                self.push_typed_slot(dst, ty);
            },
            B::LdI64(v) => {
                let ty = Type::I64;
                let dst = self.alloc_vid(ty.clone())?;
                self.current_block_instrs.push(Instr::LdI64(dst, *v));
                self.push_typed_slot(dst, ty);
            },
            B::LdI128(v) => {
                let ty = Type::I128;
                let dst = self.alloc_vid(ty.clone())?;
                self.current_block_instrs.push(Instr::LdI128(dst, *v));
                self.push_typed_slot(dst, ty);
            },
            B::LdI256(v) => {
                let ty = Type::I256;
                let dst = self.alloc_vid(ty.clone())?;
                self.current_block_instrs.push(Instr::LdI256(dst, *v));
                self.push_typed_slot(dst, ty);
            },
            B::LdConst(idx) => {
                let tok = &module.constant_pool[idx.0 as usize].type_;
                let ty = convert_sig_token(module, tok, self.struct_name_table);
                let dst = self.alloc_vid(ty.clone())?;
                self.current_block_instrs.push(Instr::LdConst(dst, *idx));
                self.push_typed_slot(dst, ty);
            },
            B::LdTrue => {
                let ty = Type::Bool;
                let dst = self.alloc_vid(ty.clone())?;
                self.current_block_instrs.push(Instr::LdTrue(dst));
                self.push_typed_slot(dst, ty);
            },
            B::LdFalse => {
                let ty = Type::Bool;
                let dst = self.alloc_vid(ty.clone())?;
                self.current_block_instrs.push(Instr::LdFalse(dst));
                self.push_typed_slot(dst, ty);
            },

            // --- Locals ---
            B::CopyLoc(idx) => {
                let src = Slot::Home(*idx as u16);
                let ty = self.local_types[*idx as usize].clone();
                let dst = self.alloc_vid(ty.clone())?;
                self.current_block_instrs.push(Instr::Copy(dst, src));
                self.push_typed_slot(dst, ty);
            },
            B::MoveLoc(idx) => {
                let src = Slot::Home(*idx as u16);
                let ty = self.local_types[*idx as usize].clone();
                let dst = self.alloc_vid(ty.clone())?;
                self.current_block_instrs.push(Instr::Move(dst, src));
                self.push_typed_slot(dst, ty);
            },
            B::StLoc(idx) => {
                let (src, _ty) = self.pop_typed_slot()?;
                let dst = Slot::Home(*idx as u16);
                self.current_block_instrs.push(Instr::Move(dst, src));
            },

            // --- Pop ---
            B::Pop => {
                let _ = self.pop_typed_slot()?;
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
            B::Lt => self.convert_binop(BinaryOp::Lt, true)?,
            B::Gt => self.convert_binop(BinaryOp::Gt, true)?,
            B::Le => self.convert_binop(BinaryOp::Le, true)?,
            B::Ge => self.convert_binop(BinaryOp::Ge, true)?,
            B::Eq => self.convert_binop(BinaryOp::Eq, true)?,
            B::Neq => self.convert_binop(BinaryOp::Neq, true)?,
            B::Or => self.convert_binop(BinaryOp::Or, true)?,
            B::And => self.convert_binop(BinaryOp::And, true)?,

            // --- Unary ops (result type specified) ---
            B::CastU8 => self.convert_unop(UnaryOp::CastU8, Some(Type::U8))?,
            B::CastU16 => self.convert_unop(UnaryOp::CastU16, Some(Type::U16))?,
            B::CastU32 => self.convert_unop(UnaryOp::CastU32, Some(Type::U32))?,
            B::CastU64 => self.convert_unop(UnaryOp::CastU64, Some(Type::U64))?,
            B::CastU128 => self.convert_unop(UnaryOp::CastU128, Some(Type::U128))?,
            B::CastU256 => self.convert_unop(UnaryOp::CastU256, Some(Type::U256))?,
            B::CastI8 => self.convert_unop(UnaryOp::CastI8, Some(Type::I8))?,
            B::CastI16 => self.convert_unop(UnaryOp::CastI16, Some(Type::I16))?,
            B::CastI32 => self.convert_unop(UnaryOp::CastI32, Some(Type::I32))?,
            B::CastI64 => self.convert_unop(UnaryOp::CastI64, Some(Type::I64))?,
            B::CastI128 => self.convert_unop(UnaryOp::CastI128, Some(Type::I128))?,
            B::CastI256 => self.convert_unop(UnaryOp::CastI256, Some(Type::I256))?,
            B::Not => self.convert_unop(UnaryOp::Not, Some(Type::Bool))?,
            // --- Unary ops (result type derived from operand) ---
            B::Negate => self.convert_unop(UnaryOp::Negate, None)?,
            B::FreezeRef => self.convert_unop(UnaryOp::FreezeRef, None)?,

            // --- Struct ops ---
            B::Pack(idx) => {
                let n = struct_field_count(module, *idx);
                let fields_typed = self.pop_n_reverse(n)?;
                let fields: Vec<Slot> = fields_typed.iter().map(|(r, _)| *r).collect();
                let result_ty = self.struct_type(module, *idx);
                let dst = self.alloc_vid(result_ty.clone())?;
                self.current_block_instrs
                    .push(Instr::Pack(dst, *idx, fields));
                self.push_typed_slot(dst, result_ty);
            },
            B::PackGeneric(idx) => {
                let inst = &module.struct_def_instantiations[idx.0 as usize];
                let n = struct_field_count(module, inst.def);
                let fields_typed = self.pop_n_reverse(n)?;
                let fields: Vec<Slot> = fields_typed.iter().map(|(r, _)| *r).collect();
                let result_ty = self.struct_inst_type(module, *idx);
                let dst = self.alloc_vid(result_ty.clone())?;
                self.current_block_instrs
                    .push(Instr::PackGeneric(dst, *idx, fields));
                self.push_typed_slot(dst, result_ty);
            },
            B::Unpack(idx) => {
                let (src, _src_ty) = self.pop_typed_slot()?;
                let n = struct_field_count(module, *idx);
                let ftypes = struct_field_type_toks(module, *idx);
                let ftypes: Vec<Type> = convert_sig_tokens(module, &ftypes, self.struct_name_table);
                let mut dsts = Vec::with_capacity(n);
                for i in 0..n {
                    let fty = ftypes
                        .get(i)
                        .cloned()
                        .context("field type index out of bounds")?;
                    dsts.push(self.alloc_vid(fty)?);
                }
                self.current_block_instrs
                    .push(Instr::Unpack(dsts.clone(), *idx, src));
                for (dst, fty) in dsts.into_iter().zip(ftypes) {
                    self.push_typed_slot(dst, fty);
                }
            },
            B::UnpackGeneric(idx) => {
                let (src, _src_ty) = self.pop_typed_slot()?;
                let inst = &module.struct_def_instantiations[idx.0 as usize];
                let n = struct_field_count(module, inst.def);
                let type_params = module.signature_at(inst.type_parameters).0.clone();
                let raw_ftypes = struct_field_type_toks(module, inst.def);
                let ftypes_tok: Vec<SignatureToken> = raw_ftypes
                    .iter()
                    .map(|ft| substitute_type_params(ft, &type_params))
                    .collect();
                let ftypes: Vec<Type> =
                    convert_sig_tokens(module, &ftypes_tok, self.struct_name_table);
                let mut dsts = Vec::with_capacity(n);
                for i in 0..n {
                    let fty = ftypes
                        .get(i)
                        .cloned()
                        .context("field type index out of bounds")?;
                    dsts.push(self.alloc_vid(fty)?);
                }
                self.current_block_instrs
                    .push(Instr::UnpackGeneric(dsts.clone(), *idx, src));
                for (dst, fty) in dsts.into_iter().zip(ftypes) {
                    self.push_typed_slot(dst, fty);
                }
            },

            // --- Variant ops ---
            B::PackVariant(idx) => {
                let handle = &module.struct_variant_handles[idx.0 as usize];
                let n = variant_field_count(module, handle.struct_index, handle.variant);
                let fields_typed = self.pop_n_reverse(n)?;
                let fields: Vec<Slot> = fields_typed.iter().map(|(r, _)| *r).collect();
                let result_ty = self.struct_type(module, handle.struct_index);
                let dst = self.alloc_vid(result_ty.clone())?;
                self.current_block_instrs
                    .push(Instr::PackVariant(dst, *idx, fields));
                self.push_typed_slot(dst, result_ty);
            },
            B::PackVariantGeneric(idx) => {
                let inst = &module.struct_variant_instantiations[idx.0 as usize];
                let handle = &module.struct_variant_handles[inst.handle.0 as usize];
                let n = variant_field_count(module, handle.struct_index, handle.variant);
                let fields_typed = self.pop_n_reverse(n)?;
                let fields: Vec<Slot> = fields_typed.iter().map(|(r, _)| *r).collect();
                let type_params = module.signature_at(inst.type_parameters).0.clone();
                let def = &module.struct_defs[handle.struct_index.0 as usize];
                let tok = SignatureToken::StructInstantiation(def.struct_handle, type_params);
                let result_ty = convert_sig_token(module, &tok, self.struct_name_table);
                let dst = self.alloc_vid(result_ty.clone())?;
                self.current_block_instrs
                    .push(Instr::PackVariantGeneric(dst, *idx, fields));
                self.push_typed_slot(dst, result_ty);
            },
            B::UnpackVariant(idx) => {
                let (src, _src_ty) = self.pop_typed_slot()?;
                let handle = &module.struct_variant_handles[idx.0 as usize];
                let n = variant_field_count(module, handle.struct_index, handle.variant);
                let ftypes_tok =
                    variant_field_type_toks(module, handle.struct_index, handle.variant);
                let ftypes: Vec<Type> =
                    convert_sig_tokens(module, &ftypes_tok, self.struct_name_table);
                let mut dsts = Vec::with_capacity(n);
                for i in 0..n {
                    let fty = ftypes
                        .get(i)
                        .cloned()
                        .context("field type index out of bounds")?;
                    dsts.push(self.alloc_vid(fty)?);
                }
                self.current_block_instrs
                    .push(Instr::UnpackVariant(dsts.clone(), *idx, src));
                for (dst, fty) in dsts.into_iter().zip(ftypes) {
                    self.push_typed_slot(dst, fty);
                }
            },
            B::UnpackVariantGeneric(idx) => {
                let (src, _src_ty) = self.pop_typed_slot()?;
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
                let ftypes: Vec<Type> =
                    convert_sig_tokens(module, &ftypes_tok, self.struct_name_table);
                let mut dsts = Vec::with_capacity(n);
                for i in 0..n {
                    let fty = ftypes
                        .get(i)
                        .cloned()
                        .context("field type index out of bounds")?;
                    dsts.push(self.alloc_vid(fty)?);
                }
                self.current_block_instrs.push(Instr::UnpackVariantGeneric(
                    dsts.clone(),
                    *idx,
                    src,
                ));
                for (dst, fty) in dsts.into_iter().zip(ftypes) {
                    self.push_typed_slot(dst, fty);
                }
            },
            B::TestVariant(idx) => {
                let (src, _src_ty) = self.pop_typed_slot()?;
                let ty = Type::Bool;
                let dst = self.alloc_vid(ty.clone())?;
                self.current_block_instrs
                    .push(Instr::TestVariant(dst, *idx, src));
                self.push_typed_slot(dst, ty);
            },
            B::TestVariantGeneric(idx) => {
                let (src, _src_ty) = self.pop_typed_slot()?;
                let ty = Type::Bool;
                let dst = self.alloc_vid(ty.clone())?;
                self.current_block_instrs
                    .push(Instr::TestVariantGeneric(dst, *idx, src));
                self.push_typed_slot(dst, ty);
            },

            // --- References ---
            B::ImmBorrowLoc(idx) => {
                let src = Slot::Home(*idx as u16);
                let inner = self.local_types[*idx as usize].clone();
                let ty = Type::Reference(Box::new(inner));
                let dst = self.alloc_vid(ty.clone())?;
                self.current_block_instrs
                    .push(Instr::ImmBorrowLoc(dst, src));
                self.push_typed_slot(dst, ty);
            },
            B::MutBorrowLoc(idx) => {
                let src = Slot::Home(*idx as u16);
                let inner = self.local_types[*idx as usize].clone();
                let ty = Type::MutableReference(Box::new(inner));
                let dst = self.alloc_vid(ty.clone())?;
                self.current_block_instrs
                    .push(Instr::MutBorrowLoc(dst, src));
                self.push_typed_slot(dst, ty);
            },
            B::ImmBorrowField(idx) => {
                let (src, _src_ty) = self.pop_typed_slot()?;
                let fty = self.field_type(module, *idx)?;
                let ty = Type::Reference(Box::new(fty));
                let dst = self.alloc_vid(ty.clone())?;
                self.current_block_instrs
                    .push(Instr::ImmBorrowField(dst, *idx, src));
                self.push_typed_slot(dst, ty);
            },
            B::MutBorrowField(idx) => {
                let (src, _src_ty) = self.pop_typed_slot()?;
                let fty = self.field_type(module, *idx)?;
                let ty = Type::MutableReference(Box::new(fty));
                let dst = self.alloc_vid(ty.clone())?;
                self.current_block_instrs
                    .push(Instr::MutBorrowField(dst, *idx, src));
                self.push_typed_slot(dst, ty);
            },
            B::ImmBorrowFieldGeneric(idx) => {
                let (src, _src_ty) = self.pop_typed_slot()?;
                let fty = self.field_inst_type(module, *idx)?;
                let ty = Type::Reference(Box::new(fty));
                let dst = self.alloc_vid(ty.clone())?;
                self.current_block_instrs
                    .push(Instr::ImmBorrowFieldGeneric(dst, *idx, src));
                self.push_typed_slot(dst, ty);
            },
            B::MutBorrowFieldGeneric(idx) => {
                let (src, _src_ty) = self.pop_typed_slot()?;
                let fty = self.field_inst_type(module, *idx)?;
                let ty = Type::MutableReference(Box::new(fty));
                let dst = self.alloc_vid(ty.clone())?;
                self.current_block_instrs
                    .push(Instr::MutBorrowFieldGeneric(dst, *idx, src));
                self.push_typed_slot(dst, ty);
            },
            B::ImmBorrowVariantField(idx) => {
                let (src, _src_ty) = self.pop_typed_slot()?;
                let fty = self.variant_field_handle_type(module, *idx)?;
                let ty = Type::Reference(Box::new(fty));
                let dst = self.alloc_vid(ty.clone())?;
                self.current_block_instrs
                    .push(Instr::ImmBorrowVariantField(dst, *idx, src));
                self.push_typed_slot(dst, ty);
            },
            B::MutBorrowVariantField(idx) => {
                let (src, _src_ty) = self.pop_typed_slot()?;
                let fty = self.variant_field_handle_type(module, *idx)?;
                let ty = Type::MutableReference(Box::new(fty));
                let dst = self.alloc_vid(ty.clone())?;
                self.current_block_instrs
                    .push(Instr::MutBorrowVariantField(dst, *idx, src));
                self.push_typed_slot(dst, ty);
            },
            B::ImmBorrowVariantFieldGeneric(idx) => {
                let (src, _src_ty) = self.pop_typed_slot()?;
                let fty = self.variant_field_inst_type(module, *idx)?;
                let ty = Type::Reference(Box::new(fty));
                let dst = self.alloc_vid(ty.clone())?;
                self.current_block_instrs
                    .push(Instr::ImmBorrowVariantFieldGeneric(dst, *idx, src));
                self.push_typed_slot(dst, ty);
            },
            B::MutBorrowVariantFieldGeneric(idx) => {
                let (src, _src_ty) = self.pop_typed_slot()?;
                let fty = self.variant_field_inst_type(module, *idx)?;
                let ty = Type::MutableReference(Box::new(fty));
                let dst = self.alloc_vid(ty.clone())?;
                self.current_block_instrs
                    .push(Instr::MutBorrowVariantFieldGeneric(dst, *idx, src));
                self.push_typed_slot(dst, ty);
            },
            B::ReadRef => {
                let (src, src_ty) = self.pop_typed_slot()?;
                let ty = match src_ty {
                    Type::Reference(inner) | Type::MutableReference(inner) => (*inner).clone(),
                    other => bail!("ReadRef on non-reference type {:?}", other),
                };
                let dst = self.alloc_vid(ty.clone())?;
                self.current_block_instrs.push(Instr::ReadRef(dst, src));
                self.push_typed_slot(dst, ty);
            },
            B::WriteRef => {
                let (ref_r, _ref_ty) = self.pop_typed_slot()?;
                let (val, _val_ty) = self.pop_typed_slot()?;
                self.current_block_instrs.push(Instr::WriteRef(ref_r, val));
            },

            // --- Globals ---
            B::Exists(idx) => {
                let (addr, _addr_ty) = self.pop_typed_slot()?;
                let ty = Type::Bool;
                let dst = self.alloc_vid(ty.clone())?;
                self.current_block_instrs
                    .push(Instr::Exists(dst, *idx, addr));
                self.push_typed_slot(dst, ty);
            },
            B::ExistsGeneric(idx) => {
                let (addr, _addr_ty) = self.pop_typed_slot()?;
                let ty = Type::Bool;
                let dst = self.alloc_vid(ty.clone())?;
                self.current_block_instrs
                    .push(Instr::ExistsGeneric(dst, *idx, addr));
                self.push_typed_slot(dst, ty);
            },
            B::MoveFrom(idx) => {
                let (addr, _addr_ty) = self.pop_typed_slot()?;
                let ty = self.struct_type(module, *idx);
                let dst = self.alloc_vid(ty.clone())?;
                self.current_block_instrs
                    .push(Instr::MoveFrom(dst, *idx, addr));
                self.push_typed_slot(dst, ty);
            },
            B::MoveFromGeneric(idx) => {
                let (addr, _addr_ty) = self.pop_typed_slot()?;
                let ty = self.struct_inst_type(module, *idx);
                let dst = self.alloc_vid(ty.clone())?;
                self.current_block_instrs
                    .push(Instr::MoveFromGeneric(dst, *idx, addr));
                self.push_typed_slot(dst, ty);
            },
            B::MoveTo(idx) => {
                let (val, _val_ty) = self.pop_typed_slot()?;
                let (signer, _signer_ty) = self.pop_typed_slot()?;
                self.current_block_instrs
                    .push(Instr::MoveTo(*idx, signer, val));
            },
            B::MoveToGeneric(idx) => {
                let (val, _val_ty) = self.pop_typed_slot()?;
                let (signer, _signer_ty) = self.pop_typed_slot()?;
                self.current_block_instrs
                    .push(Instr::MoveToGeneric(*idx, signer, val));
            },
            B::ImmBorrowGlobal(idx) => {
                let (addr, _addr_ty) = self.pop_typed_slot()?;
                let ty = Type::Reference(Box::new(self.struct_type(module, *idx)));
                let dst = self.alloc_vid(ty.clone())?;
                self.current_block_instrs
                    .push(Instr::ImmBorrowGlobal(dst, *idx, addr));
                self.push_typed_slot(dst, ty);
            },
            B::ImmBorrowGlobalGeneric(idx) => {
                let (addr, _addr_ty) = self.pop_typed_slot()?;
                let ty = Type::Reference(Box::new(self.struct_inst_type(module, *idx)));
                let dst = self.alloc_vid(ty.clone())?;
                self.current_block_instrs
                    .push(Instr::ImmBorrowGlobalGeneric(dst, *idx, addr));
                self.push_typed_slot(dst, ty);
            },
            B::MutBorrowGlobal(idx) => {
                let (addr, _addr_ty) = self.pop_typed_slot()?;
                let ty = Type::MutableReference(Box::new(self.struct_type(module, *idx)));
                let dst = self.alloc_vid(ty.clone())?;
                self.current_block_instrs
                    .push(Instr::MutBorrowGlobal(dst, *idx, addr));
                self.push_typed_slot(dst, ty);
            },
            B::MutBorrowGlobalGeneric(idx) => {
                let (addr, _addr_ty) = self.pop_typed_slot()?;
                let ty = Type::MutableReference(Box::new(self.struct_inst_type(module, *idx)));
                let dst = self.alloc_vid(ty.clone())?;
                self.current_block_instrs
                    .push(Instr::MutBorrowGlobalGeneric(dst, *idx, addr));
                self.push_typed_slot(dst, ty);
            },

            // --- Calls ---
            B::Call(idx) => {
                let handle = module.function_handle_at(*idx);
                let num_args = module.signature_at(handle.parameters).0.len();
                let ret_toks = &module.signature_at(handle.return_).0;
                let ret_types = convert_sig_tokens(module, ret_toks, self.struct_name_table);
                let args_typed = self.pop_n_reverse(num_args)?;
                let args: Vec<Slot> = args_typed.iter().map(|(r, _)| *r).collect();
                let mut rets = Vec::with_capacity(ret_types.len());
                for rty in &ret_types {
                    rets.push(self.alloc_vid(rty.clone())?);
                }
                self.current_block_instrs
                    .push(Instr::Call(rets.clone(), *idx, args));
                for (r, rty) in rets.into_iter().zip(ret_types) {
                    self.push_typed_slot(r, rty);
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
                let ret_types = convert_sig_tokens(module, &ret_toks, self.struct_name_table);
                let args_typed = self.pop_n_reverse(num_args)?;
                let args: Vec<Slot> = args_typed.iter().map(|(r, _)| *r).collect();
                let mut rets = Vec::with_capacity(ret_types.len());
                for rty in &ret_types {
                    rets.push(self.alloc_vid(rty.clone())?);
                }
                self.current_block_instrs
                    .push(Instr::CallGeneric(rets.clone(), *idx, args));
                for (r, rty) in rets.into_iter().zip(ret_types) {
                    self.push_typed_slot(r, rty);
                }
            },

            // --- Closures ---
            B::PackClosure(fhi, mask) => {
                let captured_count = mask.captured_count() as usize;
                let captured_typed = self.pop_n_reverse(captured_count)?;
                let captured: Vec<Slot> = captured_typed.iter().map(|(r, _)| *r).collect();
                let handle = module.function_handle_at(*fhi);
                let params = &module.signature_at(handle.parameters).0;
                let returns = &module.signature_at(handle.return_).0;
                let tok = SignatureToken::Function(
                    params.clone(),
                    returns.clone(),
                    move_core_types::ability::AbilitySet::EMPTY,
                );
                let ty = convert_sig_token(module, &tok, self.struct_name_table);
                let dst = self.alloc_vid(ty.clone())?;
                self.current_block_instrs
                    .push(Instr::PackClosure(dst, *fhi, *mask, captured));
                self.push_typed_slot(dst, ty);
            },
            B::PackClosureGeneric(fii, mask) => {
                let captured_count = mask.captured_count() as usize;
                let captured_typed = self.pop_n_reverse(captured_count)?;
                let captured: Vec<Slot> = captured_typed.iter().map(|(r, _)| *r).collect();
                let inst = &module.function_instantiations[fii.0 as usize];
                let handle = module.function_handle_at(inst.handle);
                let params = &module.signature_at(handle.parameters).0;
                let returns = &module.signature_at(handle.return_).0;
                let tok = SignatureToken::Function(
                    params.clone(),
                    returns.clone(),
                    move_core_types::ability::AbilitySet::EMPTY,
                );
                let ty = convert_sig_token(module, &tok, self.struct_name_table);
                let dst = self.alloc_vid(ty.clone())?;
                self.current_block_instrs
                    .push(Instr::PackClosureGeneric(dst, *fii, *mask, captured));
                self.push_typed_slot(dst, ty);
            },
            B::CallClosure(sig_idx) => {
                let sig = module.signature_at(*sig_idx);
                let (num_args, ret_toks) =
                    if let Some(SignatureToken::Function(params, results, _)) = sig.0.first() {
                        (params.len(), results.clone())
                    } else {
                        bail!("CallClosure signature must start with a Function token")
                    };
                let ret_types = convert_sig_tokens(module, &ret_toks, self.struct_name_table);
                let (closure, _closure_ty) = self.pop_typed_slot()?;
                let args_typed = self.pop_n_reverse(num_args)?;
                let mut all_args: Vec<Slot> = args_typed.iter().map(|(r, _)| *r).collect();
                all_args.push(closure);
                let mut rets = Vec::with_capacity(ret_types.len());
                for rty in &ret_types {
                    rets.push(self.alloc_vid(rty.clone())?);
                }
                self.current_block_instrs.push(Instr::CallClosure(
                    rets.clone(),
                    *sig_idx,
                    all_args,
                ));
                for (r, rty) in rets.into_iter().zip(ret_types) {
                    self.push_typed_slot(r, rty);
                }
            },

            // --- Vector ops ---
            B::VecPack(sig_idx, count) => {
                let count = *count as u16;
                let elems_typed = self.pop_n_reverse(count as usize)?;
                let elems: Vec<Slot> = elems_typed.iter().map(|(r, _)| *r).collect();
                let elem_tok = &module.signature_at(*sig_idx).0[0];
                let elem_ty = convert_sig_token(module, elem_tok, self.struct_name_table);
                let ty = Type::Vector(triomphe::Arc::new(elem_ty));
                let dst = self.alloc_vid(ty.clone())?;
                self.current_block_instrs
                    .push(Instr::VecPack(dst, *sig_idx, count, elems));
                self.push_typed_slot(dst, ty);
            },
            B::VecLen(sig_idx) => {
                let (vec_ref, _vec_ty) = self.pop_typed_slot()?;
                let ty = Type::U64;
                let dst = self.alloc_vid(ty.clone())?;
                self.current_block_instrs
                    .push(Instr::VecLen(dst, *sig_idx, vec_ref));
                self.push_typed_slot(dst, ty);
            },
            B::VecImmBorrow(sig_idx) => {
                let (idx_r, _idx_ty) = self.pop_typed_slot()?;
                let (vec_ref, _vec_ty) = self.pop_typed_slot()?;
                let elem_tok = &module.signature_at(*sig_idx).0[0];
                let elem_ty = convert_sig_token(module, elem_tok, self.struct_name_table);
                let ty = Type::Reference(Box::new(elem_ty));
                let dst = self.alloc_vid(ty.clone())?;
                self.current_block_instrs
                    .push(Instr::VecImmBorrow(dst, *sig_idx, vec_ref, idx_r));
                self.push_typed_slot(dst, ty);
            },
            B::VecMutBorrow(sig_idx) => {
                let (idx_r, _idx_ty) = self.pop_typed_slot()?;
                let (vec_ref, _vec_ty) = self.pop_typed_slot()?;
                let elem_tok = &module.signature_at(*sig_idx).0[0];
                let elem_ty = convert_sig_token(module, elem_tok, self.struct_name_table);
                let ty = Type::MutableReference(Box::new(elem_ty));
                let dst = self.alloc_vid(ty.clone())?;
                self.current_block_instrs
                    .push(Instr::VecMutBorrow(dst, *sig_idx, vec_ref, idx_r));
                self.push_typed_slot(dst, ty);
            },
            B::VecPushBack(sig_idx) => {
                let (val, _val_ty) = self.pop_typed_slot()?;
                let (vec_ref, _vec_ty) = self.pop_typed_slot()?;
                self.current_block_instrs
                    .push(Instr::VecPushBack(*sig_idx, vec_ref, val));
            },
            B::VecPopBack(sig_idx) => {
                let (vec_ref, _vec_ty) = self.pop_typed_slot()?;
                let elem_tok = &module.signature_at(*sig_idx).0[0];
                let ty = convert_sig_token(module, elem_tok, self.struct_name_table);
                let dst = self.alloc_vid(ty.clone())?;
                self.current_block_instrs
                    .push(Instr::VecPopBack(dst, *sig_idx, vec_ref));
                self.push_typed_slot(dst, ty);
            },
            B::VecUnpack(sig_idx, count) => {
                let count = *count as u16;
                let (src, _src_ty) = self.pop_typed_slot()?;
                let elem_tok = &module.signature_at(*sig_idx).0[0];
                let elem_ty = convert_sig_token(module, elem_tok, self.struct_name_table);
                let mut dsts = Vec::with_capacity(count as usize);
                for _ in 0..count {
                    dsts.push(self.alloc_vid(elem_ty.clone())?);
                }
                self.current_block_instrs.push(Instr::VecUnpack(
                    dsts.clone(),
                    *sig_idx,
                    count,
                    src,
                ));
                for dst in dsts {
                    self.push_typed_slot(dst, elem_ty.clone());
                }
            },
            B::VecSwap(sig_idx) => {
                let (j, _j_ty) = self.pop_typed_slot()?;
                let (i, _i_ty) = self.pop_typed_slot()?;
                let (vec_ref, _vec_ty) = self.pop_typed_slot()?;
                self.current_block_instrs
                    .push(Instr::VecSwap(*sig_idx, vec_ref, i, j));
            },

            // --- Control flow ---
            B::Branch(target) => {
                let label = *self.label_map.get(target).expect("branch target label");
                self.current_block_instrs.push(Instr::Branch(label));
            },
            B::BrTrue(target) => {
                let (cond, _cond_ty) = self.pop_typed_slot()?;
                let label = *self.label_map.get(target).expect("branch target label");
                self.current_block_instrs.push(Instr::BrTrue(label, cond));
            },
            B::BrFalse(target) => {
                let (cond, _cond_ty) = self.pop_typed_slot()?;
                let label = *self.label_map.get(target).expect("branch target label");
                self.current_block_instrs.push(Instr::BrFalse(label, cond));
            },
            B::Ret => {
                let rets: Vec<Slot> = self.stack.drain(..).map(|(r, _)| r).collect();
                self.current_block_instrs.push(Instr::Ret(rets));
            },
            B::Abort => {
                let (code, _code_ty) = self.pop_typed_slot()?;
                self.current_block_instrs.push(Instr::Abort(code));
            },
            B::AbortMsg => {
                let (msg, _msg_ty) = self.pop_typed_slot()?;
                let (code, _code_ty) = self.pop_typed_slot()?;
                self.current_block_instrs.push(Instr::AbortMsg(code, msg));
            },

            B::Nop => {},
        }
        Ok(())
    }

    fn convert_binop(&mut self, op: BinaryOp, result_is_bool: bool) -> Result<()> {
        let (rhs, _rhs_ty) = self.pop_typed_slot()?;
        let (lhs, lhs_ty) = self.pop_typed_slot()?;
        let result_ty = if result_is_bool { Type::Bool } else { lhs_ty };
        let dst = self.alloc_vid(result_ty.clone())?;
        self.current_block_instrs
            .push(Instr::BinaryOp(dst, op, lhs, rhs));
        self.push_typed_slot(dst, result_ty);
        Ok(())
    }

    /// If `result_ty` is `Some`, use it directly. If `None`, derive from the operand type:
    /// `Negate` preserves the type, `FreezeRef` converts `&mut T` → `&T`.
    fn convert_unop(&mut self, op: UnaryOp, result_ty: Option<Type>) -> Result<()> {
        let (src, src_ty) = self.pop_typed_slot()?;
        let result_ty = match result_ty {
            Some(ty) => ty,
            None => match (&op, src_ty) {
                (UnaryOp::FreezeRef, Type::MutableReference(inner)) => Type::Reference(inner),
                (UnaryOp::Negate | UnaryOp::FreezeRef, ty) => ty,
                _ => bail!("unary op {:?} requires an explicit result type", op),
            },
        };
        let dst = self.alloc_vid(result_ty.clone())?;
        self.current_block_instrs.push(Instr::UnaryOp(dst, op, src));
        self.push_typed_slot(dst, result_ty);
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
