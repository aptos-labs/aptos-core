// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! V2 conversion pipeline: Intra-block SSA + greedy register allocation.
//!
//! Pass 1: Simulate the operand stack, assigning fresh sequential value IDs
//!         (pure SSA within each basic block). Locals (params + declared locals)
//!         are mutable across blocks and keep their original register indices.
//!
//! Pass 2: Map value IDs to physical registers using liveness-driven reuse
//!         with StLoc look-ahead and CopyLoc/MoveLoc coalescing.

use crate::ir::{BinaryOp, FunctionIR, Instr, Label, ModuleIR, Reg, UnaryOp};
use crate::type_conversion::{convert_sig_token, convert_sig_tokens};
use move_binary_format::{
    access::ModuleAccess,
    file_format::{
        Bytecode, CodeOffset, SignatureToken, StructDefInstantiationIndex,
        StructDefinitionIndex, StructFieldInformation,
    },
    CompiledModule,
};
use move_vm_types::loaded_data::{
    runtime_types::Type,
    struct_name_indexing::StructNameIndex,
};
use std::collections::BTreeMap;

/// Convert an entire compiled module to stackless IR using the v2 pipeline.
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
pub fn convert_module_v2(module: CompiledModule, struct_name_table: &[StructNameIndex]) -> ModuleIR {
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
                let num_pinned = num_params + num_locals;

                let param_toks = &module.signature_at(handle.parameters).0;
                let local_toks = &module.signature_at(code.locals).0;
                let all_toks: Vec<SignatureToken> =
                    param_toks.iter().chain(local_toks.iter()).cloned().collect();
                let local_types = convert_sig_tokens(&module, &all_toks, struct_name_table);

                // Pass 1: Bytecode → Intra-Block SSA
                let mut converter =
                    SsaConverter::new(num_params, num_locals, local_types.clone(), struct_name_table);
                converter.convert_function(&module, &code.code);
                let ssa_instrs = converter.instrs;
                let vid_types = converter.vid_types;

                // Pass 2: Greedy Register Allocation
                let (allocated_instrs, num_regs, reg_types) =
                    allocate_registers(&ssa_instrs, num_pinned, &local_types, &vid_types);

                FunctionIR {
                    name_idx,
                    handle_idx,
                    num_params,
                    num_locals,
                    num_regs,
                    instrs: allocated_instrs,
                    reg_types,
                }
            })
        })
        .collect();

    ModuleIR { module, functions }
}

// ================================================================================================
// Pass 1: Bytecode → Intra-Block SSA
// ================================================================================================

struct SsaConverter<'a> {
    num_pinned: Reg,
    /// Next value ID (starts at num_pinned, resets per block)
    next_vid: Reg,
    /// Simulated operand stack with type information.
    stack: Vec<(Reg, Type)>,
    /// Types of all locals (params ++ declared locals).
    local_types: Vec<Type>,
    /// Types of value IDs (indexed by vid - num_pinned). Set when vid is allocated.
    vid_types: Vec<Type>,
    /// Struct name table for type conversion.
    struct_name_table: &'a [StructNameIndex],
    /// Output instructions
    instrs: Vec<Instr>,
    /// Map from bytecode offset to label
    label_map: BTreeMap<CodeOffset, Label>,
    /// Next label index
    next_label: u16,
}

impl<'a> SsaConverter<'a> {
    fn new(
        num_params: Reg,
        num_locals: Reg,
        local_types: Vec<Type>,
        struct_name_table: &'a [StructNameIndex],
    ) -> Self {
        let num_pinned = num_params + num_locals;
        Self {
            num_pinned,
            next_vid: num_pinned,
            stack: Vec::new(),
            local_types,
            vid_types: Vec::new(),
            struct_name_table,
            instrs: Vec::new(),
            label_map: BTreeMap::new(),
            next_label: 0,
        }
    }

    fn alloc_vid(&mut self, ty: Type) -> Reg {
        let vid = self.next_vid;
        self.next_vid += 1;
        self.vid_types.push(ty);
        vid
    }

    fn push(&mut self, r: Reg, ty: Type) {
        self.stack.push((r, ty));
    }

    fn pop(&mut self) -> (Reg, Type) {
        self.stack.pop().expect("stack underflow")
    }

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

    fn assign_labels(&mut self, code: &[Bytecode]) {
        for (offset, bc) in code.iter().enumerate() {
            match bc {
                Bytecode::Branch(target)
                | Bytecode::BrTrue(target)
                | Bytecode::BrFalse(target) => {
                    self.get_or_create_label(*target);
                },
                _ => {},
            }
            if matches!(bc, Bytecode::BrTrue(_) | Bytecode::BrFalse(_)) {
                self.get_or_create_label((offset + 1) as CodeOffset);
            }
        }
    }

    // --------------------------------------------------------------------------------------------
    // Type helpers
    // --------------------------------------------------------------------------------------------

    fn struct_type(&self, module: &CompiledModule, idx: StructDefinitionIndex) -> Type {
        let def = &module.struct_defs[idx.0 as usize];
        let tok = SignatureToken::Struct(def.struct_handle);
        convert_sig_token(module, &tok, self.struct_name_table)
    }

    fn struct_inst_type(
        &self,
        module: &CompiledModule,
        idx: StructDefInstantiationIndex,
    ) -> Type {
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
    ) -> Type {
        let handle = &module.field_handles[idx.0 as usize];
        let tok = match &module.struct_defs[handle.owner.0 as usize].field_information {
            StructFieldInformation::Declared(fields) => {
                fields[handle.field as usize].signature.0.clone()
            },
            _ => SignatureToken::Bool,
        };
        convert_sig_token(module, &tok, self.struct_name_table)
    }

    fn field_inst_type(
        &self,
        module: &CompiledModule,
        idx: move_binary_format::file_format::FieldInstantiationIndex,
    ) -> Type {
        let inst = &module.field_instantiations[idx.0 as usize];
        let handle = &module.field_handles[inst.handle.0 as usize];
        let base_tok = match &module.struct_defs[handle.owner.0 as usize].field_information {
            StructFieldInformation::Declared(fields) => {
                fields[handle.field as usize].signature.0.clone()
            },
            _ => SignatureToken::Bool,
        };
        let type_params = &module.signature_at(inst.type_parameters).0;
        let tok = substitute_type_params(&base_tok, type_params);
        convert_sig_token(module, &tok, self.struct_name_table)
    }

    fn variant_field_handle_type(
        &self,
        module: &CompiledModule,
        idx: move_binary_format::file_format::VariantFieldHandleIndex,
    ) -> Type {
        let handle = &module.variant_field_handles[idx.0 as usize];
        let tok = match &module.struct_defs[handle.struct_index.0 as usize].field_information {
            StructFieldInformation::DeclaredVariants(variants) => {
                variants[handle.variants[0] as usize].fields[handle.field as usize]
                    .signature
                    .0
                    .clone()
            },
            _ => SignatureToken::Bool,
        };
        convert_sig_token(module, &tok, self.struct_name_table)
    }

    fn variant_field_inst_type(
        &self,
        module: &CompiledModule,
        idx: move_binary_format::file_format::VariantFieldInstantiationIndex,
    ) -> Type {
        let inst = &module.variant_field_instantiations[idx.0 as usize];
        let handle = &module.variant_field_handles[inst.handle.0 as usize];
        let base_tok = match &module.struct_defs[handle.struct_index.0 as usize].field_information {
            StructFieldInformation::DeclaredVariants(variants) => {
                variants[handle.variants[0] as usize].fields[handle.field as usize]
                    .signature
                    .0
                    .clone()
            },
            _ => SignatureToken::Bool,
        };
        let type_params = &module.signature_at(inst.type_parameters).0;
        let tok = substitute_type_params(&base_tok, type_params);
        convert_sig_token(module, &tok, self.struct_name_table)
    }

    // --------------------------------------------------------------------------------------------
    // Function conversion
    // --------------------------------------------------------------------------------------------

    #[allow(clippy::needless_range_loop)]
    fn convert_function(&mut self, module: &CompiledModule, code: &[Bytecode]) {
        self.assign_labels(code);

        // Split into basic blocks
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
            // Reset value ID counter per block (stack is empty at boundaries)
            self.next_vid = self.num_pinned;
            self.stack.clear();

            for offset in start..end {
                if let Some(&label) = self.label_map.get(&(offset as CodeOffset)) {
                    self.instrs.push(Instr::Label(label));
                }
                self.convert_bytecode(module, &code[offset]);
            }
        }
    }

    fn convert_bytecode(&mut self, module: &CompiledModule, bc: &Bytecode) {
        use Bytecode as B;
        match bc {
            // --- Loads ---
            B::LdU8(v) => {
                let ty = Type::U8;
                let d = self.alloc_vid(ty.clone());
                self.instrs.push(Instr::LdU8(d, *v));
                self.push(d, ty);
            },
            B::LdU16(v) => {
                let ty = Type::U16;
                let d = self.alloc_vid(ty.clone());
                self.instrs.push(Instr::LdU16(d, *v));
                self.push(d, ty);
            },
            B::LdU32(v) => {
                let ty = Type::U32;
                let d = self.alloc_vid(ty.clone());
                self.instrs.push(Instr::LdU32(d, *v));
                self.push(d, ty);
            },
            B::LdU64(v) => {
                let ty = Type::U64;
                let d = self.alloc_vid(ty.clone());
                self.instrs.push(Instr::LdU64(d, *v));
                self.push(d, ty);
            },
            B::LdU128(v) => {
                let ty = Type::U128;
                let d = self.alloc_vid(ty.clone());
                self.instrs.push(Instr::LdU128(d, *v));
                self.push(d, ty);
            },
            B::LdU256(v) => {
                let ty = Type::U256;
                let d = self.alloc_vid(ty.clone());
                self.instrs.push(Instr::LdU256(d, *v));
                self.push(d, ty);
            },
            B::LdI8(v) => {
                let ty = Type::I8;
                let d = self.alloc_vid(ty.clone());
                self.instrs.push(Instr::LdI8(d, *v));
                self.push(d, ty);
            },
            B::LdI16(v) => {
                let ty = Type::I16;
                let d = self.alloc_vid(ty.clone());
                self.instrs.push(Instr::LdI16(d, *v));
                self.push(d, ty);
            },
            B::LdI32(v) => {
                let ty = Type::I32;
                let d = self.alloc_vid(ty.clone());
                self.instrs.push(Instr::LdI32(d, *v));
                self.push(d, ty);
            },
            B::LdI64(v) => {
                let ty = Type::I64;
                let d = self.alloc_vid(ty.clone());
                self.instrs.push(Instr::LdI64(d, *v));
                self.push(d, ty);
            },
            B::LdI128(v) => {
                let ty = Type::I128;
                let d = self.alloc_vid(ty.clone());
                self.instrs.push(Instr::LdI128(d, *v));
                self.push(d, ty);
            },
            B::LdI256(v) => {
                let ty = Type::I256;
                let d = self.alloc_vid(ty.clone());
                self.instrs.push(Instr::LdI256(d, *v));
                self.push(d, ty);
            },
            B::LdConst(idx) => {
                let tok = &module.constant_pool[idx.0 as usize].type_;
                let ty = convert_sig_token(module, tok, self.struct_name_table);
                let d = self.alloc_vid(ty.clone());
                self.instrs.push(Instr::LdConst(d, *idx));
                self.push(d, ty);
            },
            B::LdTrue => {
                let ty = Type::Bool;
                let d = self.alloc_vid(ty.clone());
                self.instrs.push(Instr::LdTrue(d));
                self.push(d, ty);
            },
            B::LdFalse => {
                let ty = Type::Bool;
                let d = self.alloc_vid(ty.clone());
                self.instrs.push(Instr::LdFalse(d));
                self.push(d, ty);
            },

            // --- Locals ---
            B::CopyLoc(idx) => {
                let src = *idx as Reg;
                let ty = self.local_types[*idx as usize].clone();
                let d = self.alloc_vid(ty.clone());
                self.instrs.push(Instr::Copy(d, src));
                self.push(d, ty);
            },
            B::MoveLoc(idx) => {
                let src = *idx as Reg;
                let ty = self.local_types[*idx as usize].clone();
                let d = self.alloc_vid(ty.clone());
                self.instrs.push(Instr::Move(d, src));
                self.push(d, ty);
            },
            B::StLoc(_idx) => {
                let (src, _ty) = self.pop();
                let dst = *_idx as Reg;
                self.instrs.push(Instr::Move(dst, src));
            },

            // --- Pop ---
            B::Pop => {
                let _ = self.pop();
            },

            // --- Binary ops ---
            B::Add => self.convert_binop(BinaryOp::Add),
            B::Sub => self.convert_binop(BinaryOp::Sub),
            B::Mul => self.convert_binop(BinaryOp::Mul),
            B::Div => self.convert_binop(BinaryOp::Div),
            B::Mod => self.convert_binop(BinaryOp::Mod),
            B::BitOr => self.convert_binop(BinaryOp::BitOr),
            B::BitAnd => self.convert_binop(BinaryOp::BitAnd),
            B::Xor => self.convert_binop(BinaryOp::Xor),
            B::Shl => self.convert_binop(BinaryOp::Shl),
            B::Shr => self.convert_binop(BinaryOp::Shr),
            B::Lt => self.convert_binop(BinaryOp::Lt),
            B::Gt => self.convert_binop(BinaryOp::Gt),
            B::Le => self.convert_binop(BinaryOp::Le),
            B::Ge => self.convert_binop(BinaryOp::Ge),
            B::Eq => self.convert_binop(BinaryOp::Eq),
            B::Neq => self.convert_binop(BinaryOp::Neq),
            B::Or => self.convert_binop(BinaryOp::Or),
            B::And => self.convert_binop(BinaryOp::And),

            // --- Unary ops ---
            B::CastU8 => self.convert_unop(UnaryOp::CastU8),
            B::CastU16 => self.convert_unop(UnaryOp::CastU16),
            B::CastU32 => self.convert_unop(UnaryOp::CastU32),
            B::CastU64 => self.convert_unop(UnaryOp::CastU64),
            B::CastU128 => self.convert_unop(UnaryOp::CastU128),
            B::CastU256 => self.convert_unop(UnaryOp::CastU256),
            B::CastI8 => self.convert_unop(UnaryOp::CastI8),
            B::CastI16 => self.convert_unop(UnaryOp::CastI16),
            B::CastI32 => self.convert_unop(UnaryOp::CastI32),
            B::CastI64 => self.convert_unop(UnaryOp::CastI64),
            B::CastI128 => self.convert_unop(UnaryOp::CastI128),
            B::CastI256 => self.convert_unop(UnaryOp::CastI256),
            B::Not => self.convert_unop(UnaryOp::Not),
            B::Negate => self.convert_unop(UnaryOp::Negate),
            B::FreezeRef => self.convert_unop(UnaryOp::FreezeRef),

            // --- Struct ops ---
            B::Pack(idx) => {
                let n = struct_field_count(module, *idx);
                let fields_typed = self.pop_n_reverse(n);
                let fields: Vec<Reg> = fields_typed.iter().map(|(r, _)| *r).collect();
                let result_ty = self.struct_type(module, *idx);
                let d = self.alloc_vid(result_ty.clone());
                self.instrs.push(Instr::Pack(d, *idx, fields));
                self.push(d, result_ty);
            },
            B::PackGeneric(idx) => {
                let inst = &module.struct_def_instantiations[idx.0 as usize];
                let n = struct_field_count(module, inst.def);
                let fields_typed = self.pop_n_reverse(n);
                let fields: Vec<Reg> = fields_typed.iter().map(|(r, _)| *r).collect();
                let result_ty = self.struct_inst_type(module, *idx);
                let d = self.alloc_vid(result_ty.clone());
                self.instrs.push(Instr::PackGeneric(d, *idx, fields));
                self.push(d, result_ty);
            },
            B::Unpack(idx) => {
                let (src, _src_ty) = self.pop();
                let n = struct_field_count(module, *idx);
                let ftypes = struct_field_type_toks(module, *idx);
                let ftypes: Vec<Type> =
                    convert_sig_tokens(module, &ftypes, self.struct_name_table);
                let mut dsts = Vec::with_capacity(n);
                for i in 0..n {
                    let fty = ftypes.get(i).cloned().unwrap_or(Type::Bool);
                    dsts.push(self.alloc_vid(fty));
                }
                self.instrs.push(Instr::Unpack(dsts.clone(), *idx, src));
                for (d, fty) in dsts.into_iter().zip(ftypes) {
                    self.push(d, fty);
                }
            },
            B::UnpackGeneric(idx) => {
                let (src, _src_ty) = self.pop();
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
                    let fty = ftypes.get(i).cloned().unwrap_or(Type::Bool);
                    dsts.push(self.alloc_vid(fty));
                }
                self.instrs.push(Instr::UnpackGeneric(dsts.clone(), *idx, src));
                for (d, fty) in dsts.into_iter().zip(ftypes) {
                    self.push(d, fty);
                }
            },

            // --- Variant ops ---
            B::PackVariant(idx) => {
                let handle = &module.struct_variant_handles[idx.0 as usize];
                let n = variant_field_count(module, handle.struct_index, handle.variant);
                let fields_typed = self.pop_n_reverse(n);
                let fields: Vec<Reg> = fields_typed.iter().map(|(r, _)| *r).collect();
                let result_ty = self.struct_type(module, handle.struct_index);
                let d = self.alloc_vid(result_ty.clone());
                self.instrs.push(Instr::PackVariant(d, *idx, fields));
                self.push(d, result_ty);
            },
            B::PackVariantGeneric(idx) => {
                let inst = &module.struct_variant_instantiations[idx.0 as usize];
                let handle = &module.struct_variant_handles[inst.handle.0 as usize];
                let n = variant_field_count(module, handle.struct_index, handle.variant);
                let fields_typed = self.pop_n_reverse(n);
                let fields: Vec<Reg> = fields_typed.iter().map(|(r, _)| *r).collect();
                let type_params = module.signature_at(inst.type_parameters).0.clone();
                let def = &module.struct_defs[handle.struct_index.0 as usize];
                let tok = SignatureToken::StructInstantiation(def.struct_handle, type_params);
                let result_ty = convert_sig_token(module, &tok, self.struct_name_table);
                let d = self.alloc_vid(result_ty.clone());
                self.instrs.push(Instr::PackVariantGeneric(d, *idx, fields));
                self.push(d, result_ty);
            },
            B::UnpackVariant(idx) => {
                let (src, _src_ty) = self.pop();
                let handle = &module.struct_variant_handles[idx.0 as usize];
                let n = variant_field_count(module, handle.struct_index, handle.variant);
                let ftypes_tok = variant_field_type_toks(module, handle.struct_index, handle.variant);
                let ftypes: Vec<Type> =
                    convert_sig_tokens(module, &ftypes_tok, self.struct_name_table);
                let mut dsts = Vec::with_capacity(n);
                for i in 0..n {
                    let fty = ftypes.get(i).cloned().unwrap_or(Type::Bool);
                    dsts.push(self.alloc_vid(fty));
                }
                self.instrs.push(Instr::UnpackVariant(dsts.clone(), *idx, src));
                for (d, fty) in dsts.into_iter().zip(ftypes) {
                    self.push(d, fty);
                }
            },
            B::UnpackVariantGeneric(idx) => {
                let (src, _src_ty) = self.pop();
                let inst = &module.struct_variant_instantiations[idx.0 as usize];
                let handle = &module.struct_variant_handles[inst.handle.0 as usize];
                let n = variant_field_count(module, handle.struct_index, handle.variant);
                let type_params = module.signature_at(inst.type_parameters).0.clone();
                let raw_ftypes = variant_field_type_toks(module, handle.struct_index, handle.variant);
                let ftypes_tok: Vec<SignatureToken> = raw_ftypes
                    .iter()
                    .map(|ft| substitute_type_params(ft, &type_params))
                    .collect();
                let ftypes: Vec<Type> =
                    convert_sig_tokens(module, &ftypes_tok, self.struct_name_table);
                let mut dsts = Vec::with_capacity(n);
                for i in 0..n {
                    let fty = ftypes.get(i).cloned().unwrap_or(Type::Bool);
                    dsts.push(self.alloc_vid(fty));
                }
                self.instrs
                    .push(Instr::UnpackVariantGeneric(dsts.clone(), *idx, src));
                for (d, fty) in dsts.into_iter().zip(ftypes) {
                    self.push(d, fty);
                }
            },
            B::TestVariant(idx) => {
                let (src, _src_ty) = self.pop();
                let ty = Type::Bool;
                let d = self.alloc_vid(ty.clone());
                self.instrs.push(Instr::TestVariant(d, *idx, src));
                self.push(d, ty);
            },
            B::TestVariantGeneric(idx) => {
                let (src, _src_ty) = self.pop();
                let ty = Type::Bool;
                let d = self.alloc_vid(ty.clone());
                self.instrs.push(Instr::TestVariantGeneric(d, *idx, src));
                self.push(d, ty);
            },

            // --- References ---
            B::ImmBorrowLoc(idx) => {
                let src = *idx as Reg;
                let inner = self.local_types[*idx as usize].clone();
                let ty = Type::Reference(Box::new(inner));
                let d = self.alloc_vid(ty.clone());
                self.instrs.push(Instr::ImmBorrowLoc(d, src));
                self.push(d, ty);
            },
            B::MutBorrowLoc(idx) => {
                let src = *idx as Reg;
                let inner = self.local_types[*idx as usize].clone();
                let ty = Type::MutableReference(Box::new(inner));
                let d = self.alloc_vid(ty.clone());
                self.instrs.push(Instr::MutBorrowLoc(d, src));
                self.push(d, ty);
            },
            B::ImmBorrowField(idx) => {
                let (src, _src_ty) = self.pop();
                let fty = self.field_type(module, *idx);
                let ty = Type::Reference(Box::new(fty));
                let d = self.alloc_vid(ty.clone());
                self.instrs.push(Instr::ImmBorrowField(d, *idx, src));
                self.push(d, ty);
            },
            B::MutBorrowField(idx) => {
                let (src, _src_ty) = self.pop();
                let fty = self.field_type(module, *idx);
                let ty = Type::MutableReference(Box::new(fty));
                let d = self.alloc_vid(ty.clone());
                self.instrs.push(Instr::MutBorrowField(d, *idx, src));
                self.push(d, ty);
            },
            B::ImmBorrowFieldGeneric(idx) => {
                let (src, _src_ty) = self.pop();
                let fty = self.field_inst_type(module, *idx);
                let ty = Type::Reference(Box::new(fty));
                let d = self.alloc_vid(ty.clone());
                self.instrs.push(Instr::ImmBorrowFieldGeneric(d, *idx, src));
                self.push(d, ty);
            },
            B::MutBorrowFieldGeneric(idx) => {
                let (src, _src_ty) = self.pop();
                let fty = self.field_inst_type(module, *idx);
                let ty = Type::MutableReference(Box::new(fty));
                let d = self.alloc_vid(ty.clone());
                self.instrs.push(Instr::MutBorrowFieldGeneric(d, *idx, src));
                self.push(d, ty);
            },
            B::ImmBorrowVariantField(idx) => {
                let (src, _src_ty) = self.pop();
                let fty = self.variant_field_handle_type(module, *idx);
                let ty = Type::Reference(Box::new(fty));
                let d = self.alloc_vid(ty.clone());
                self.instrs.push(Instr::ImmBorrowVariantField(d, *idx, src));
                self.push(d, ty);
            },
            B::MutBorrowVariantField(idx) => {
                let (src, _src_ty) = self.pop();
                let fty = self.variant_field_handle_type(module, *idx);
                let ty = Type::MutableReference(Box::new(fty));
                let d = self.alloc_vid(ty.clone());
                self.instrs.push(Instr::MutBorrowVariantField(d, *idx, src));
                self.push(d, ty);
            },
            B::ImmBorrowVariantFieldGeneric(idx) => {
                let (src, _src_ty) = self.pop();
                let fty = self.variant_field_inst_type(module, *idx);
                let ty = Type::Reference(Box::new(fty));
                let d = self.alloc_vid(ty.clone());
                self.instrs
                    .push(Instr::ImmBorrowVariantFieldGeneric(d, *idx, src));
                self.push(d, ty);
            },
            B::MutBorrowVariantFieldGeneric(idx) => {
                let (src, _src_ty) = self.pop();
                let fty = self.variant_field_inst_type(module, *idx);
                let ty = Type::MutableReference(Box::new(fty));
                let d = self.alloc_vid(ty.clone());
                self.instrs
                    .push(Instr::MutBorrowVariantFieldGeneric(d, *idx, src));
                self.push(d, ty);
            },
            B::ReadRef => {
                let (src, src_ty) = self.pop();
                let ty = match &src_ty {
                    Type::Reference(inner) | Type::MutableReference(inner) => (**inner).clone(),
                    other => other.clone(),
                };
                let d = self.alloc_vid(ty.clone());
                self.instrs.push(Instr::ReadRef(d, src));
                self.push(d, ty);
            },
            B::WriteRef => {
                let (ref_r, _ref_ty) = self.pop();
                let (val, _val_ty) = self.pop();
                self.instrs.push(Instr::WriteRef(ref_r, val));
            },

            // --- Globals ---
            B::Exists(idx) => {
                let (addr, _addr_ty) = self.pop();
                let ty = Type::Bool;
                let d = self.alloc_vid(ty.clone());
                self.instrs.push(Instr::Exists(d, *idx, addr));
                self.push(d, ty);
            },
            B::ExistsGeneric(idx) => {
                let (addr, _addr_ty) = self.pop();
                let ty = Type::Bool;
                let d = self.alloc_vid(ty.clone());
                self.instrs.push(Instr::ExistsGeneric(d, *idx, addr));
                self.push(d, ty);
            },
            B::MoveFrom(idx) => {
                let (addr, _addr_ty) = self.pop();
                let ty = self.struct_type(module, *idx);
                let d = self.alloc_vid(ty.clone());
                self.instrs.push(Instr::MoveFrom(d, *idx, addr));
                self.push(d, ty);
            },
            B::MoveFromGeneric(idx) => {
                let (addr, _addr_ty) = self.pop();
                let ty = self.struct_inst_type(module, *idx);
                let d = self.alloc_vid(ty.clone());
                self.instrs.push(Instr::MoveFromGeneric(d, *idx, addr));
                self.push(d, ty);
            },
            B::MoveTo(idx) => {
                let (val, _val_ty) = self.pop();
                let (signer, _signer_ty) = self.pop();
                self.instrs.push(Instr::MoveTo(*idx, signer, val));
            },
            B::MoveToGeneric(idx) => {
                let (val, _val_ty) = self.pop();
                let (signer, _signer_ty) = self.pop();
                self.instrs.push(Instr::MoveToGeneric(*idx, signer, val));
            },
            B::ImmBorrowGlobal(idx) => {
                let (addr, _addr_ty) = self.pop();
                let ty = Type::Reference(Box::new(self.struct_type(module, *idx)));
                let d = self.alloc_vid(ty.clone());
                self.instrs.push(Instr::ImmBorrowGlobal(d, *idx, addr));
                self.push(d, ty);
            },
            B::ImmBorrowGlobalGeneric(idx) => {
                let (addr, _addr_ty) = self.pop();
                let ty = Type::Reference(Box::new(self.struct_inst_type(module, *idx)));
                let d = self.alloc_vid(ty.clone());
                self.instrs
                    .push(Instr::ImmBorrowGlobalGeneric(d, *idx, addr));
                self.push(d, ty);
            },
            B::MutBorrowGlobal(idx) => {
                let (addr, _addr_ty) = self.pop();
                let ty = Type::MutableReference(Box::new(self.struct_type(module, *idx)));
                let d = self.alloc_vid(ty.clone());
                self.instrs.push(Instr::MutBorrowGlobal(d, *idx, addr));
                self.push(d, ty);
            },
            B::MutBorrowGlobalGeneric(idx) => {
                let (addr, _addr_ty) = self.pop();
                let ty = Type::MutableReference(Box::new(self.struct_inst_type(module, *idx)));
                let d = self.alloc_vid(ty.clone());
                self.instrs
                    .push(Instr::MutBorrowGlobalGeneric(d, *idx, addr));
                self.push(d, ty);
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
                for rty in &ret_types {
                    rets.push(self.alloc_vid(rty.clone()));
                }
                self.instrs.push(Instr::Call(rets.clone(), *idx, args));
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
                for rty in &ret_types {
                    rets.push(self.alloc_vid(rty.clone()));
                }
                self.instrs.push(Instr::CallGeneric(rets.clone(), *idx, args));
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
                let ty = convert_sig_token(module, &tok, self.struct_name_table);
                let d = self.alloc_vid(ty.clone());
                self.instrs
                    .push(Instr::PackClosure(d, *fhi, *mask, captured));
                self.push(d, ty);
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
                let ty = convert_sig_token(module, &tok, self.struct_name_table);
                let d = self.alloc_vid(ty.clone());
                self.instrs
                    .push(Instr::PackClosureGeneric(d, *fii, *mask, captured));
                self.push(d, ty);
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
                let (closure, _closure_ty) = self.pop();
                let args_typed = self.pop_n_reverse(num_args);
                let mut all_args: Vec<Reg> = args_typed.iter().map(|(r, _)| *r).collect();
                all_args.push(closure);
                let mut rets = Vec::with_capacity(ret_types.len());
                for rty in &ret_types {
                    rets.push(self.alloc_vid(rty.clone()));
                }
                self.instrs
                    .push(Instr::CallClosure(rets.clone(), *sig_idx, all_args));
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
                let ty = Type::Vector(triomphe::Arc::new(elem_ty));
                let d = self.alloc_vid(ty.clone());
                self.instrs.push(Instr::VecPack(d, *sig_idx, *count, elems));
                self.push(d, ty);
            },
            B::VecLen(sig_idx) => {
                let (vec_ref, _vec_ty) = self.pop();
                let ty = Type::U64;
                let d = self.alloc_vid(ty.clone());
                self.instrs.push(Instr::VecLen(d, *sig_idx, vec_ref));
                self.push(d, ty);
            },
            B::VecImmBorrow(sig_idx) => {
                let (idx_r, _idx_ty) = self.pop();
                let (vec_ref, _vec_ty) = self.pop();
                let elem_tok = &module.signature_at(*sig_idx).0[0];
                let elem_ty = convert_sig_token(module, elem_tok, self.struct_name_table);
                let ty = Type::Reference(Box::new(elem_ty));
                let d = self.alloc_vid(ty.clone());
                self.instrs
                    .push(Instr::VecImmBorrow(d, *sig_idx, vec_ref, idx_r));
                self.push(d, ty);
            },
            B::VecMutBorrow(sig_idx) => {
                let (idx_r, _idx_ty) = self.pop();
                let (vec_ref, _vec_ty) = self.pop();
                let elem_tok = &module.signature_at(*sig_idx).0[0];
                let elem_ty = convert_sig_token(module, elem_tok, self.struct_name_table);
                let ty = Type::MutableReference(Box::new(elem_ty));
                let d = self.alloc_vid(ty.clone());
                self.instrs
                    .push(Instr::VecMutBorrow(d, *sig_idx, vec_ref, idx_r));
                self.push(d, ty);
            },
            B::VecPushBack(sig_idx) => {
                let (val, _val_ty) = self.pop();
                let (vec_ref, _vec_ty) = self.pop();
                self.instrs.push(Instr::VecPushBack(*sig_idx, vec_ref, val));
            },
            B::VecPopBack(sig_idx) => {
                let (vec_ref, _vec_ty) = self.pop();
                let elem_tok = &module.signature_at(*sig_idx).0[0];
                let ty = convert_sig_token(module, elem_tok, self.struct_name_table);
                let d = self.alloc_vid(ty.clone());
                self.instrs.push(Instr::VecPopBack(d, *sig_idx, vec_ref));
                self.push(d, ty);
            },
            B::VecUnpack(sig_idx, count) => {
                let (src, _src_ty) = self.pop();
                let elem_tok = &module.signature_at(*sig_idx).0[0];
                let elem_ty = convert_sig_token(module, elem_tok, self.struct_name_table);
                let mut dsts = Vec::with_capacity(*count as usize);
                for _ in 0..*count {
                    dsts.push(self.alloc_vid(elem_ty.clone()));
                }
                self.instrs
                    .push(Instr::VecUnpack(dsts.clone(), *sig_idx, *count, src));
                for d in dsts {
                    self.push(d, elem_ty.clone());
                }
            },
            B::VecSwap(sig_idx) => {
                let (j, _j_ty) = self.pop();
                let (i, _i_ty) = self.pop();
                let (vec_ref, _vec_ty) = self.pop();
                self.instrs.push(Instr::VecSwap(*sig_idx, vec_ref, i, j));
            },

            // --- Control flow ---
            B::Branch(target) => {
                let label = self.label_map[target];
                self.instrs.push(Instr::Branch(label));
            },
            B::BrTrue(target) => {
                let (cond, _cond_ty) = self.pop();
                let label = self.label_map[target];
                self.instrs.push(Instr::BrTrue(label, cond));
            },
            B::BrFalse(target) => {
                let (cond, _cond_ty) = self.pop();
                let label = self.label_map[target];
                self.instrs.push(Instr::BrFalse(label, cond));
            },
            B::Ret => {
                let rets: Vec<Reg> = self.stack.drain(..).map(|(r, _)| r).collect();
                self.instrs.push(Instr::Ret(rets));
            },
            B::Abort => {
                let (code, _code_ty) = self.pop();
                self.instrs.push(Instr::Abort(code));
            },
            B::AbortMsg => {
                let (msg, _msg_ty) = self.pop();
                let (code, _code_ty) = self.pop();
                self.instrs.push(Instr::AbortMsg(code, msg));
            },

            B::Nop => {},
        }
    }

    fn convert_binop(&mut self, op: BinaryOp) {
        let (rhs, _rhs_ty) = self.pop();
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
            _ => lhs_ty,
        };
        let d = self.alloc_vid(result_ty.clone());
        self.instrs.push(Instr::BinaryOp(d, op, lhs, rhs));
        self.push(d, result_ty);
    }

    fn convert_unop(&mut self, op: UnaryOp) {
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
        let d = self.alloc_vid(result_ty.clone());
        self.instrs.push(Instr::UnaryOp(d, op, src));
        self.push(d, result_ty);
    }
}

// ================================================================================================
// Pass 2: Greedy Register Allocation (per block)
// ================================================================================================

use crate::optimize_v1::{get_defs_uses, split_into_blocks};

fn allocate_registers(
    instrs: &[Instr],
    num_pinned: Reg,
    local_types: &[Type],
    vid_types: &[Type],
) -> (Vec<Instr>, Reg, Vec<Type>) {
    let blocks = split_into_blocks(instrs);
    let mut result = Vec::with_capacity(instrs.len());
    let mut global_next_reg = num_pinned;
    // The free pool carries across block boundaries, keyed by type.
    let mut free_pool: BTreeMap<Type, Vec<Reg>> = BTreeMap::new();
    // Map physical register -> type.
    let mut phys_reg_types: BTreeMap<Reg, Type> = BTreeMap::new();
    // Local types are known ahead of time.
    for (i, ty) in local_types.iter().enumerate() {
        phys_reg_types.insert(i as Reg, ty.clone());
    }

    for (start, end) in blocks {
        let block_instrs = &instrs[start..end];
        let (allocated, block_max, returned_pool) =
            allocate_block(
                block_instrs,
                num_pinned,
                global_next_reg,
                free_pool,
                vid_types,
                &mut phys_reg_types,
            );
        free_pool = returned_pool;
        if block_max > global_next_reg {
            global_next_reg = block_max;
        }
        result.extend(allocated);
    }

    // Build reg_types from physical register -> type mapping.
    let mut reg_types = Vec::with_capacity(global_next_reg as usize);
    for i in 0..global_next_reg {
        reg_types.push(
            phys_reg_types
                .get(&i)
                .cloned()
                .unwrap_or(Type::Bool),
        );
    }

    (result, global_next_reg, reg_types)
}

fn vid_type(vid: Reg, num_pinned: Reg, vid_types: &[Type]) -> Type {
    if vid < num_pinned {
        // This shouldn't happen for temp vid lookups, but fallback.
        Type::Bool
    } else {
        vid_types
            .get((vid - num_pinned) as usize)
            .cloned()
            .unwrap_or(Type::Bool)
    }
}

fn allocate_block(
    instrs: &[Instr],
    num_pinned: Reg,
    start_reg: Reg,
    carry_pool: BTreeMap<Type, Vec<Reg>>,
    vid_types: &[Type],
    phys_reg_types: &mut BTreeMap<Reg, Type>,
) -> (Vec<Instr>, Reg, BTreeMap<Type, Vec<Reg>>) {
    if instrs.is_empty() {
        return (Vec::new(), start_reg, carry_pool);
    }

    // Step 1: Backward scan to compute last_use[vid] = instruction offset of final use
    let mut last_use: BTreeMap<Reg, usize> = BTreeMap::new();
    for (i, instr) in instrs.iter().enumerate() {
        let (defs, uses) = get_defs_uses(instr);
        for r in uses {
            if r >= num_pinned {
                last_use.insert(r, i);
            }
        }
        for r in defs {
            if r >= num_pinned {
                last_use.entry(r).or_insert(i);
            }
        }
    }

    // Step 2: Forward scan with type-keyed free-register pool
    let mut vid_to_phys: BTreeMap<Reg, Reg> = BTreeMap::new();
    // Pinned registers map to themselves
    for r in 0..num_pinned {
        vid_to_phys.insert(r, r);
    }
    let mut free_pool = carry_pool;
    let mut next_reg = start_reg;

    // Pre-scan for StLoc look-ahead
    let mut stloc_target: BTreeMap<Reg, Reg> = BTreeMap::new();
    for (i, instr) in instrs.iter().enumerate() {
        if let Instr::Move(dst, src) = instr
            && *dst < num_pinned
            && *src >= num_pinned
            && last_use.get(src) == Some(&i)
        {
            stloc_target.insert(*src, *dst);
        }
    }

    // CopyLoc/MoveLoc coalescing
    let mut coalesce_to_local: BTreeMap<Reg, Reg> = BTreeMap::new();
    for (i, instr) in instrs.iter().enumerate() {
        match instr {
            Instr::Copy(dst, src) | Instr::Move(dst, src)
                if *dst >= num_pinned && *src < num_pinned =>
            {
                let vid = *dst;
                if let Some(&lu) = last_use.get(&vid)
                    && lu > i
                    && !stloc_target.contains_key(&vid)
                {
                    let use_count: usize = instrs[i + 1..=lu]
                        .iter()
                        .map(|ins| {
                            let (_, u) = get_defs_uses(ins);
                            u.iter().filter(|&&r| r == vid).count()
                        })
                        .sum();
                    if use_count == 1 {
                        let local = *src;
                        let local_redefined = instrs[i + 1..lu].iter().any(|ins| {
                            let (d, _) = get_defs_uses(ins);
                            d.contains(&local)
                        });
                        if !local_redefined {
                            coalesce_to_local.insert(vid, local);
                        }
                    }
                }
            },
            _ => {},
        }
    }

    let mut output = Vec::with_capacity(instrs.len());

    for (i, instr) in instrs.iter().enumerate() {
        let mut mapped_instr = instr.clone();
        let (defs, _) = get_defs_uses(instr);

        // Allocate physical registers for destination vids
        for d in &defs {
            if *d >= num_pinned && !vid_to_phys.contains_key(d) {
                if let Some(&local_r) = stloc_target.get(d) {
                    vid_to_phys.insert(*d, local_r);
                } else if let Some(&local_r) = coalesce_to_local.get(d) {
                    vid_to_phys.insert(*d, local_r);
                } else {
                    let ty = vid_type(*d, num_pinned, vid_types);
                    // Try to find a free register of the same type.
                    let phys = if let Some(regs) = free_pool.get_mut(&ty) {
                        regs.pop()
                    } else {
                        None
                    };
                    let phys = phys.unwrap_or_else(|| {
                        let r = next_reg;
                        next_reg += 1;
                        phys_reg_types.insert(r, ty);
                        r
                    });
                    vid_to_phys.insert(*d, phys);
                }
            }
        }

        apply_mapping(&mut mapped_instr, &vid_to_phys);
        output.push(mapped_instr);

        // Free registers for vids that reach their last use at this instruction
        let (_, uses) = get_defs_uses(instr);
        for r in uses {
            if r >= num_pinned
                && last_use.get(&r) == Some(&i)
                && let Some(&phys) = vid_to_phys.get(&r)
                && phys >= num_pinned
            {
                let ty = phys_reg_types
                    .get(&phys)
                    .cloned()
                    .unwrap_or(Type::Bool);
                free_pool.entry(ty).or_default().push(phys);
            }
        }
        for d in &defs {
            if *d >= num_pinned
                && last_use.get(d) == Some(&i)
            {
                let (_, ref uses_list) = get_defs_uses(instr);
                if !uses_list.contains(d)
                    && let Some(&phys) = vid_to_phys.get(d)
                    && phys >= num_pinned
                {
                    let ty = phys_reg_types
                        .get(&phys)
                        .cloned()
                        .unwrap_or(Type::Bool);
                    free_pool.entry(ty).or_default().push(phys);
                }
            }
        }
    }

    (output, next_reg, free_pool)
}

/// Apply vid-to-physical-register mapping to an instruction.
fn apply_mapping(instr: &mut Instr, map: &BTreeMap<Reg, Reg>) {
    crate::optimize_v1::rename_instr(instr, map);
}

// ================================================================================================
// Type/field helpers
// ================================================================================================

fn struct_field_count(module: &CompiledModule, idx: StructDefinitionIndex) -> usize {
    match &module.struct_defs[idx.0 as usize].field_information {
        StructFieldInformation::Native => 0,
        StructFieldInformation::Declared(fields) => fields.len(),
        StructFieldInformation::DeclaredVariants(_) => 0,
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
        _ => vec![],
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
        StructFieldInformation::DeclaredVariants(variants) => {
            variants[variant as usize]
                .fields
                .iter()
                .map(|f| f.signature.0.clone())
                .collect()
        },
        _ => vec![],
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
            let new_tps: Vec<_> = tps.iter().map(|p| substitute_type_params(p, params)).collect();
            SignatureToken::StructInstantiation(*handle, new_tps)
        },
        _ => ty.clone(),
    }
}
