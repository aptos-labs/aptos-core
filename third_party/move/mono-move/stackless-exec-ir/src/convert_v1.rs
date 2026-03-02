// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Pass 1: Convert stack-based Move bytecode to stackless register-based IR
//! via stack simulation with type-aware register recycling and
//! destination-driven allocation.

use crate::{
    ir::{BinaryOp, FunctionIR, Instr, Label, ModuleIR, Reg, UnaryOp},
    type_conversion::{convert_sig_token, convert_sig_tokens},
};
use move_binary_format::{
    access::ModuleAccess,
    file_format::{
        Bytecode, CodeOffset, FieldHandleIndex, FieldInstantiationIndex, FunctionDefinition,
        SignatureToken, StructDefInstantiationIndex, StructDefinitionIndex, StructFieldInformation,
    },
    CompiledModule,
};
use move_vm_types::loaded_data::{runtime_types::Type, struct_name_indexing::StructNameIndex};
use std::collections::BTreeMap;

/// Convert an entire compiled module to stackless IR.
///
/// The caller is responsible for running the bytecode verifier beforehand
/// if the module comes from an untrusted source. The conversion relies on
/// the following verifier-guaranteed invariants:
///
/// - **Stack balance**: every pop has a matching push; the stack is empty at
///   basic-block boundaries and `Ret` drains exactly the declared return values.
/// - **Type consistency**: operand types on the stack match what each instruction
///   expects (e.g. `ReadRef` sees a reference, arithmetic operands are the same
///   integer type, `FreezeRef` sees a `&mut`).
/// - **Index bounds**: all pool indices (`StructDefinitionIndex`,
///   `FieldHandleIndex`, `FunctionHandleIndex`, `ConstantPoolIndex`,
///   `SignatureIndex`, variant indices, etc.) are within their respective tables.
/// - **Struct/variant field shape**: `Pack`/`Unpack` target structs with
///   `Declared` fields; variant instructions target `DeclaredVariants` with
///   valid variant and field indices.
/// - **Branch target validity**: every branch offset maps to a valid bytecode
///   position inside the function.
/// - **Local initialization**: locals are assigned via `StLoc` before any
///   `CopyLoc`/`MoveLoc`; `MoveLoc` is not used on an already-moved local.
/// - **Function signature correctness**: the number of arguments on the stack
///   matches the callee's declared parameter count, and return-type signatures
///   are well-formed.
/// - **Type parameter bounds**: `TypeParameter(idx)` indices fall within the
///   type-parameter list of the enclosing generic context.
/// - **Reference safety**: the borrow checker guarantees that freed registers
///   truly hold dead values, so type-keyed register recycling is sound.
pub fn convert_module_v1(module: CompiledModule, struct_name_table: &[StructNameIndex]) -> ModuleIR {
    let functions = module
        .function_defs
        .iter()
        .filter_map(|fdef| {
            fdef.code.as_ref().map(|code| {
                let handle = module.function_handle_at(fdef.function);
                let num_params = module.signature_at(handle.parameters).0.len() as Reg;
                let num_locals = module.signature_at(code.locals).0.len() as Reg;
                let name_idx = handle.name;
                let handle_idx = fdef.function;

                let param_toks = &module.signature_at(handle.parameters).0;
                let local_toks = &module.signature_at(code.locals).0;
                let all_toks: Vec<SignatureToken> = param_toks
                    .iter()
                    .chain(local_toks.iter())
                    .cloned()
                    .collect();
                let local_types = convert_sig_tokens(&module, &all_toks, struct_name_table);

                let mut converter =
                    Converter::new(num_params, num_locals, local_types, struct_name_table);
                converter.convert_function(&module, fdef, &code.code);

                let reg_types = converter.build_reg_types();

                FunctionIR {
                    name_idx,
                    handle_idx,
                    num_params,
                    num_locals,
                    num_regs: converter.next_reg,
                    instrs: converter.instrs,
                    reg_types,
                }
            })
        })
        .collect();

    ModuleIR { module, functions }
}

// ================================================================================================
// Converter
// ================================================================================================

struct Converter<'a> {
    /// Number of function parameters (fixed registers 0..num_params-1).
    num_params: Reg,
    /// Next register index.
    next_reg: Reg,
    /// Free registers bucketed by type for type-aware recycling.
    free_regs: BTreeMap<Type, Vec<Reg>>,
    /// Simulated operand stack with type information.
    stack: Vec<(Reg, Type)>,
    /// Types of all locals (params ++ declared locals).
    local_types: Vec<Type>,
    /// Types of temp registers.
    temp_types: Vec<Type>,
    /// Struct name table for type conversion.
    struct_name_table: &'a [StructNameIndex],
    /// Output instructions.
    instrs: Vec<Instr>,
    /// Map from bytecode offset to label.
    label_map: BTreeMap<CodeOffset, Label>,
    /// Next label index.
    next_label: u16,
}

impl<'a> Converter<'a> {
    fn new(
        num_params: Reg,
        num_declared_locals: Reg,
        local_types: Vec<Type>,
        struct_name_table: &'a [StructNameIndex],
    ) -> Self {
        let num_pinned = num_params + num_declared_locals;
        Self {
            num_params,
            next_reg: num_pinned,
            free_regs: BTreeMap::new(),
            stack: Vec::new(),
            local_types,
            temp_types: Vec::new(),
            struct_name_table,
            instrs: Vec::new(),
            label_map: BTreeMap::new(),
            next_label: 0,
        }
    }

    fn alloc_reg(&mut self, ty: &Type) -> Reg {
        // Try to recycle a free register of the same type before allocating a fresh one.
        if let Some(regs) = self.free_regs.get_mut(ty)
            && let Some(r) = regs.pop()
        {
            return r;
        }
        let r = self.next_reg;
        self.next_reg += 1;
        self.temp_types.push(ty.clone());
        r
    }

    fn alloc_or_hint(&mut self, hint: Option<Reg>, ty: &Type) -> Reg {
        match hint {
            Some(r) if !self.stack.iter().any(|(reg, _)| *reg == r) => {
                self.remove_from_free_regs(r);
                r
            },
            _ => self.alloc_reg(ty),
        }
    }

    fn free_reg(&mut self, r: Reg, ty: Type) {
        if r >= self.num_params {
            self.free_regs.entry(ty).or_default().push(r);
        }
    }

    /// Remove a register from all free-list buckets (used before reclaiming a
    /// local register for StLoc).
    fn remove_from_free_regs(&mut self, r: Reg) {
        for regs in self.free_regs.values_mut() {
            regs.retain(|&x| x != r);
        }
    }

    /// Evict a register from the stack by moving its value to a fresh register
    /// of the same type. Needed when a freed local register was re-allocated as
    /// a temp and now must be reclaimed for StLoc.
    fn evict_reg(&mut self, target: Reg) {
        if let Some(idx) = self.stack.iter().position(|(r, _)| *r == target) {
            let ty = self.stack[idx].1.clone();
            let new_r = self.alloc_reg(&ty);
            self.instrs.push(Instr::Move(new_r, target));
            self.stack[idx] = (new_r, ty);
        }
    }

    fn push(&mut self, r: Reg, ty: Type) {
        self.stack.push((r, ty));
    }

    fn pop(&mut self) -> (Reg, Type) {
        self.stack.pop().expect("stack underflow")
    }

    /// Pop N items from the stack in reverse order (first pushed = first in vec).
    fn pop_n_reverse(&mut self, n: usize) -> Vec<(Reg, Type)> {
        let mut items = Vec::with_capacity(n);
        for _ in 0..n {
            items.push(self.pop());
        }
        items.reverse();
        items
    }

    fn get_or_create_label(&mut self, offset: CodeOffset) -> Label {
        let next = self.next_label;
        let label = self.label_map.entry(offset).or_insert_with(|| Label(next));
        if label.0 == next {
            self.next_label += 1;
        }
        *label
    }

    /// Build the final `reg_types` vector (indexed by Reg).
    fn build_reg_types(&self) -> Vec<Type> {
        let mut reg_types = Vec::with_capacity(self.next_reg as usize);
        // Locals (params + declared locals) get their types from local_types.
        for ty in &self.local_types {
            reg_types.push(ty.clone());
        }
        // Temps get their types from temp_types.
        for ty in &self.temp_types {
            reg_types.push(ty.clone());
        }
        reg_types
    }

    // --------------------------------------------------------------------------------------------
    // Label assignment
    // --------------------------------------------------------------------------------------------

    /// Pre-scan all branch targets and assign labels.
    fn assign_labels(&mut self, code: &[Bytecode]) {
        for (offset, bc) in code.iter().enumerate() {
            match bc {
                Bytecode::Branch(target) | Bytecode::BrTrue(target) | Bytecode::BrFalse(target) => {
                    self.get_or_create_label(*target);
                },
                _ => {},
            }
            // Conditional branches need a fall-through label.
            if matches!(bc, Bytecode::BrTrue(_) | Bytecode::BrFalse(_)) {
                self.get_or_create_label((offset + 1) as CodeOffset);
            }
        }
    }

    // --------------------------------------------------------------------------------------------
    // Destination hints
    // --------------------------------------------------------------------------------------------

    /// Compute destination hints for a basic block range.
    /// Returns a map from bytecode offset to destination hint register.
    #[allow(clippy::needless_range_loop)]
    fn compute_hints(
        &self,
        code: &[Bytecode],
        start: usize,
        end: usize,
    ) -> BTreeMap<usize, Vec<Option<Reg>>> {
        let mut hints: BTreeMap<usize, Vec<Option<Reg>>> = BTreeMap::new();
        let mut slot_producers: Vec<(usize, usize)> = Vec::new();

        for offset in start..end {
            let bc = &code[offset];
            let (pops, pushes) = stack_effect(bc);

            let stack_len = slot_producers.len();
            let pop_start = stack_len.saturating_sub(pops);
            let popped: Vec<(usize, usize)> = slot_producers.drain(pop_start..).collect();

            if let Bytecode::StLoc(idx) = bc
                && popped.len() == 1
            {
                let (prod_off, prod_idx) = popped[0];
                if let Some(h) = hints
                    .entry(prod_off)
                    .or_insert_with(|| vec![None; push_count_at(&code[prod_off])])
                    .get_mut(prod_idx)
                {
                    *h = Some(*idx as Reg);
                }
            }

            for push_idx in 0..pushes {
                slot_producers.push((offset, push_idx));
            }
        }

        hints
    }

    // --------------------------------------------------------------------------------------------
    // Function conversion
    // --------------------------------------------------------------------------------------------

    #[allow(clippy::needless_range_loop)]
    fn convert_function(
        &mut self,
        module: &CompiledModule,
        _fdef: &FunctionDefinition,
        code: &[Bytecode],
    ) {
        self.assign_labels(code);

        let mut block_start = 0;
        let mut block_boundaries = Vec::new();

        for (offset, bc) in code.iter().enumerate() {
            if self.label_map.contains_key(&(offset as CodeOffset)) && offset > block_start {
                block_boundaries.push((block_start, offset));
                block_start = offset;
            }
            match bc {
                Bytecode::Branch(_)
                | Bytecode::BrTrue(_)
                | Bytecode::BrFalse(_)
                | Bytecode::Ret
                | Bytecode::Abort
                | Bytecode::AbortMsg => {
                    block_boundaries.push((block_start, offset + 1));
                    block_start = offset + 1;
                },
                _ => {},
            }
        }
        if block_start < code.len() {
            block_boundaries.push((block_start, code.len()));
        }

        for (start, end) in block_boundaries {
            let hints = self.compute_hints(code, start, end);
            for offset in start..end {
                if let Some(&label) = self.label_map.get(&(offset as CodeOffset)) {
                    self.instrs.push(Instr::Label(label));
                }
                let hint_vec = hints.get(&offset);
                self.convert_bytecode(module, &code[offset], hint_vec);
            }
        }
    }

    fn get_hint(&self, hint_vec: Option<&Vec<Option<Reg>>>, push_idx: usize) -> Option<Reg> {
        hint_vec.and_then(|v| v.get(push_idx).copied().flatten())
    }

    // --------------------------------------------------------------------------------------------
    // Type helpers
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

    fn field_type(&self, module: &CompiledModule, idx: FieldHandleIndex) -> Type {
        let tok = field_type_tok(module, idx);
        convert_sig_token(module, &tok, self.struct_name_table)
    }

    fn field_inst_type(&self, module: &CompiledModule, idx: FieldInstantiationIndex) -> Type {
        let tok = field_inst_type_tok(module, idx);
        convert_sig_token(module, &tok, self.struct_name_table)
    }

    fn variant_field_handle_type(
        &self,
        module: &CompiledModule,
        idx: move_binary_format::file_format::VariantFieldHandleIndex,
    ) -> Type {
        let tok = variant_field_handle_type_tok(module, idx);
        convert_sig_token(module, &tok, self.struct_name_table)
    }

    fn variant_field_inst_type(
        &self,
        module: &CompiledModule,
        idx: move_binary_format::file_format::VariantFieldInstantiationIndex,
    ) -> Type {
        let tok = variant_field_inst_type_tok(module, idx);
        convert_sig_token(module, &tok, self.struct_name_table)
    }

    // --------------------------------------------------------------------------------------------
    // Bytecode conversion
    // --------------------------------------------------------------------------------------------

    fn convert_bytecode(
        &mut self,
        module: &CompiledModule,
        bc: &Bytecode,
        hint_vec: Option<&Vec<Option<Reg>>>,
    ) {
        use Bytecode as B;
        match bc {
            // --- Loads ---
            B::LdU8(v) => {
                let ty = Type::U8;
                let dst = self.alloc_or_hint(self.get_hint(hint_vec, 0), &ty);
                self.instrs.push(Instr::LdU8(dst, *v));
                self.push(dst, ty);
            },
            B::LdU16(v) => {
                let ty = Type::U16;
                let dst = self.alloc_or_hint(self.get_hint(hint_vec, 0), &ty);
                self.instrs.push(Instr::LdU16(dst, *v));
                self.push(dst, ty);
            },
            B::LdU32(v) => {
                let ty = Type::U32;
                let dst = self.alloc_or_hint(self.get_hint(hint_vec, 0), &ty);
                self.instrs.push(Instr::LdU32(dst, *v));
                self.push(dst, ty);
            },
            B::LdU64(v) => {
                let ty = Type::U64;
                let dst = self.alloc_or_hint(self.get_hint(hint_vec, 0), &ty);
                self.instrs.push(Instr::LdU64(dst, *v));
                self.push(dst, ty);
            },
            B::LdU128(v) => {
                let ty = Type::U128;
                let dst = self.alloc_or_hint(self.get_hint(hint_vec, 0), &ty);
                self.instrs.push(Instr::LdU128(dst, *v));
                self.push(dst, ty);
            },
            B::LdU256(v) => {
                let ty = Type::U256;
                let dst = self.alloc_or_hint(self.get_hint(hint_vec, 0), &ty);
                self.instrs.push(Instr::LdU256(dst, *v));
                self.push(dst, ty);
            },
            B::LdI8(v) => {
                let ty = Type::I8;
                let dst = self.alloc_or_hint(self.get_hint(hint_vec, 0), &ty);
                self.instrs.push(Instr::LdI8(dst, *v));
                self.push(dst, ty);
            },
            B::LdI16(v) => {
                let ty = Type::I16;
                let dst = self.alloc_or_hint(self.get_hint(hint_vec, 0), &ty);
                self.instrs.push(Instr::LdI16(dst, *v));
                self.push(dst, ty);
            },
            B::LdI32(v) => {
                let ty = Type::I32;
                let dst = self.alloc_or_hint(self.get_hint(hint_vec, 0), &ty);
                self.instrs.push(Instr::LdI32(dst, *v));
                self.push(dst, ty);
            },
            B::LdI64(v) => {
                let ty = Type::I64;
                let dst = self.alloc_or_hint(self.get_hint(hint_vec, 0), &ty);
                self.instrs.push(Instr::LdI64(dst, *v));
                self.push(dst, ty);
            },
            B::LdI128(v) => {
                let ty = Type::I128;
                let dst = self.alloc_or_hint(self.get_hint(hint_vec, 0), &ty);
                self.instrs.push(Instr::LdI128(dst, *v));
                self.push(dst, ty);
            },
            B::LdI256(v) => {
                let ty = Type::I256;
                let dst = self.alloc_or_hint(self.get_hint(hint_vec, 0), &ty);
                self.instrs.push(Instr::LdI256(dst, *v));
                self.push(dst, ty);
            },
            B::LdConst(idx) => {
                let tok = &module.constant_pool[idx.0 as usize].type_;
                let ty = convert_sig_token(module, tok, self.struct_name_table);
                let dst = self.alloc_or_hint(self.get_hint(hint_vec, 0), &ty);
                self.instrs.push(Instr::LdConst(dst, *idx));
                self.push(dst, ty);
            },
            B::LdTrue => {
                let ty = Type::Bool;
                let dst = self.alloc_or_hint(self.get_hint(hint_vec, 0), &ty);
                self.instrs.push(Instr::LdTrue(dst));
                self.push(dst, ty);
            },
            B::LdFalse => {
                let ty = Type::Bool;
                let dst = self.alloc_or_hint(self.get_hint(hint_vec, 0), &ty);
                self.instrs.push(Instr::LdFalse(dst));
                self.push(dst, ty);
            },

            // --- Locals ---
            B::CopyLoc(idx) => {
                let src = *idx as Reg;
                let ty = self.local_types[*idx as usize].clone();
                let dst = self.alloc_or_hint(self.get_hint(hint_vec, 0), &ty);
                if dst != src {
                    self.instrs.push(Instr::Copy(dst, src));
                }
                self.push(dst, ty);
            },
            B::MoveLoc(idx) => {
                let src = *idx as Reg;
                let ty = self.local_types[*idx as usize].clone();
                let dst = self.alloc_or_hint(self.get_hint(hint_vec, 0), &ty);
                if dst != src {
                    self.instrs.push(Instr::Move(dst, src));
                    self.free_reg(src, ty.clone());
                }
                self.push(dst, ty);
            },
            B::StLoc(idx) => {
                let (src, ty) = self.pop();
                let dst = *idx as Reg;
                // Reclaim the local register if it was in the free list.
                self.remove_from_free_regs(dst);
                // Evict any stack temp currently using this register.
                self.evict_reg(dst);
                if src != dst {
                    self.instrs.push(Instr::Move(dst, src));
                    self.free_reg(src, ty);
                }
            },

            // --- Pop ---
            B::Pop => {
                let (r, ty) = self.pop();
                self.free_reg(r, ty);
            },

            // --- Binary ops ---
            B::Add => self.convert_binop(BinaryOp::Add, hint_vec),
            B::Sub => self.convert_binop(BinaryOp::Sub, hint_vec),
            B::Mul => self.convert_binop(BinaryOp::Mul, hint_vec),
            B::Div => self.convert_binop(BinaryOp::Div, hint_vec),
            B::Mod => self.convert_binop(BinaryOp::Mod, hint_vec),
            B::BitOr => self.convert_binop(BinaryOp::BitOr, hint_vec),
            B::BitAnd => self.convert_binop(BinaryOp::BitAnd, hint_vec),
            B::Xor => self.convert_binop(BinaryOp::Xor, hint_vec),
            B::Shl => self.convert_binop(BinaryOp::Shl, hint_vec),
            B::Shr => self.convert_binop(BinaryOp::Shr, hint_vec),
            B::Lt => self.convert_binop(BinaryOp::Lt, hint_vec),
            B::Gt => self.convert_binop(BinaryOp::Gt, hint_vec),
            B::Le => self.convert_binop(BinaryOp::Le, hint_vec),
            B::Ge => self.convert_binop(BinaryOp::Ge, hint_vec),
            B::Eq => self.convert_binop(BinaryOp::Eq, hint_vec),
            B::Neq => self.convert_binop(BinaryOp::Neq, hint_vec),
            B::Or => self.convert_binop(BinaryOp::Or, hint_vec),
            B::And => self.convert_binop(BinaryOp::And, hint_vec),

            // --- Unary ops ---
            B::CastU8 => self.convert_unop(UnaryOp::CastU8, hint_vec),
            B::CastU16 => self.convert_unop(UnaryOp::CastU16, hint_vec),
            B::CastU32 => self.convert_unop(UnaryOp::CastU32, hint_vec),
            B::CastU64 => self.convert_unop(UnaryOp::CastU64, hint_vec),
            B::CastU128 => self.convert_unop(UnaryOp::CastU128, hint_vec),
            B::CastU256 => self.convert_unop(UnaryOp::CastU256, hint_vec),
            B::CastI8 => self.convert_unop(UnaryOp::CastI8, hint_vec),
            B::CastI16 => self.convert_unop(UnaryOp::CastI16, hint_vec),
            B::CastI32 => self.convert_unop(UnaryOp::CastI32, hint_vec),
            B::CastI64 => self.convert_unop(UnaryOp::CastI64, hint_vec),
            B::CastI128 => self.convert_unop(UnaryOp::CastI128, hint_vec),
            B::CastI256 => self.convert_unop(UnaryOp::CastI256, hint_vec),
            B::Not => self.convert_unop(UnaryOp::Not, hint_vec),
            B::Negate => self.convert_unop(UnaryOp::Negate, hint_vec),
            B::FreezeRef => self.convert_unop(UnaryOp::FreezeRef, hint_vec),

            // --- Struct ops ---
            B::Pack(idx) => {
                let field_count = struct_field_count(module, *idx);
                let fields_typed = self.pop_n_reverse(field_count);
                let fields: Vec<Reg> = fields_typed.iter().map(|(r, _)| *r).collect();
                let result_ty = self.struct_type(module, *idx);
                let dst = self.alloc_or_hint(self.get_hint(hint_vec, 0), &result_ty);
                self.instrs.push(Instr::Pack(dst, *idx, fields));
                for (r, ty) in fields_typed {
                    self.free_reg(r, ty);
                }
                self.push(dst, result_ty);
            },
            B::PackGeneric(idx) => {
                let inst = &module.struct_def_instantiations[idx.0 as usize];
                let field_count = struct_field_count(module, inst.def);
                let fields_typed = self.pop_n_reverse(field_count);
                let fields: Vec<Reg> = fields_typed.iter().map(|(r, _)| *r).collect();
                let result_ty = self.struct_inst_type(module, *idx);
                let dst = self.alloc_or_hint(self.get_hint(hint_vec, 0), &result_ty);
                self.instrs.push(Instr::PackGeneric(dst, *idx, fields));
                for (r, ty) in fields_typed {
                    self.free_reg(r, ty);
                }
                self.push(dst, result_ty);
            },
            B::Unpack(idx) => {
                let (src, src_ty) = self.pop();
                let ftypes = struct_field_type_toks(module, *idx);
                let ftypes: Vec<Type> = convert_sig_tokens(module, &ftypes, self.struct_name_table);
                let mut dsts = Vec::with_capacity(ftypes.len());
                for (i, fty) in ftypes.iter().enumerate() {
                    dsts.push(self.alloc_or_hint(self.get_hint(hint_vec, i), fty));
                }
                self.instrs.push(Instr::Unpack(dsts.clone(), *idx, src));
                self.free_reg(src, src_ty);
                for (d, fty) in dsts.into_iter().zip(ftypes) {
                    self.push(d, fty);
                }
            },
            B::UnpackGeneric(idx) => {
                let (src, src_ty) = self.pop();
                let inst = &module.struct_def_instantiations[idx.0 as usize];
                let type_params = module.signature_at(inst.type_parameters).0.clone();
                let raw_ftypes = struct_field_type_toks(module, inst.def);
                let ftypes: Vec<SignatureToken> = raw_ftypes
                    .iter()
                    .map(|ft| substitute_type_params(ft, &type_params))
                    .collect();
                let ftypes: Vec<Type> = convert_sig_tokens(module, &ftypes, self.struct_name_table);
                let mut dsts = Vec::with_capacity(ftypes.len());
                for (i, fty) in ftypes.iter().enumerate() {
                    dsts.push(self.alloc_or_hint(self.get_hint(hint_vec, i), fty));
                }
                self.instrs
                    .push(Instr::UnpackGeneric(dsts.clone(), *idx, src));
                self.free_reg(src, src_ty);
                for (d, fty) in dsts.into_iter().zip(ftypes) {
                    self.push(d, fty);
                }
            },

            // --- Variant ops ---
            B::PackVariant(idx) => {
                let handle = &module.struct_variant_handles[idx.0 as usize];
                let field_count = variant_field_count(module, handle.struct_index, handle.variant);
                let fields_typed = self.pop_n_reverse(field_count);
                let fields: Vec<Reg> = fields_typed.iter().map(|(r, _)| *r).collect();
                let result_ty = self.struct_type(module, handle.struct_index);
                let dst = self.alloc_or_hint(self.get_hint(hint_vec, 0), &result_ty);
                self.instrs.push(Instr::PackVariant(dst, *idx, fields));
                for (r, ty) in fields_typed {
                    self.free_reg(r, ty);
                }
                self.push(dst, result_ty);
            },
            B::PackVariantGeneric(idx) => {
                let inst = &module.struct_variant_instantiations[idx.0 as usize];
                let handle = &module.struct_variant_handles[inst.handle.0 as usize];
                let field_count = variant_field_count(module, handle.struct_index, handle.variant);
                let fields_typed = self.pop_n_reverse(field_count);
                let fields: Vec<Reg> = fields_typed.iter().map(|(r, _)| *r).collect();
                let type_params = module.signature_at(inst.type_parameters).0.clone();
                let def = &module.struct_defs[handle.struct_index.0 as usize];
                let tok = SignatureToken::StructInstantiation(def.struct_handle, type_params);
                let result_ty = convert_sig_token(module, &tok, self.struct_name_table);
                let dst = self.alloc_or_hint(self.get_hint(hint_vec, 0), &result_ty);
                self.instrs
                    .push(Instr::PackVariantGeneric(dst, *idx, fields));
                for (r, ty) in fields_typed {
                    self.free_reg(r, ty);
                }
                self.push(dst, result_ty);
            },
            B::UnpackVariant(idx) => {
                let (src, src_ty) = self.pop();
                let handle = &module.struct_variant_handles[idx.0 as usize];
                let ftypes_tok =
                    variant_field_type_toks(module, handle.struct_index, handle.variant);
                let ftypes: Vec<Type> =
                    convert_sig_tokens(module, &ftypes_tok, self.struct_name_table);
                let mut dsts = Vec::with_capacity(ftypes.len());
                for (i, fty) in ftypes.iter().enumerate() {
                    dsts.push(self.alloc_or_hint(self.get_hint(hint_vec, i), fty));
                }
                self.instrs
                    .push(Instr::UnpackVariant(dsts.clone(), *idx, src));
                self.free_reg(src, src_ty);
                for (d, fty) in dsts.into_iter().zip(ftypes) {
                    self.push(d, fty);
                }
            },
            B::UnpackVariantGeneric(idx) => {
                let (src, src_ty) = self.pop();
                let inst = &module.struct_variant_instantiations[idx.0 as usize];
                let handle = &module.struct_variant_handles[inst.handle.0 as usize];
                let type_params = module.signature_at(inst.type_parameters).0.clone();
                let raw_ftypes =
                    variant_field_type_toks(module, handle.struct_index, handle.variant);
                let ftypes_tok: Vec<SignatureToken> = raw_ftypes
                    .iter()
                    .map(|ft| substitute_type_params(ft, &type_params))
                    .collect();
                let ftypes: Vec<Type> =
                    convert_sig_tokens(module, &ftypes_tok, self.struct_name_table);
                let mut dsts = Vec::with_capacity(ftypes.len());
                for (i, fty) in ftypes.iter().enumerate() {
                    dsts.push(self.alloc_or_hint(self.get_hint(hint_vec, i), fty));
                }
                self.instrs
                    .push(Instr::UnpackVariantGeneric(dsts.clone(), *idx, src));
                self.free_reg(src, src_ty);
                for (d, fty) in dsts.into_iter().zip(ftypes) {
                    self.push(d, fty);
                }
            },
            B::TestVariant(idx) => {
                let (src, src_ty) = self.pop();
                let result_ty = Type::Bool;
                let dst = self.alloc_or_hint(self.get_hint(hint_vec, 0), &result_ty);
                self.instrs.push(Instr::TestVariant(dst, *idx, src));
                self.free_reg(src, src_ty);
                self.push(dst, result_ty);
            },
            B::TestVariantGeneric(idx) => {
                let (src, src_ty) = self.pop();
                let result_ty = Type::Bool;
                let dst = self.alloc_or_hint(self.get_hint(hint_vec, 0), &result_ty);
                self.instrs.push(Instr::TestVariantGeneric(dst, *idx, src));
                self.free_reg(src, src_ty);
                self.push(dst, result_ty);
            },

            // --- References ---
            B::ImmBorrowLoc(idx) => {
                let src = *idx as Reg;
                let inner = self.local_types[*idx as usize].clone();
                let result_ty = Type::Reference(Box::new(inner));
                let dst = self.alloc_or_hint(self.get_hint(hint_vec, 0), &result_ty);
                self.instrs.push(Instr::ImmBorrowLoc(dst, src));
                self.push(dst, result_ty);
            },
            B::MutBorrowLoc(idx) => {
                let src = *idx as Reg;
                let inner = self.local_types[*idx as usize].clone();
                let result_ty = Type::MutableReference(Box::new(inner));
                let dst = self.alloc_or_hint(self.get_hint(hint_vec, 0), &result_ty);
                self.instrs.push(Instr::MutBorrowLoc(dst, src));
                self.push(dst, result_ty);
            },
            B::ImmBorrowField(idx) => {
                let (src, src_ty) = self.pop();
                let fty = self.field_type(module, *idx);
                let result_ty = Type::Reference(Box::new(fty));
                let dst = self.alloc_or_hint(self.get_hint(hint_vec, 0), &result_ty);
                self.instrs.push(Instr::ImmBorrowField(dst, *idx, src));
                self.free_reg(src, src_ty);
                self.push(dst, result_ty);
            },
            B::MutBorrowField(idx) => {
                let (src, src_ty) = self.pop();
                let fty = self.field_type(module, *idx);
                let result_ty = Type::MutableReference(Box::new(fty));
                let dst = self.alloc_or_hint(self.get_hint(hint_vec, 0), &result_ty);
                self.instrs.push(Instr::MutBorrowField(dst, *idx, src));
                self.free_reg(src, src_ty);
                self.push(dst, result_ty);
            },
            B::ImmBorrowFieldGeneric(idx) => {
                let (src, src_ty) = self.pop();
                let fty = self.field_inst_type(module, *idx);
                let result_ty = Type::Reference(Box::new(fty));
                let dst = self.alloc_or_hint(self.get_hint(hint_vec, 0), &result_ty);
                self.instrs
                    .push(Instr::ImmBorrowFieldGeneric(dst, *idx, src));
                self.free_reg(src, src_ty);
                self.push(dst, result_ty);
            },
            B::MutBorrowFieldGeneric(idx) => {
                let (src, src_ty) = self.pop();
                let fty = self.field_inst_type(module, *idx);
                let result_ty = Type::MutableReference(Box::new(fty));
                let dst = self.alloc_or_hint(self.get_hint(hint_vec, 0), &result_ty);
                self.instrs
                    .push(Instr::MutBorrowFieldGeneric(dst, *idx, src));
                self.free_reg(src, src_ty);
                self.push(dst, result_ty);
            },
            B::ImmBorrowVariantField(idx) => {
                let (src, src_ty) = self.pop();
                let fty = self.variant_field_handle_type(module, *idx);
                let result_ty = Type::Reference(Box::new(fty));
                let dst = self.alloc_or_hint(self.get_hint(hint_vec, 0), &result_ty);
                self.instrs
                    .push(Instr::ImmBorrowVariantField(dst, *idx, src));
                self.free_reg(src, src_ty);
                self.push(dst, result_ty);
            },
            B::MutBorrowVariantField(idx) => {
                let (src, src_ty) = self.pop();
                let fty = self.variant_field_handle_type(module, *idx);
                let result_ty = Type::MutableReference(Box::new(fty));
                let dst = self.alloc_or_hint(self.get_hint(hint_vec, 0), &result_ty);
                self.instrs
                    .push(Instr::MutBorrowVariantField(dst, *idx, src));
                self.free_reg(src, src_ty);
                self.push(dst, result_ty);
            },
            B::ImmBorrowVariantFieldGeneric(idx) => {
                let (src, src_ty) = self.pop();
                let fty = self.variant_field_inst_type(module, *idx);
                let result_ty = Type::Reference(Box::new(fty));
                let dst = self.alloc_or_hint(self.get_hint(hint_vec, 0), &result_ty);
                self.instrs
                    .push(Instr::ImmBorrowVariantFieldGeneric(dst, *idx, src));
                self.free_reg(src, src_ty);
                self.push(dst, result_ty);
            },
            B::MutBorrowVariantFieldGeneric(idx) => {
                let (src, src_ty) = self.pop();
                let fty = self.variant_field_inst_type(module, *idx);
                let result_ty = Type::MutableReference(Box::new(fty));
                let dst = self.alloc_or_hint(self.get_hint(hint_vec, 0), &result_ty);
                self.instrs
                    .push(Instr::MutBorrowVariantFieldGeneric(dst, *idx, src));
                self.free_reg(src, src_ty);
                self.push(dst, result_ty);
            },
            B::ReadRef => {
                let (src, src_ty) = self.pop();
                let result_ty = match &src_ty {
                    Type::Reference(inner) | Type::MutableReference(inner) => (**inner).clone(),
                    other => other.clone(),
                };
                let dst = self.alloc_or_hint(self.get_hint(hint_vec, 0), &result_ty);
                self.instrs.push(Instr::ReadRef(dst, src));
                self.free_reg(src, src_ty);
                self.push(dst, result_ty);
            },
            B::WriteRef => {
                let (ref_r, ref_ty) = self.pop();
                let (val, val_ty) = self.pop();
                self.instrs.push(Instr::WriteRef(ref_r, val));
                self.free_reg(ref_r, ref_ty);
                self.free_reg(val, val_ty);
            },

            // --- Globals ---
            B::Exists(idx) => {
                let (addr, addr_ty) = self.pop();
                let result_ty = Type::Bool;
                let dst = self.alloc_or_hint(self.get_hint(hint_vec, 0), &result_ty);
                self.instrs.push(Instr::Exists(dst, *idx, addr));
                self.free_reg(addr, addr_ty);
                self.push(dst, result_ty);
            },
            B::ExistsGeneric(idx) => {
                let (addr, addr_ty) = self.pop();
                let result_ty = Type::Bool;
                let dst = self.alloc_or_hint(self.get_hint(hint_vec, 0), &result_ty);
                self.instrs.push(Instr::ExistsGeneric(dst, *idx, addr));
                self.free_reg(addr, addr_ty);
                self.push(dst, result_ty);
            },
            B::MoveFrom(idx) => {
                let (addr, addr_ty) = self.pop();
                let result_ty = self.struct_type(module, *idx);
                let dst = self.alloc_or_hint(self.get_hint(hint_vec, 0), &result_ty);
                self.instrs.push(Instr::MoveFrom(dst, *idx, addr));
                self.free_reg(addr, addr_ty);
                self.push(dst, result_ty);
            },
            B::MoveFromGeneric(idx) => {
                let (addr, addr_ty) = self.pop();
                let result_ty = self.struct_inst_type(module, *idx);
                let dst = self.alloc_or_hint(self.get_hint(hint_vec, 0), &result_ty);
                self.instrs.push(Instr::MoveFromGeneric(dst, *idx, addr));
                self.free_reg(addr, addr_ty);
                self.push(dst, result_ty);
            },
            B::MoveTo(idx) => {
                let (val, val_ty) = self.pop();
                let (signer, signer_ty) = self.pop();
                self.instrs.push(Instr::MoveTo(*idx, signer, val));
                self.free_reg(signer, signer_ty);
                self.free_reg(val, val_ty);
            },
            B::MoveToGeneric(idx) => {
                let (val, val_ty) = self.pop();
                let (signer, signer_ty) = self.pop();
                self.instrs.push(Instr::MoveToGeneric(*idx, signer, val));
                self.free_reg(signer, signer_ty);
                self.free_reg(val, val_ty);
            },
            B::ImmBorrowGlobal(idx) => {
                let (addr, addr_ty) = self.pop();
                let result_ty = Type::Reference(Box::new(self.struct_type(module, *idx)));
                let dst = self.alloc_or_hint(self.get_hint(hint_vec, 0), &result_ty);
                self.instrs.push(Instr::ImmBorrowGlobal(dst, *idx, addr));
                self.free_reg(addr, addr_ty);
                self.push(dst, result_ty);
            },
            B::ImmBorrowGlobalGeneric(idx) => {
                let (addr, addr_ty) = self.pop();
                let result_ty = Type::Reference(Box::new(self.struct_inst_type(module, *idx)));
                let dst = self.alloc_or_hint(self.get_hint(hint_vec, 0), &result_ty);
                self.instrs
                    .push(Instr::ImmBorrowGlobalGeneric(dst, *idx, addr));
                self.free_reg(addr, addr_ty);
                self.push(dst, result_ty);
            },
            B::MutBorrowGlobal(idx) => {
                let (addr, addr_ty) = self.pop();
                let result_ty = Type::MutableReference(Box::new(self.struct_type(module, *idx)));
                let dst = self.alloc_or_hint(self.get_hint(hint_vec, 0), &result_ty);
                self.instrs.push(Instr::MutBorrowGlobal(dst, *idx, addr));
                self.free_reg(addr, addr_ty);
                self.push(dst, result_ty);
            },
            B::MutBorrowGlobalGeneric(idx) => {
                let (addr, addr_ty) = self.pop();
                let result_ty =
                    Type::MutableReference(Box::new(self.struct_inst_type(module, *idx)));
                let dst = self.alloc_or_hint(self.get_hint(hint_vec, 0), &result_ty);
                self.instrs
                    .push(Instr::MutBorrowGlobalGeneric(dst, *idx, addr));
                self.free_reg(addr, addr_ty);
                self.push(dst, result_ty);
            },

            // --- Calls ---
            B::Call(idx) => {
                let handle = module.function_handle_at(*idx);
                let num_args = module.signature_at(handle.parameters).0.len();
                let ret_toks = &module.signature_at(handle.return_).0;
                let ret_types = convert_sig_tokens(module, ret_toks, self.struct_name_table);
                let args_typed = self.pop_n_reverse(num_args);
                let args: Vec<Reg> = args_typed.iter().map(|(r, _)| *r).collect();
                let mut rets = Vec::with_capacity(ret_types.len());
                for (i, rty) in ret_types.iter().enumerate() {
                    rets.push(self.alloc_or_hint(self.get_hint(hint_vec, i), rty));
                }
                self.instrs.push(Instr::Call(rets.clone(), *idx, args));
                for (r, ty) in args_typed {
                    self.free_reg(r, ty);
                }
                for (r, rty) in rets.into_iter().zip(ret_types) {
                    self.push(r, rty);
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
                let args_typed = self.pop_n_reverse(num_args);
                let args: Vec<Reg> = args_typed.iter().map(|(r, _)| *r).collect();
                let mut rets = Vec::with_capacity(ret_types.len());
                for (i, rty) in ret_types.iter().enumerate() {
                    rets.push(self.alloc_or_hint(self.get_hint(hint_vec, i), rty));
                }
                self.instrs
                    .push(Instr::CallGeneric(rets.clone(), *idx, args));
                for (r, ty) in args_typed {
                    self.free_reg(r, ty);
                }
                for (r, rty) in rets.into_iter().zip(ret_types) {
                    self.push(r, rty);
                }
            },

            // --- Closures ---
            B::PackClosure(fhi, mask) => {
                let captured_count = mask.captured_count() as usize;
                let captured_typed = self.pop_n_reverse(captured_count);
                let captured: Vec<Reg> = captured_typed.iter().map(|(r, _)| *r).collect();
                let handle = module.function_handle_at(*fhi);
                let params = &module.signature_at(handle.parameters).0;
                let returns = &module.signature_at(handle.return_).0;
                let tok = SignatureToken::Function(
                    params.clone(),
                    returns.clone(),
                    move_core_types::ability::AbilitySet::EMPTY,
                );
                let result_ty = convert_sig_token(module, &tok, self.struct_name_table);
                let dst = self.alloc_or_hint(self.get_hint(hint_vec, 0), &result_ty);
                self.instrs
                    .push(Instr::PackClosure(dst, *fhi, *mask, captured));
                for (r, ty) in captured_typed {
                    self.free_reg(r, ty);
                }
                self.push(dst, result_ty);
            },
            B::PackClosureGeneric(fii, mask) => {
                let captured_count = mask.captured_count() as usize;
                let captured_typed = self.pop_n_reverse(captured_count);
                let captured: Vec<Reg> = captured_typed.iter().map(|(r, _)| *r).collect();
                let inst = &module.function_instantiations[fii.0 as usize];
                let handle = module.function_handle_at(inst.handle);
                let params = &module.signature_at(handle.parameters).0;
                let returns = &module.signature_at(handle.return_).0;
                let tok = SignatureToken::Function(
                    params.clone(),
                    returns.clone(),
                    move_core_types::ability::AbilitySet::EMPTY,
                );
                let result_ty = convert_sig_token(module, &tok, self.struct_name_table);
                let dst = self.alloc_or_hint(self.get_hint(hint_vec, 0), &result_ty);
                self.instrs
                    .push(Instr::PackClosureGeneric(dst, *fii, *mask, captured));
                for (r, ty) in captured_typed {
                    self.free_reg(r, ty);
                }
                self.push(dst, result_ty);
            },
            B::CallClosure(sig_idx) => {
                let sig = module.signature_at(*sig_idx);
                let (num_args, ret_toks) =
                    if let Some(SignatureToken::Function(params, results, _)) = sig.0.first() {
                        (params.len(), results.clone())
                    } else {
                        (sig.0.len().saturating_sub(1), vec![])
                    };
                let ret_types = convert_sig_tokens(module, &ret_toks, self.struct_name_table);
                let (closure, closure_ty) = self.pop();
                let args_typed = self.pop_n_reverse(num_args);
                let mut all_args: Vec<Reg> = args_typed.iter().map(|(r, _)| *r).collect();
                all_args.push(closure);
                let mut rets = Vec::with_capacity(ret_types.len());
                for (i, rty) in ret_types.iter().enumerate() {
                    rets.push(self.alloc_or_hint(self.get_hint(hint_vec, i), rty));
                }
                self.instrs
                    .push(Instr::CallClosure(rets.clone(), *sig_idx, all_args));
                self.free_reg(closure, closure_ty);
                for (r, ty) in args_typed {
                    self.free_reg(r, ty);
                }
                for (r, rty) in rets.into_iter().zip(ret_types) {
                    self.push(r, rty);
                }
            },

            // --- Vector ops ---
            B::VecPack(sig_idx, count) => {
                let elems_typed = self.pop_n_reverse(*count as usize);
                let elems: Vec<Reg> = elems_typed.iter().map(|(r, _)| *r).collect();
                let elem_tok = &module.signature_at(*sig_idx).0[0];
                let elem_ty = convert_sig_token(module, elem_tok, self.struct_name_table);
                let result_ty = Type::Vector(triomphe::Arc::new(elem_ty));
                let dst = self.alloc_or_hint(self.get_hint(hint_vec, 0), &result_ty);
                self.instrs
                    .push(Instr::VecPack(dst, *sig_idx, *count, elems));
                for (r, ty) in elems_typed {
                    self.free_reg(r, ty);
                }
                self.push(dst, result_ty);
            },
            B::VecLen(sig_idx) => {
                let (vec_ref, vec_ty) = self.pop();
                let result_ty = Type::U64;
                let dst = self.alloc_or_hint(self.get_hint(hint_vec, 0), &result_ty);
                self.instrs.push(Instr::VecLen(dst, *sig_idx, vec_ref));
                self.free_reg(vec_ref, vec_ty);
                self.push(dst, result_ty);
            },
            B::VecImmBorrow(sig_idx) => {
                let (idx_r, idx_ty) = self.pop();
                let (vec_ref, vec_ty) = self.pop();
                let elem_tok = &module.signature_at(*sig_idx).0[0];
                let elem_ty = convert_sig_token(module, elem_tok, self.struct_name_table);
                let result_ty = Type::Reference(Box::new(elem_ty));
                let dst = self.alloc_or_hint(self.get_hint(hint_vec, 0), &result_ty);
                self.instrs
                    .push(Instr::VecImmBorrow(dst, *sig_idx, vec_ref, idx_r));
                self.free_reg(vec_ref, vec_ty);
                self.free_reg(idx_r, idx_ty);
                self.push(dst, result_ty);
            },
            B::VecMutBorrow(sig_idx) => {
                let (idx_r, idx_ty) = self.pop();
                let (vec_ref, vec_ty) = self.pop();
                let elem_tok = &module.signature_at(*sig_idx).0[0];
                let elem_ty = convert_sig_token(module, elem_tok, self.struct_name_table);
                let result_ty = Type::MutableReference(Box::new(elem_ty));
                let dst = self.alloc_or_hint(self.get_hint(hint_vec, 0), &result_ty);
                self.instrs
                    .push(Instr::VecMutBorrow(dst, *sig_idx, vec_ref, idx_r));
                self.free_reg(vec_ref, vec_ty);
                self.free_reg(idx_r, idx_ty);
                self.push(dst, result_ty);
            },
            B::VecPushBack(sig_idx) => {
                let (val, val_ty) = self.pop();
                let (vec_ref, vec_ty) = self.pop();
                self.instrs.push(Instr::VecPushBack(*sig_idx, vec_ref, val));
                self.free_reg(vec_ref, vec_ty);
                self.free_reg(val, val_ty);
            },
            B::VecPopBack(sig_idx) => {
                let (vec_ref, vec_ty) = self.pop();
                let elem_tok = &module.signature_at(*sig_idx).0[0];
                let result_ty = convert_sig_token(module, elem_tok, self.struct_name_table);
                let dst = self.alloc_or_hint(self.get_hint(hint_vec, 0), &result_ty);
                self.instrs.push(Instr::VecPopBack(dst, *sig_idx, vec_ref));
                self.free_reg(vec_ref, vec_ty);
                self.push(dst, result_ty);
            },
            B::VecUnpack(sig_idx, count) => {
                let (src, src_ty) = self.pop();
                let elem_tok = &module.signature_at(*sig_idx).0[0];
                let elem_ty = convert_sig_token(module, elem_tok, self.struct_name_table);
                let mut dsts = Vec::with_capacity(*count as usize);
                for i in 0..*count as usize {
                    dsts.push(self.alloc_or_hint(self.get_hint(hint_vec, i), &elem_ty));
                }
                self.instrs
                    .push(Instr::VecUnpack(dsts.clone(), *sig_idx, *count, src));
                self.free_reg(src, src_ty);
                for d in dsts {
                    self.push(d, elem_ty.clone());
                }
            },
            B::VecSwap(sig_idx) => {
                let (j, j_ty) = self.pop();
                let (i, i_ty) = self.pop();
                let (vec_ref, vec_ty) = self.pop();
                self.instrs.push(Instr::VecSwap(*sig_idx, vec_ref, i, j));
                self.free_reg(vec_ref, vec_ty);
                self.free_reg(i, i_ty);
                self.free_reg(j, j_ty);
            },

            // --- Control flow ---
            B::Branch(target) => {
                let label = self.label_map[target];
                self.instrs.push(Instr::Branch(label));
            },
            B::BrTrue(target) => {
                let (cond, cond_ty) = self.pop();
                let label = self.label_map[target];
                self.instrs.push(Instr::BrTrue(label, cond));
                self.free_reg(cond, cond_ty);
            },
            B::BrFalse(target) => {
                let (cond, cond_ty) = self.pop();
                let label = self.label_map[target];
                self.instrs.push(Instr::BrFalse(label, cond));
                self.free_reg(cond, cond_ty);
            },
            B::Ret => {
                let typed_rets: Vec<(Reg, Type)> = self.stack.drain(..).collect();
                let rets: Vec<Reg> = typed_rets.iter().map(|(r, _)| *r).collect();
                self.instrs.push(Instr::Ret(rets));
            },
            B::Abort => {
                let (code, code_ty) = self.pop();
                self.instrs.push(Instr::Abort(code));
                self.free_reg(code, code_ty);
            },
            B::AbortMsg => {
                let (msg, msg_ty) = self.pop();
                let (code, code_ty) = self.pop();
                self.instrs.push(Instr::AbortMsg(code, msg));
                self.free_reg(code, code_ty);
                self.free_reg(msg, msg_ty);
            },

            // --- Misc ---
            B::Nop => {},
        }
    }

    fn convert_binop(&mut self, op: BinaryOp, hint_vec: Option<&Vec<Option<Reg>>>) {
        let (rhs, rhs_ty) = self.pop();
        let (lhs, lhs_ty) = self.pop();
        let result_ty = match op {
            BinaryOp::Lt
            | BinaryOp::Gt
            | BinaryOp::Le
            | BinaryOp::Ge
            | BinaryOp::Eq
            | BinaryOp::Neq
            | BinaryOp::Or
            | BinaryOp::And => Type::Bool,
            _ => lhs_ty.clone(),
        };
        let dst = self.alloc_or_hint(self.get_hint(hint_vec, 0), &result_ty);
        self.instrs.push(Instr::BinaryOp(dst, op, lhs, rhs));
        self.free_reg(lhs, lhs_ty);
        self.free_reg(rhs, rhs_ty);
        self.push(dst, result_ty);
    }

    fn convert_unop(&mut self, op: UnaryOp, hint_vec: Option<&Vec<Option<Reg>>>) {
        let (src, src_ty) = self.pop();
        let result_ty = match op {
            UnaryOp::CastU8 => Type::U8,
            UnaryOp::CastU16 => Type::U16,
            UnaryOp::CastU32 => Type::U32,
            UnaryOp::CastU64 => Type::U64,
            UnaryOp::CastU128 => Type::U128,
            UnaryOp::CastU256 => Type::U256,
            UnaryOp::CastI8 => Type::I8,
            UnaryOp::CastI16 => Type::I16,
            UnaryOp::CastI32 => Type::I32,
            UnaryOp::CastI64 => Type::I64,
            UnaryOp::CastI128 => Type::I128,
            UnaryOp::CastI256 => Type::I256,
            UnaryOp::Not => Type::Bool,
            UnaryOp::Negate => src_ty.clone(),
            UnaryOp::FreezeRef => match &src_ty {
                Type::MutableReference(inner) => Type::Reference(Box::new((**inner).clone())),
                other => other.clone(),
            },
        };
        let dst = self.alloc_or_hint(self.get_hint(hint_vec, 0), &result_ty);
        self.instrs.push(Instr::UnaryOp(dst, op, src));
        self.free_reg(src, src_ty);
        self.push(dst, result_ty);
    }
}

// ================================================================================================
// SignatureToken helpers (for field count/type extraction before conversion)
// ================================================================================================

fn struct_field_type_toks(
    module: &CompiledModule,
    idx: StructDefinitionIndex,
) -> Vec<SignatureToken> {
    match &module.struct_defs[idx.0 as usize].field_information {
        StructFieldInformation::Declared(fields) => {
            fields.iter().map(|f| f.signature.0.clone()).collect()
        },
        _ => vec![],
    }
}

fn struct_field_count(module: &CompiledModule, idx: StructDefinitionIndex) -> usize {
    match &module.struct_defs[idx.0 as usize].field_information {
        StructFieldInformation::Native => 0,
        StructFieldInformation::Declared(fields) => fields.len(),
        StructFieldInformation::DeclaredVariants(_) => 0,
    }
}

fn variant_field_count(
    module: &CompiledModule,
    struct_idx: StructDefinitionIndex,
    variant: move_binary_format::file_format::VariantIndex,
) -> usize {
    match &module.struct_defs[struct_idx.0 as usize].field_information {
        StructFieldInformation::DeclaredVariants(variants) => {
            variants[variant as usize].fields.len()
        },
        _ => 0,
    }
}

fn variant_field_type_toks(
    module: &CompiledModule,
    struct_idx: StructDefinitionIndex,
    variant: move_binary_format::file_format::VariantIndex,
) -> Vec<SignatureToken> {
    match &module.struct_defs[struct_idx.0 as usize].field_information {
        StructFieldInformation::DeclaredVariants(variants) => variants[variant as usize]
            .fields
            .iter()
            .map(|f| f.signature.0.clone())
            .collect(),
        _ => vec![],
    }
}

fn field_type_tok(module: &CompiledModule, idx: FieldHandleIndex) -> SignatureToken {
    let handle = &module.field_handles[idx.0 as usize];
    match &module.struct_defs[handle.owner.0 as usize].field_information {
        StructFieldInformation::Declared(fields) => {
            fields[handle.field as usize].signature.0.clone()
        },
        _ => SignatureToken::Bool, // unreachable for well-typed code
    }
}

fn field_inst_type_tok(module: &CompiledModule, idx: FieldInstantiationIndex) -> SignatureToken {
    let inst = &module.field_instantiations[idx.0 as usize];
    let base_ty = field_type_tok(module, inst.handle);
    let type_params = &module.signature_at(inst.type_parameters).0;
    substitute_type_params(&base_ty, type_params)
}

fn variant_field_handle_type_tok(
    module: &CompiledModule,
    idx: move_binary_format::file_format::VariantFieldHandleIndex,
) -> SignatureToken {
    let handle = &module.variant_field_handles[idx.0 as usize];
    match &module.struct_defs[handle.struct_index.0 as usize].field_information {
        StructFieldInformation::DeclaredVariants(variants) => variants[handle.variants[0] as usize]
            .fields[handle.field as usize]
            .signature
            .0
            .clone(),
        _ => SignatureToken::Bool,
    }
}

fn variant_field_inst_type_tok(
    module: &CompiledModule,
    idx: move_binary_format::file_format::VariantFieldInstantiationIndex,
) -> SignatureToken {
    let inst = &module.variant_field_instantiations[idx.0 as usize];
    let base_ty = variant_field_handle_type_tok(module, inst.handle);
    let type_params = &module.signature_at(inst.type_parameters).0;
    substitute_type_params(&base_ty, type_params)
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
        _ => ty.clone(),
    }
}

// ================================================================================================
// Stack effect (for destination hint computation)
// ================================================================================================

/// Compute the stack effect (pops, pushes) of a bytecode instruction.
fn stack_effect(bc: &Bytecode) -> (usize, usize) {
    use Bytecode as B;
    match bc {
        B::LdU8(_)
        | B::LdU16(_)
        | B::LdU32(_)
        | B::LdU64(_)
        | B::LdU128(_)
        | B::LdU256(_)
        | B::LdI8(_)
        | B::LdI16(_)
        | B::LdI32(_)
        | B::LdI64(_)
        | B::LdI128(_)
        | B::LdI256(_)
        | B::LdConst(_)
        | B::LdTrue
        | B::LdFalse => (0, 1),

        B::CopyLoc(_) | B::MoveLoc(_) => (0, 1),
        B::StLoc(_) => (1, 0),

        B::Pop => (1, 0),

        B::Add
        | B::Sub
        | B::Mul
        | B::Div
        | B::Mod
        | B::BitOr
        | B::BitAnd
        | B::Xor
        | B::Shl
        | B::Shr
        | B::Lt
        | B::Gt
        | B::Le
        | B::Ge
        | B::Eq
        | B::Neq
        | B::Or
        | B::And => (2, 1),

        B::CastU8
        | B::CastU16
        | B::CastU32
        | B::CastU64
        | B::CastU128
        | B::CastU256
        | B::CastI8
        | B::CastI16
        | B::CastI32
        | B::CastI64
        | B::CastI128
        | B::CastI256
        | B::Not
        | B::Negate
        | B::FreezeRef => (1, 1),

        B::Pack(_) | B::PackGeneric(_) => (0, 1),
        B::Unpack(_) | B::UnpackGeneric(_) => (1, 0),

        B::PackVariant(_) | B::PackVariantGeneric(_) => (0, 1),
        B::UnpackVariant(_) | B::UnpackVariantGeneric(_) => (1, 0),
        B::TestVariant(_) | B::TestVariantGeneric(_) => (1, 1),

        B::ImmBorrowLoc(_) | B::MutBorrowLoc(_) => (0, 1),
        B::ImmBorrowField(_)
        | B::MutBorrowField(_)
        | B::ImmBorrowFieldGeneric(_)
        | B::MutBorrowFieldGeneric(_)
        | B::ImmBorrowVariantField(_)
        | B::MutBorrowVariantField(_)
        | B::ImmBorrowVariantFieldGeneric(_)
        | B::MutBorrowVariantFieldGeneric(_) => (1, 1),
        B::ReadRef => (1, 1),
        B::WriteRef => (2, 0),

        B::Exists(_) | B::ExistsGeneric(_) => (1, 1),
        B::MoveFrom(_) | B::MoveFromGeneric(_) => (1, 1),
        B::MoveTo(_) | B::MoveToGeneric(_) => (2, 0),
        B::ImmBorrowGlobal(_)
        | B::ImmBorrowGlobalGeneric(_)
        | B::MutBorrowGlobal(_)
        | B::MutBorrowGlobalGeneric(_) => (1, 1),

        B::Call(_) | B::CallGeneric(_) => (0, 0),
        B::PackClosure(_, _) | B::PackClosureGeneric(_, _) => (0, 1),
        B::CallClosure(_) => (0, 0),

        B::VecPack(_, count) => (*count as usize, 1),
        B::VecLen(_) => (1, 1),
        B::VecImmBorrow(_) | B::VecMutBorrow(_) => (2, 1),
        B::VecPushBack(_) => (2, 0),
        B::VecPopBack(_) => (1, 1),
        B::VecUnpack(_, count) => (1, *count as usize),
        B::VecSwap(_) => (3, 0),

        B::Branch(_) => (0, 0),
        B::BrTrue(_) | B::BrFalse(_) => (1, 0),
        B::Ret => (0, 0),
        B::Abort => (1, 0),
        B::AbortMsg => (2, 0),

        B::Nop => (0, 0),
    }
}

/// Return the push count for a given bytecode (for hint tracking).
fn push_count_at(bc: &Bytecode) -> usize {
    stack_effect(bc).1
}
