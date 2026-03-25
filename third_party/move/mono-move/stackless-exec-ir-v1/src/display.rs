// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Human-readable display for the stackless execution IR.
//! Resolves pool indices using the CompiledModule for readable output.

use crate::ir::{BinaryOp, FunctionIR, ImmValue, Instr, ModuleIR, Reg, UnaryOp};
use move_binary_format::{
    access::ModuleAccess,
    file_format::{
        FieldHandleIndex, FieldInstantiationIndex, FunctionHandleIndex, FunctionInstantiationIndex,
        SignatureToken, StructDefInstantiationIndex, StructDefinitionIndex, StructHandleIndex,
        StructVariantHandleIndex, StructVariantInstantiationIndex, VariantFieldHandleIndex,
        VariantFieldInstantiationIndex,
    },
    CompiledModule,
};
use move_vm_types::loaded_data::runtime_types::Type;
use std::fmt;

/// A display wrapper for ModuleIR.
pub struct ModuleIRDisplay<'a> {
    module_ir: &'a ModuleIR,
}

impl ModuleIR {
    pub fn display(&self) -> ModuleIRDisplay<'_> {
        ModuleIRDisplay { module_ir: self }
    }
}

impl fmt::Display for ModuleIRDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let module = &self.module_ir.module;
        let self_handle = module.module_handle_at(module.self_module_handle_idx);
        let addr = module.address_identifier_at(self_handle.address);
        let name = module.identifier_at(self_handle.name);
        writeln!(
            f,
            "=== Module 0x{}::{} ===",
            addr.short_str_lossless(),
            name
        )?;

        for func_ir in &self.module_ir.functions {
            writeln!(f)?;
            display_function(f, module, func_ir)?;
        }
        Ok(())
    }
}

fn display_function(
    f: &mut fmt::Formatter<'_>,
    module: &CompiledModule,
    func: &FunctionIR,
) -> fmt::Result {
    let handle = module.function_handle_at(func.handle_idx);
    let name = module.identifier_at(func.name_idx);

    // Function signature
    let params = &module.signature_at(handle.parameters).0;
    let returns = &module.signature_at(handle.return_).0;

    write!(f, "fun {}(", name)?;
    for (i, _param_ty) in params.iter().enumerate() {
        if i > 0 {
            write!(f, ", ")?;
        }
        write!(f, "{}", reg(Reg::Home(i as u16)))?;
    }
    write!(f, ")")?;
    if !returns.is_empty() {
        write!(f, ": ")?;
        for (i, ret_ty) in returns.iter().enumerate() {
            if i > 0 {
                write!(f, " * ")?;
            }
            display_sig_token(f, module, ret_ty)?;
        }
    }
    writeln!(f, " {{")?;
    let num_temps = func.num_regs - func.num_params - func.num_locals;
    if func.num_arg_regs > 0 {
        writeln!(
            f,
            "  slots: params({}), locals({}), temps({}), args({})",
            func.num_params, func.num_locals, num_temps, func.num_arg_regs
        )?;
    } else {
        writeln!(
            f,
            "  slots: params({}), locals({}), temps({})",
            func.num_params, func.num_locals, num_temps
        )?;
    }

    // Display register declarations with types
    for i in 0..func.num_regs {
        let ty = &func.reg_types[i as usize];
        write!(f, "    r{}: ", i)?;
        display_type(f, module, ty)?;
        writeln!(f)?;
    }
    writeln!(f, "  code:")?;

    // Instructions
    let mut instr_num = 0;
    for instr in &func.instrs {
        if let Instr::Label(label) = instr {
            writeln!(f, "  L{}:", label.0)?;
            continue;
        }
        write!(f, "    {}: ", instr_num)?;
        display_instr(f, module, instr)?;
        writeln!(f)?;
        instr_num += 1;
    }

    writeln!(f, "}}")?;
    Ok(())
}

fn reg(r: Reg) -> String {
    match r {
        Reg::Home(i) => format!("r{}", i),
        Reg::Arg(i) => format!("a{}", i),
    }
}

fn regs(rs: &[Reg]) -> String {
    let parts: Vec<String> = rs.iter().map(|r| reg(*r)).collect();
    format!("[{}]", parts.join(", "))
}

/// Format a destination: single register bare, multiple in brackets.
fn dst(r: Reg) -> String {
    reg(r)
}

/// Format multiple destinations in brackets.
fn dsts(rs: &[Reg]) -> String {
    regs(rs)
}

/// Write `dest := ` prefix for a single destination register.
fn write_dst(f: &mut fmt::Formatter<'_>, d: Reg) -> fmt::Result {
    write!(f, "{} := ", dst(d))
}

/// Write `[dests] := ` prefix for multiple destination registers.
fn write_dsts(f: &mut fmt::Formatter<'_>, ds: &[Reg]) -> fmt::Result {
    write!(f, "{} := ", dsts(ds))
}

fn struct_name(module: &CompiledModule, idx: StructDefinitionIndex) -> String {
    let def = &module.struct_defs[idx.0 as usize];
    let handle = module.struct_handle_at(def.struct_handle);
    module.identifier_at(handle.name).to_string()
}

fn struct_inst_name(module: &CompiledModule, idx: StructDefInstantiationIndex) -> String {
    let inst = &module.struct_def_instantiations[idx.0 as usize];
    struct_name(module, inst.def)
}

fn func_name(module: &CompiledModule, idx: FunctionHandleIndex) -> String {
    let handle = module.function_handle_at(idx);
    module.identifier_at(handle.name).to_string()
}

fn func_inst_name(module: &CompiledModule, idx: FunctionInstantiationIndex) -> String {
    let inst = &module.function_instantiations[idx.0 as usize];
    func_name(module, inst.handle)
}

fn field_name(module: &CompiledModule, idx: FieldHandleIndex) -> String {
    let handle = &module.field_handles[idx.0 as usize];
    let struct_def = &module.struct_defs[handle.owner.0 as usize];
    let struct_handle = module.struct_handle_at(struct_def.struct_handle);
    let sname = module.identifier_at(struct_handle.name);
    let fname = match &struct_def.field_information {
        move_binary_format::file_format::StructFieldInformation::Declared(fields) => module
            .identifier_at(fields[handle.field as usize].name)
            .to_string(),
        _ => format!("#{}", handle.field),
    };
    format!("{}::{}", sname, fname)
}

fn field_inst_name(module: &CompiledModule, idx: FieldInstantiationIndex) -> String {
    let inst = &module.field_instantiations[idx.0 as usize];
    field_name(module, inst.handle)
}

fn variant_handle_name(module: &CompiledModule, idx: StructVariantHandleIndex) -> String {
    let handle = &module.struct_variant_handles[idx.0 as usize];
    let def = &module.struct_defs[handle.struct_index.0 as usize];
    let struct_handle = module.struct_handle_at(def.struct_handle);
    let sname = module.identifier_at(struct_handle.name);
    let vname = match &def.field_information {
        move_binary_format::file_format::StructFieldInformation::DeclaredVariants(variants) => {
            module
                .identifier_at(variants[handle.variant as usize].name)
                .to_string()
        },
        _ => format!("#{}", handle.variant),
    };
    format!("{}::{}", sname, vname)
}

fn variant_inst_name(module: &CompiledModule, idx: StructVariantInstantiationIndex) -> String {
    let inst = &module.struct_variant_instantiations[idx.0 as usize];
    variant_handle_name(module, inst.handle)
}

fn variant_field_name(module: &CompiledModule, idx: VariantFieldHandleIndex) -> String {
    let handle = &module.variant_field_handles[idx.0 as usize];
    let def = &module.struct_defs[handle.struct_index.0 as usize];
    let struct_handle = module.struct_handle_at(def.struct_handle);
    let sname = module.identifier_at(struct_handle.name);
    let (vname, fname) = match &def.field_information {
        move_binary_format::file_format::StructFieldInformation::DeclaredVariants(variants) => {
            let var_def = &variants[handle.variants[0] as usize];
            let vn = module.identifier_at(var_def.name).to_string();
            let fn_ = module
                .identifier_at(var_def.fields[handle.field as usize].name)
                .to_string();
            (vn, fn_)
        },
        _ => (
            format!("#{}", handle.variants[0]),
            format!("#{}", handle.field),
        ),
    };
    format!("{}::{}::{}", sname, vname, fname)
}

fn variant_field_inst_name(module: &CompiledModule, idx: VariantFieldInstantiationIndex) -> String {
    let inst = &module.variant_field_instantiations[idx.0 as usize];
    variant_field_name(module, inst.handle)
}

fn display_instr(
    f: &mut fmt::Formatter<'_>,
    module: &CompiledModule,
    instr: &Instr,
) -> fmt::Result {
    match instr {
        // --- Loads: dst := instr literal ---
        Instr::LdConst(d, idx) => {
            write_dst(f, *d)?;
            write!(f, "ld_const #{}", idx.0)
        },
        Instr::LdTrue(d) => {
            write_dst(f, *d)?;
            write!(f, "ld_true")
        },
        Instr::LdFalse(d) => {
            write_dst(f, *d)?;
            write!(f, "ld_false")
        },
        Instr::LdU8(d, v) => {
            write_dst(f, *d)?;
            write!(f, "ld_u8 {}", v)
        },
        Instr::LdU16(d, v) => {
            write_dst(f, *d)?;
            write!(f, "ld_u16 {}", v)
        },
        Instr::LdU32(d, v) => {
            write_dst(f, *d)?;
            write!(f, "ld_u32 {}", v)
        },
        Instr::LdU64(d, v) => {
            write_dst(f, *d)?;
            write!(f, "ld_u64 {}", v)
        },
        Instr::LdU128(d, v) => {
            write_dst(f, *d)?;
            write!(f, "ld_u128 {}", v)
        },
        Instr::LdU256(d, v) => {
            write_dst(f, *d)?;
            write!(f, "ld_u256 {}", v)
        },
        Instr::LdI8(d, v) => {
            write_dst(f, *d)?;
            write!(f, "ld_i8 {}", v)
        },
        Instr::LdI16(d, v) => {
            write_dst(f, *d)?;
            write!(f, "ld_i16 {}", v)
        },
        Instr::LdI32(d, v) => {
            write_dst(f, *d)?;
            write!(f, "ld_i32 {}", v)
        },
        Instr::LdI64(d, v) => {
            write_dst(f, *d)?;
            write!(f, "ld_i64 {}", v)
        },
        Instr::LdI128(d, v) => {
            write_dst(f, *d)?;
            write!(f, "ld_i128 {}", v)
        },
        Instr::LdI256(d, v) => {
            write_dst(f, *d)?;
            write!(f, "ld_i256 {}", v)
        },

        // --- Register ops: dst := copy/move src ---
        Instr::Copy(d, s) => {
            write_dst(f, *d)?;
            write!(f, "copy {}", reg(*s))
        },
        Instr::Move(d, s) => {
            write_dst(f, *d)?;
            write!(f, "move {}", reg(*s))
        },

        // --- Unary: dst := op src ---
        Instr::UnaryOp(d, op, s) => {
            write_dst(f, *d)?;
            write!(f, "{} {}", unary_op_name(op), reg(*s))
        },
        // --- Binary: dst := op lhs, rhs ---
        Instr::BinaryOp(d, op, l, r) => {
            write_dst(f, *d)?;
            write!(f, "{} {}, {}", binary_op_name(op), reg(*l), reg(*r))
        },
        // --- Binary immediate: dst := op lhs, #imm ---
        Instr::BinaryOpImm(d, op, l, imm) => {
            write_dst(f, *d)?;
            write!(f, "{} {}, {}", binary_op_name(op), reg(*l), imm_value(imm))
        },

        // --- Struct ---
        Instr::Pack(d, idx, fields) => {
            write_dst(f, *d)?;
            write!(f, "pack {}, {}", struct_name(module, *idx), regs(fields))
        },
        Instr::PackGeneric(d, idx, fields) => {
            write_dst(f, *d)?;
            write!(
                f,
                "pack {}, {}",
                struct_inst_name(module, *idx),
                regs(fields)
            )
        },
        Instr::Unpack(ds, idx, s) => {
            write_dsts(f, ds)?;
            write!(f, "unpack {}, {}", struct_name(module, *idx), reg(*s))
        },
        Instr::UnpackGeneric(ds, idx, s) => {
            write_dsts(f, ds)?;
            write!(f, "unpack {}, {}", struct_inst_name(module, *idx), reg(*s))
        },

        // --- Variant ---
        Instr::PackVariant(d, idx, fields) => {
            write_dst(f, *d)?;
            write!(
                f,
                "pack_variant {}, {}",
                variant_handle_name(module, *idx),
                regs(fields)
            )
        },
        Instr::PackVariantGeneric(d, idx, fields) => {
            write_dst(f, *d)?;
            write!(
                f,
                "pack_variant {}, {}",
                variant_inst_name(module, *idx),
                regs(fields)
            )
        },
        Instr::UnpackVariant(ds, idx, s) => {
            write_dsts(f, ds)?;
            write!(
                f,
                "unpack_variant {}, {}",
                variant_handle_name(module, *idx),
                reg(*s)
            )
        },
        Instr::UnpackVariantGeneric(ds, idx, s) => {
            write_dsts(f, ds)?;
            write!(
                f,
                "unpack_variant {}, {}",
                variant_inst_name(module, *idx),
                reg(*s)
            )
        },
        Instr::TestVariant(d, idx, s) => {
            write_dst(f, *d)?;
            write!(
                f,
                "test_variant {}, {}",
                variant_handle_name(module, *idx),
                reg(*s)
            )
        },
        Instr::TestVariantGeneric(d, idx, s) => {
            write_dst(f, *d)?;
            write!(
                f,
                "test_variant {}, {}",
                variant_inst_name(module, *idx),
                reg(*s)
            )
        },

        // --- References ---
        Instr::ImmBorrowLoc(d, s) => {
            write_dst(f, *d)?;
            write!(f, "imm_borrow_loc {}", reg(*s))
        },
        Instr::MutBorrowLoc(d, s) => {
            write_dst(f, *d)?;
            write!(f, "mut_borrow_loc {}", reg(*s))
        },
        Instr::ImmBorrowField(d, idx, s) => {
            write_dst(f, *d)?;
            write!(
                f,
                "imm_borrow_field {}, {}",
                field_name(module, *idx),
                reg(*s)
            )
        },
        Instr::MutBorrowField(d, idx, s) => {
            write_dst(f, *d)?;
            write!(
                f,
                "mut_borrow_field {}, {}",
                field_name(module, *idx),
                reg(*s)
            )
        },
        Instr::ImmBorrowFieldGeneric(d, idx, s) => {
            write_dst(f, *d)?;
            write!(
                f,
                "imm_borrow_field {}, {}",
                field_inst_name(module, *idx),
                reg(*s)
            )
        },
        Instr::MutBorrowFieldGeneric(d, idx, s) => {
            write_dst(f, *d)?;
            write!(
                f,
                "mut_borrow_field {}, {}",
                field_inst_name(module, *idx),
                reg(*s)
            )
        },
        Instr::ImmBorrowVariantField(d, idx, s) => {
            write_dst(f, *d)?;
            write!(
                f,
                "imm_borrow_variant_field {}, {}",
                variant_field_name(module, *idx),
                reg(*s)
            )
        },
        Instr::MutBorrowVariantField(d, idx, s) => {
            write_dst(f, *d)?;
            write!(
                f,
                "mut_borrow_variant_field {}, {}",
                variant_field_name(module, *idx),
                reg(*s)
            )
        },
        Instr::ImmBorrowVariantFieldGeneric(d, idx, s) => {
            write_dst(f, *d)?;
            write!(
                f,
                "imm_borrow_variant_field {}, {}",
                variant_field_inst_name(module, *idx),
                reg(*s)
            )
        },
        Instr::MutBorrowVariantFieldGeneric(d, idx, s) => {
            write_dst(f, *d)?;
            write!(
                f,
                "mut_borrow_variant_field {}, {}",
                variant_field_inst_name(module, *idx),
                reg(*s)
            )
        },
        Instr::ReadRef(d, s) => {
            write_dst(f, *d)?;
            write!(f, "read_ref {}", reg(*s))
        },
        // WriteRef has no destination (side-effect only)
        Instr::WriteRef(d, v) => write!(f, "write_ref {}, {}", reg(*d), reg(*v)),

        // --- Fused field access ---
        Instr::ReadField(d, idx, s) => {
            write_dst(f, *d)?;
            write!(f, "read_field {}, {}", field_name(module, *idx), reg(*s))
        },
        Instr::ReadFieldGeneric(d, idx, s) => {
            write_dst(f, *d)?;
            write!(
                f,
                "read_field {}, {}",
                field_inst_name(module, *idx),
                reg(*s)
            )
        },
        Instr::WriteField(idx, d, v) => {
            write!(
                f,
                "write_field {}, {}, {}",
                field_name(module, *idx),
                reg(*d),
                reg(*v)
            )
        },
        Instr::WriteFieldGeneric(idx, d, v) => {
            write!(
                f,
                "write_field {}, {}, {}",
                field_inst_name(module, *idx),
                reg(*d),
                reg(*v)
            )
        },
        Instr::ReadVariantField(d, idx, s) => {
            write_dst(f, *d)?;
            write!(
                f,
                "read_variant_field {}, {}",
                variant_field_name(module, *idx),
                reg(*s)
            )
        },
        Instr::ReadVariantFieldGeneric(d, idx, s) => {
            write_dst(f, *d)?;
            write!(
                f,
                "read_variant_field {}, {}",
                variant_field_inst_name(module, *idx),
                reg(*s)
            )
        },
        Instr::WriteVariantField(idx, d, v) => {
            write!(
                f,
                "write_variant_field {}, {}, {}",
                variant_field_name(module, *idx),
                reg(*d),
                reg(*v)
            )
        },
        Instr::WriteVariantFieldGeneric(idx, d, v) => {
            write!(
                f,
                "write_variant_field {}, {}, {}",
                variant_field_inst_name(module, *idx),
                reg(*d),
                reg(*v)
            )
        },

        // --- Globals ---
        Instr::Exists(d, idx, a) => {
            write_dst(f, *d)?;
            write!(f, "exists {}, {}", struct_name(module, *idx), reg(*a))
        },
        Instr::ExistsGeneric(d, idx, a) => {
            write_dst(f, *d)?;
            write!(f, "exists {}, {}", struct_inst_name(module, *idx), reg(*a))
        },
        Instr::MoveFrom(d, idx, a) => {
            write_dst(f, *d)?;
            write!(f, "move_from {}, {}", struct_name(module, *idx), reg(*a))
        },
        Instr::MoveFromGeneric(d, idx, a) => {
            write_dst(f, *d)?;
            write!(
                f,
                "move_from {}, {}",
                struct_inst_name(module, *idx),
                reg(*a)
            )
        },
        // MoveTo has no destination (side-effect)
        Instr::MoveTo(idx, s, v) => {
            write!(
                f,
                "move_to {}, {}, {}",
                struct_name(module, *idx),
                reg(*s),
                reg(*v)
            )
        },
        Instr::MoveToGeneric(idx, s, v) => {
            write!(
                f,
                "move_to {}, {}, {}",
                struct_inst_name(module, *idx),
                reg(*s),
                reg(*v)
            )
        },
        Instr::ImmBorrowGlobal(d, idx, a) => {
            write_dst(f, *d)?;
            write!(
                f,
                "imm_borrow_global {}, {}",
                struct_name(module, *idx),
                reg(*a)
            )
        },
        Instr::ImmBorrowGlobalGeneric(d, idx, a) => {
            write_dst(f, *d)?;
            write!(
                f,
                "imm_borrow_global {}, {}",
                struct_inst_name(module, *idx),
                reg(*a)
            )
        },
        Instr::MutBorrowGlobal(d, idx, a) => {
            write_dst(f, *d)?;
            write!(
                f,
                "mut_borrow_global {}, {}",
                struct_name(module, *idx),
                reg(*a)
            )
        },
        Instr::MutBorrowGlobalGeneric(d, idx, a) => {
            write_dst(f, *d)?;
            write!(
                f,
                "mut_borrow_global {}, {}",
                struct_inst_name(module, *idx),
                reg(*a)
            )
        },

        // --- Calls ---
        Instr::Call(rets, idx, args) => {
            write_dsts(f, rets)?;
            write!(f, "call {}, {}", func_name(module, *idx), regs(args))
        },
        Instr::CallGeneric(rets, idx, args) => {
            write_dsts(f, rets)?;
            write!(f, "call {}, {}", func_inst_name(module, *idx), regs(args))
        },

        // --- Closures ---
        Instr::PackClosure(d, idx, mask, captured) => {
            write_dst(f, *d)?;
            write!(
                f,
                "pack_closure {}, {}, {}",
                func_name(module, *idx),
                mask,
                regs(captured)
            )
        },
        Instr::PackClosureGeneric(d, idx, mask, captured) => {
            write_dst(f, *d)?;
            write!(
                f,
                "pack_closure {}, {}, {}",
                func_inst_name(module, *idx),
                mask,
                regs(captured)
            )
        },
        Instr::CallClosure(rets, sig_idx, args) => {
            write_dsts(f, rets)?;
            write!(f, "call_closure #{}, {}", sig_idx.0, regs(args))
        },

        // --- Vector ---
        Instr::VecPack(d, sig, count, elems) => {
            write_dst(f, *d)?;
            write!(f, "vec_pack #{}, {}, {}", sig.0, count, regs(elems))
        },
        Instr::VecLen(d, sig, s) => {
            write_dst(f, *d)?;
            write!(f, "vec_len #{}, {}", sig.0, reg(*s))
        },
        Instr::VecImmBorrow(d, sig, v, i) => {
            write_dst(f, *d)?;
            write!(f, "vec_imm_borrow #{}, {}, {}", sig.0, reg(*v), reg(*i))
        },
        Instr::VecMutBorrow(d, sig, v, i) => {
            write_dst(f, *d)?;
            write!(f, "vec_mut_borrow #{}, {}, {}", sig.0, reg(*v), reg(*i))
        },
        // VecPushBack has no destination
        Instr::VecPushBack(sig, v, val) => {
            write!(f, "vec_push_back #{}, {}, {}", sig.0, reg(*v), reg(*val))
        },
        Instr::VecPopBack(d, sig, s) => {
            write_dst(f, *d)?;
            write!(f, "vec_pop_back #{}, {}", sig.0, reg(*s))
        },
        Instr::VecUnpack(ds, sig, count, s) => {
            write_dsts(f, ds)?;
            write!(f, "vec_unpack #{}, {}, {}", sig.0, count, reg(*s))
        },
        // VecSwap has no destination
        Instr::VecSwap(sig, v, i, j) => {
            write!(
                f,
                "vec_swap #{}, {}, {}, {}",
                sig.0,
                reg(*v),
                reg(*i),
                reg(*j)
            )
        },

        // --- Control flow (no destinations) ---
        Instr::Label(l) => write!(f, "L{}:", l.0),
        Instr::Branch(l) => write!(f, "branch L{}", l.0),
        Instr::BrTrue(l, c) => write!(f, "br_true L{}, {}", l.0, reg(*c)),
        Instr::BrFalse(l, c) => write!(f, "br_false L{}, {}", l.0, reg(*c)),
        Instr::Ret(rs) => write!(f, "ret {}", regs(rs)),
        Instr::Abort(c) => write!(f, "abort {}", reg(*c)),
        Instr::AbortMsg(c, m) => write!(f, "abort_msg {}, {}", reg(*c), reg(*m)),
    }
}

fn unary_op_name(op: &UnaryOp) -> &'static str {
    match op {
        UnaryOp::CastU8 => "cast_u8",
        UnaryOp::CastU16 => "cast_u16",
        UnaryOp::CastU32 => "cast_u32",
        UnaryOp::CastU64 => "cast_u64",
        UnaryOp::CastU128 => "cast_u128",
        UnaryOp::CastU256 => "cast_u256",
        UnaryOp::CastI8 => "cast_i8",
        UnaryOp::CastI16 => "cast_i16",
        UnaryOp::CastI32 => "cast_i32",
        UnaryOp::CastI64 => "cast_i64",
        UnaryOp::CastI128 => "cast_i128",
        UnaryOp::CastI256 => "cast_i256",
        UnaryOp::Not => "not",
        UnaryOp::Negate => "negate",
        UnaryOp::FreezeRef => "freeze_ref",
    }
}

fn binary_op_name(op: &BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "add",
        BinaryOp::Sub => "sub",
        BinaryOp::Mul => "mul",
        BinaryOp::Div => "div",
        BinaryOp::Mod => "mod",
        BinaryOp::BitOr => "bit_or",
        BinaryOp::BitAnd => "bit_and",
        BinaryOp::Xor => "xor",
        BinaryOp::Shl => "shl",
        BinaryOp::Shr => "shr",
        BinaryOp::Lt => "lt",
        BinaryOp::Gt => "gt",
        BinaryOp::Le => "le",
        BinaryOp::Ge => "ge",
        BinaryOp::Eq => "eq",
        BinaryOp::Neq => "neq",
        BinaryOp::Or => "or",
        BinaryOp::And => "and",
    }
}

fn imm_value(imm: &ImmValue) -> String {
    match imm {
        ImmValue::Bool(true) => "#true".to_string(),
        ImmValue::Bool(false) => "#false".to_string(),
        ImmValue::U8(v) => format!("#{}u8", v),
        ImmValue::U16(v) => format!("#{}u16", v),
        ImmValue::U32(v) => format!("#{}u32", v),
        ImmValue::U64(v) => format!("#{}", v),
        ImmValue::I8(v) => format!("#{}i8", v),
        ImmValue::I16(v) => format!("#{}i16", v),
        ImmValue::I32(v) => format!("#{}i32", v),
        ImmValue::I64(v) => format!("#{}i64", v),
    }
}

/// Display a runtime `Type` with struct names resolved via the module.
///
/// `StructNameIndex` values are assumed to be ordinals matching `StructHandleIndex`,
/// which is how this crate's `type_conversion` builds them.
fn display_type(f: &mut fmt::Formatter<'_>, module: &CompiledModule, ty: &Type) -> fmt::Result {
    match ty {
        Type::Bool => write!(f, "bool"),
        Type::U8 => write!(f, "u8"),
        Type::U16 => write!(f, "u16"),
        Type::U32 => write!(f, "u32"),
        Type::U64 => write!(f, "u64"),
        Type::U128 => write!(f, "u128"),
        Type::U256 => write!(f, "u256"),
        Type::I8 => write!(f, "i8"),
        Type::I16 => write!(f, "i16"),
        Type::I32 => write!(f, "i32"),
        Type::I64 => write!(f, "i64"),
        Type::I128 => write!(f, "i128"),
        Type::I256 => write!(f, "i256"),
        Type::Address => write!(f, "address"),
        Type::Signer => write!(f, "signer"),
        Type::TyParam(idx) => write!(f, "_{}", idx),
        Type::Vector(inner) => {
            write!(f, "vector<")?;
            display_type(f, module, inner)?;
            write!(f, ">")
        },
        Type::Reference(inner) => {
            write!(f, "&")?;
            display_type(f, module, inner)
        },
        Type::MutableReference(inner) => {
            write!(f, "&mut ")?;
            display_type(f, module, inner)
        },
        Type::Struct { idx, .. } => {
            let sh_idx = StructHandleIndex(idx.to_string().parse::<u16>().unwrap());
            let handle = module.struct_handle_at(sh_idx);
            write!(f, "{}", module.identifier_at(handle.name))
        },
        Type::StructInstantiation { idx, ty_args, .. } => {
            let sh_idx = StructHandleIndex(idx.to_string().parse::<u16>().unwrap());
            let handle = module.struct_handle_at(sh_idx);
            write!(f, "{}<", module.identifier_at(handle.name))?;
            for (i, arg) in ty_args.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                display_type(f, module, arg)?;
            }
            write!(f, ">")
        },
        Type::Function { args, results, .. } => {
            write!(f, "|")?;
            for (i, arg) in args.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                display_type(f, module, arg)?;
            }
            write!(f, "|")?;
            for (i, r) in results.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                display_type(f, module, r)?;
            }
            Ok(())
        },
    }
}

fn display_sig_token(
    f: &mut fmt::Formatter<'_>,
    module: &CompiledModule,
    tok: &SignatureToken,
) -> fmt::Result {
    match tok {
        SignatureToken::Bool => write!(f, "bool"),
        SignatureToken::U8 => write!(f, "u8"),
        SignatureToken::U16 => write!(f, "u16"),
        SignatureToken::U32 => write!(f, "u32"),
        SignatureToken::U64 => write!(f, "u64"),
        SignatureToken::U128 => write!(f, "u128"),
        SignatureToken::U256 => write!(f, "u256"),
        SignatureToken::I8 => write!(f, "i8"),
        SignatureToken::I16 => write!(f, "i16"),
        SignatureToken::I32 => write!(f, "i32"),
        SignatureToken::I64 => write!(f, "i64"),
        SignatureToken::I128 => write!(f, "i128"),
        SignatureToken::I256 => write!(f, "i256"),
        SignatureToken::Address => write!(f, "address"),
        SignatureToken::Signer => write!(f, "signer"),
        SignatureToken::TypeParameter(idx) => write!(f, "_{}", idx),
        SignatureToken::Vector(inner) => {
            write!(f, "vector<")?;
            display_sig_token(f, module, inner)?;
            write!(f, ">")
        },
        SignatureToken::Reference(inner) => {
            write!(f, "&")?;
            display_sig_token(f, module, inner)
        },
        SignatureToken::MutableReference(inner) => {
            write!(f, "&mut ")?;
            display_sig_token(f, module, inner)
        },
        SignatureToken::Struct(sh_idx) => {
            let handle = module.struct_handle_at(*sh_idx);
            write!(f, "{}", module.identifier_at(handle.name))
        },
        SignatureToken::StructInstantiation(sh_idx, tys) => {
            let handle = module.struct_handle_at(*sh_idx);
            write!(f, "{}<", module.identifier_at(handle.name))?;
            for (i, ty) in tys.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                display_sig_token(f, module, ty)?;
            }
            write!(f, ">")
        },
        SignatureToken::Function(args, results, _abilities) => {
            write!(f, "|")?;
            for (i, ty) in args.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                display_sig_token(f, module, ty)?;
            }
            write!(f, "|")?;
            for (i, ty) in results.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                display_sig_token(f, module, ty)?;
            }
            Ok(())
        },
    }
}
