// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Display for the stackless IR. Types render from their interned form;
//! entity handles (functions, fields, variants) resolve via `CompiledModule`.

use super::{BinaryOp, CmpOp, FunctionIR, ImmValue, Instr, ModuleIR, Slot, UnaryOp};
use mono_move_core::types::{
    view_name, view_type, view_type_list, InternedType, InternedTypeList, Type,
};
use move_binary_format::{
    access::ModuleAccess,
    file_format::{
        FieldHandleIndex, FieldInstantiationIndex, FunctionHandleIndex, FunctionInstantiationIndex,
        SignatureToken, VariantFieldHandleIndex, VariantFieldInstantiationIndex,
    },
    CompiledModule,
};
use std::fmt;

impl fmt::Display for ModuleIR {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let module = &self.module;
        let self_handle = module.module_handle_at(module.self_module_handle_idx);
        let addr = module.address_identifier_at(self_handle.address);
        let name = module.identifier_at(self_handle.name);
        writeln!(f, "// module 0x{}::{}", addr.short_str_lossless(), name)?;

        for func_ir in self.functions.iter().flatten() {
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
        write!(f, "{}", slot_name(Slot::Home(i as u16)))?;
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
    let num_temps = func.num_home_slots - func.num_params - func.num_locals;
    if func.num_xfer_slots > 0 {
        writeln!(
            f,
            "  slots: params({}), locals({}), temps({}), xfer({})",
            func.num_params, func.num_locals, num_temps, func.num_xfer_slots
        )?;
    } else {
        writeln!(
            f,
            "  slots: params({}), locals({}), temps({})",
            func.num_params, func.num_locals, num_temps
        )?;
    }

    // Display slot declarations with types
    for i in 0..func.num_home_slots {
        let ty = func.home_slot_types[i as usize];
        write!(f, "    r{}: ", i)?;
        display_type(f, ty)?;
        writeln!(f)?;
    }
    writeln!(f, "  code:")?;

    // Instructions
    let mut instr_num = 0;
    for block in &func.blocks {
        writeln!(f, "  L{}:", block.label.0)?;
        for instr in &block.instrs {
            write!(f, "    {}: ", instr_num)?;
            display_instr(f, module, instr)?;
            writeln!(f)?;
            instr_num += 1;
        }
    }

    writeln!(f, "}}")?;
    Ok(())
}

fn slot_name(s: Slot) -> String {
    match s {
        Slot::Home(i) => format!("r{}", i),
        Slot::Xfer(i) => format!("x{}", i),
        Slot::Vid(i) => format!("v{}", i),
    }
}

fn slot_names(ss: &[Slot]) -> String {
    let parts: Vec<String> = ss.iter().map(|s| slot_name(*s)).collect();
    format!("[{}]", parts.join(", "))
}

/// Write `dest := ` prefix for a single destination slot.
fn write_dst(f: &mut fmt::Formatter<'_>, d: Slot) -> fmt::Result {
    write!(f, "{} := ", slot_name(d))
}

/// Write `[dests] := ` prefix for multiple destination slots.
fn write_dsts(f: &mut fmt::Formatter<'_>, ds: &[Slot]) -> fmt::Result {
    write!(f, "{} := ", slot_names(ds))
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

        // --- Slot ops: dst := copy/move src ---
        Instr::Copy(d, s) => {
            write_dst(f, *d)?;
            write!(f, "copy {}", slot_name(*s))
        },
        Instr::Move(d, s) => {
            write_dst(f, *d)?;
            write!(f, "move {}", slot_name(*s))
        },

        // --- Unary: dst := op src ---
        Instr::UnaryOp(d, op, s) => {
            write_dst(f, *d)?;
            write!(f, "{} {}", unary_op_name(op), slot_name(*s))
        },
        // --- Binary: dst := op lhs, rhs ---
        Instr::BinaryOp(d, op, l, r) => {
            write_dst(f, *d)?;
            write!(
                f,
                "{} {}, {}",
                binary_op_name(op),
                slot_name(*l),
                slot_name(*r)
            )
        },
        // --- Binary immediate: dst := op lhs, #imm ---
        Instr::BinaryOpImm(d, op, l, imm) => {
            write_dst(f, *d)?;
            write!(
                f,
                "{} {}, {}",
                binary_op_name(op),
                slot_name(*l),
                imm_value(imm)
            )
        },

        // --- Struct ---
        Instr::Pack(d, ty, fields) => {
            write_dst(f, *d)?;
            write!(f, "pack ")?;
            display_type(f, *ty)?;
            write!(f, ", {}", slot_names(fields))
        },
        Instr::PackGeneric(d, base_ty, ty_args, fields) => {
            write_dst(f, *d)?;
            write!(f, "pack ")?;
            display_type(f, *base_ty)?;
            display_type_list(f, *ty_args)?;
            write!(f, ", {}", slot_names(fields))
        },
        Instr::Unpack(ds, ty, s) => {
            write_dsts(f, ds)?;
            write!(f, "unpack ")?;
            display_type(f, *ty)?;
            write!(f, ", {}", slot_name(*s))
        },
        Instr::UnpackGeneric(ds, base_ty, ty_args, s) => {
            write_dsts(f, ds)?;
            write!(f, "unpack ")?;
            display_type(f, *base_ty)?;
            display_type_list(f, *ty_args)?;
            write!(f, ", {}", slot_name(*s))
        },

        // --- Variant ---
        Instr::PackVariant(d, ty, variant, fields) => {
            write_dst(f, *d)?;
            write!(f, "pack_variant ")?;
            display_type(f, *ty)?;
            write!(f, "@{}, {}", variant, slot_names(fields))
        },
        Instr::PackVariantGeneric(d, enum_ty, variant, ty_args, fields) => {
            write_dst(f, *d)?;
            write!(f, "pack_variant ")?;
            display_type(f, *enum_ty)?;
            display_type_list(f, *ty_args)?;
            write!(f, "@{}, {}", variant, slot_names(fields))
        },
        Instr::UnpackVariant(ds, ty, variant, s) => {
            write_dsts(f, ds)?;
            write!(f, "unpack_variant ")?;
            display_type(f, *ty)?;
            write!(f, "@{}, {}", variant, slot_name(*s))
        },
        Instr::UnpackVariantGeneric(ds, enum_ty, variant, ty_args, s) => {
            write_dsts(f, ds)?;
            write!(f, "unpack_variant ")?;
            display_type(f, *enum_ty)?;
            display_type_list(f, *ty_args)?;
            write!(f, "@{}, {}", variant, slot_name(*s))
        },
        Instr::TestVariant(d, ty, variant, s) => {
            write_dst(f, *d)?;
            write!(f, "test_variant ")?;
            display_type(f, *ty)?;
            write!(f, "@{}, {}", variant, slot_name(*s))
        },
        Instr::TestVariantGeneric(d, enum_ty, variant, ty_args, s) => {
            write_dst(f, *d)?;
            write!(f, "test_variant ")?;
            display_type(f, *enum_ty)?;
            display_type_list(f, *ty_args)?;
            write!(f, "@{}, {}", variant, slot_name(*s))
        },

        // --- References ---
        Instr::ImmBorrowLoc(d, s) => {
            write_dst(f, *d)?;
            write!(f, "imm_borrow_loc {}", slot_name(*s))
        },
        Instr::MutBorrowLoc(d, s) => {
            write_dst(f, *d)?;
            write!(f, "mut_borrow_loc {}", slot_name(*s))
        },
        Instr::ImmBorrowField(d, idx, s) => {
            write_dst(f, *d)?;
            write!(
                f,
                "imm_borrow_field {}, {}",
                field_name(module, *idx),
                slot_name(*s)
            )
        },
        Instr::MutBorrowField(d, idx, s) => {
            write_dst(f, *d)?;
            write!(
                f,
                "mut_borrow_field {}, {}",
                field_name(module, *idx),
                slot_name(*s)
            )
        },
        Instr::ImmBorrowFieldGeneric(d, idx, s) => {
            write_dst(f, *d)?;
            write!(
                f,
                "imm_borrow_field {}, {}",
                field_inst_name(module, *idx),
                slot_name(*s)
            )
        },
        Instr::MutBorrowFieldGeneric(d, idx, s) => {
            write_dst(f, *d)?;
            write!(
                f,
                "mut_borrow_field {}, {}",
                field_inst_name(module, *idx),
                slot_name(*s)
            )
        },
        Instr::ImmBorrowVariantField(d, idx, s) => {
            write_dst(f, *d)?;
            write!(
                f,
                "imm_borrow_variant_field {}, {}",
                variant_field_name(module, *idx),
                slot_name(*s)
            )
        },
        Instr::MutBorrowVariantField(d, idx, s) => {
            write_dst(f, *d)?;
            write!(
                f,
                "mut_borrow_variant_field {}, {}",
                variant_field_name(module, *idx),
                slot_name(*s)
            )
        },
        Instr::ImmBorrowVariantFieldGeneric(d, idx, s) => {
            write_dst(f, *d)?;
            write!(
                f,
                "imm_borrow_variant_field {}, {}",
                variant_field_inst_name(module, *idx),
                slot_name(*s)
            )
        },
        Instr::MutBorrowVariantFieldGeneric(d, idx, s) => {
            write_dst(f, *d)?;
            write!(
                f,
                "mut_borrow_variant_field {}, {}",
                variant_field_inst_name(module, *idx),
                slot_name(*s)
            )
        },
        Instr::ReadRef(d, s) => {
            write_dst(f, *d)?;
            write!(f, "read_ref {}", slot_name(*s))
        },
        // WriteRef has no destination (side-effect only)
        Instr::WriteRef(d, v) => write!(f, "write_ref {}, {}", slot_name(*d), slot_name(*v)),

        // --- Fused field access ---
        Instr::ReadField(d, idx, s) => {
            write_dst(f, *d)?;
            write!(
                f,
                "read_field {}, {}",
                field_name(module, *idx),
                slot_name(*s)
            )
        },
        Instr::ReadFieldGeneric(d, idx, s) => {
            write_dst(f, *d)?;
            write!(
                f,
                "read_field {}, {}",
                field_inst_name(module, *idx),
                slot_name(*s)
            )
        },
        Instr::WriteField(idx, d, v) => {
            write!(
                f,
                "write_field {}, {}, {}",
                field_name(module, *idx),
                slot_name(*d),
                slot_name(*v)
            )
        },
        Instr::WriteFieldGeneric(idx, d, v) => {
            write!(
                f,
                "write_field {}, {}, {}",
                field_inst_name(module, *idx),
                slot_name(*d),
                slot_name(*v)
            )
        },
        Instr::ReadVariantField(d, idx, s) => {
            write_dst(f, *d)?;
            write!(
                f,
                "read_variant_field {}, {}",
                variant_field_name(module, *idx),
                slot_name(*s)
            )
        },
        Instr::ReadVariantFieldGeneric(d, idx, s) => {
            write_dst(f, *d)?;
            write!(
                f,
                "read_variant_field {}, {}",
                variant_field_inst_name(module, *idx),
                slot_name(*s)
            )
        },
        Instr::WriteVariantField(idx, d, v) => {
            write!(
                f,
                "write_variant_field {}, {}, {}",
                variant_field_name(module, *idx),
                slot_name(*d),
                slot_name(*v)
            )
        },
        Instr::WriteVariantFieldGeneric(idx, d, v) => {
            write!(
                f,
                "write_variant_field {}, {}, {}",
                variant_field_inst_name(module, *idx),
                slot_name(*d),
                slot_name(*v)
            )
        },

        // --- Globals ---
        Instr::Exists(d, ty, a) => {
            write_dst(f, *d)?;
            write!(f, "exists ")?;
            display_type(f, *ty)?;
            write!(f, ", {}", slot_name(*a))
        },
        Instr::ExistsGeneric(d, base_ty, ty_args, a) => {
            write_dst(f, *d)?;
            write!(f, "exists ")?;
            display_type(f, *base_ty)?;
            display_type_list(f, *ty_args)?;
            write!(f, ", {}", slot_name(*a))
        },
        Instr::MoveFrom(d, ty, a) => {
            write_dst(f, *d)?;
            write!(f, "move_from ")?;
            display_type(f, *ty)?;
            write!(f, ", {}", slot_name(*a))
        },
        Instr::MoveFromGeneric(d, base_ty, ty_args, a) => {
            write_dst(f, *d)?;
            write!(f, "move_from ")?;
            display_type(f, *base_ty)?;
            display_type_list(f, *ty_args)?;
            write!(f, ", {}", slot_name(*a))
        },
        // MoveTo has no destination (side-effect)
        Instr::MoveTo(ty, s, v) => {
            write!(f, "move_to ")?;
            display_type(f, *ty)?;
            write!(f, ", {}, {}", slot_name(*s), slot_name(*v))
        },
        Instr::MoveToGeneric(base_ty, ty_args, s, v) => {
            write!(f, "move_to ")?;
            display_type(f, *base_ty)?;
            display_type_list(f, *ty_args)?;
            write!(f, ", {}, {}", slot_name(*s), slot_name(*v))
        },
        Instr::ImmBorrowGlobal(d, ty, a) => {
            write_dst(f, *d)?;
            write!(f, "imm_borrow_global ")?;
            display_type(f, *ty)?;
            write!(f, ", {}", slot_name(*a))
        },
        Instr::ImmBorrowGlobalGeneric(d, base_ty, ty_args, a) => {
            write_dst(f, *d)?;
            write!(f, "imm_borrow_global ")?;
            display_type(f, *base_ty)?;
            display_type_list(f, *ty_args)?;
            write!(f, ", {}", slot_name(*a))
        },
        Instr::MutBorrowGlobal(d, ty, a) => {
            write_dst(f, *d)?;
            write!(f, "mut_borrow_global ")?;
            display_type(f, *ty)?;
            write!(f, ", {}", slot_name(*a))
        },
        Instr::MutBorrowGlobalGeneric(d, base_ty, ty_args, a) => {
            write_dst(f, *d)?;
            write!(f, "mut_borrow_global ")?;
            display_type(f, *base_ty)?;
            display_type_list(f, *ty_args)?;
            write!(f, ", {}", slot_name(*a))
        },

        // --- Calls ---
        Instr::Call(rets, idx, args) => {
            write_dsts(f, rets)?;
            write!(f, "call {}, {}", func_name(module, *idx), slot_names(args))
        },
        Instr::CallGeneric(rets, idx, args) => {
            write_dsts(f, rets)?;
            write!(
                f,
                "call {}, {}",
                func_inst_name(module, *idx),
                slot_names(args)
            )
        },

        // --- Closures ---
        Instr::PackClosure(d, idx, mask, captured) => {
            write_dst(f, *d)?;
            write!(
                f,
                "pack_closure {}, {}, {}",
                func_name(module, *idx),
                mask,
                slot_names(captured)
            )
        },
        Instr::PackClosureGeneric(d, idx, mask, captured) => {
            write_dst(f, *d)?;
            write!(
                f,
                "pack_closure {}, {}, {}",
                func_inst_name(module, *idx),
                mask,
                slot_names(captured)
            )
        },
        Instr::CallClosure(rets, sig_types, args) => {
            write_dsts(f, rets)?;
            write!(f, "call_closure ")?;
            display_type_list(f, *sig_types)?;
            write!(f, ", {}", slot_names(args))
        },

        // --- Vector ---
        Instr::VecPack(d, elem_ty, count, elems) => {
            write_dst(f, *d)?;
            write!(f, "vec_pack ")?;
            display_type(f, *elem_ty)?;
            write!(f, ", {}, {}", count, slot_names(elems))
        },
        Instr::VecLen(d, elem_ty, s) => {
            write_dst(f, *d)?;
            write!(f, "vec_len ")?;
            display_type(f, *elem_ty)?;
            write!(f, ", {}", slot_name(*s))
        },
        Instr::VecImmBorrow(d, elem_ty, v, i) => {
            write_dst(f, *d)?;
            write!(f, "vec_imm_borrow ")?;
            display_type(f, *elem_ty)?;
            write!(f, ", {}, {}", slot_name(*v), slot_name(*i))
        },
        Instr::VecMutBorrow(d, elem_ty, v, i) => {
            write_dst(f, *d)?;
            write!(f, "vec_mut_borrow ")?;
            display_type(f, *elem_ty)?;
            write!(f, ", {}, {}", slot_name(*v), slot_name(*i))
        },
        // VecPushBack has no destination
        Instr::VecPushBack(elem_ty, v, val) => {
            write!(f, "vec_push_back ")?;
            display_type(f, *elem_ty)?;
            write!(f, ", {}, {}", slot_name(*v), slot_name(*val))
        },
        Instr::VecPopBack(d, elem_ty, s) => {
            write_dst(f, *d)?;
            write!(f, "vec_pop_back ")?;
            display_type(f, *elem_ty)?;
            write!(f, ", {}", slot_name(*s))
        },
        Instr::VecUnpack(ds, elem_ty, count, s) => {
            write_dsts(f, ds)?;
            write!(f, "vec_unpack ")?;
            display_type(f, *elem_ty)?;
            write!(f, ", {}, {}", count, slot_name(*s))
        },
        // VecSwap has no destination
        Instr::VecSwap(elem_ty, v, i, j) => {
            write!(f, "vec_swap ")?;
            display_type(f, *elem_ty)?;
            write!(
                f,
                ", {}, {}, {}",
                slot_name(*v),
                slot_name(*i),
                slot_name(*j)
            )
        },

        // --- Control flow (no destinations) ---
        Instr::Branch(l) => write!(f, "branch L{}", l.0),
        Instr::BrTrue(l, c) => write!(f, "br_true L{}, {}", l.0, slot_name(*c)),
        Instr::BrFalse(l, c) => write!(f, "br_false L{}, {}", l.0, slot_name(*c)),
        Instr::BrCmp(l, op, lhs, rhs) => write!(
            f,
            "br_{} L{}, {}, {}",
            cmp_op_name(op),
            l.0,
            slot_name(*lhs),
            slot_name(*rhs)
        ),
        Instr::BrCmpImm(l, op, src, imm) => write!(
            f,
            "br_{} L{}, {}, {}",
            cmp_op_name(op),
            l.0,
            slot_name(*src),
            imm_value(imm)
        ),
        Instr::Ret(rs) => write!(f, "ret {}", slot_names(rs)),
        Instr::Abort(c) => write!(f, "abort {}", slot_name(*c)),
        Instr::AbortMsg(c, m) => write!(f, "abort_msg {}, {}", slot_name(*c), slot_name(*m)),
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
        BinaryOp::Cmp(cmp) => cmp_op_name(cmp),
        BinaryOp::Or => "or",
        BinaryOp::And => "and",
    }
}

fn cmp_op_name(op: &CmpOp) -> &'static str {
    match op {
        CmpOp::Lt => "lt",
        CmpOp::Gt => "gt",
        CmpOp::Le => "le",
        CmpOp::Ge => "ge",
        CmpOp::Eq => "eq",
        CmpOp::Neq => "neq",
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

/// Display an interned type list as `[T0, T1, ...]`.
fn display_type_list(f: &mut fmt::Formatter<'_>, types: InternedTypeList) -> fmt::Result {
    write!(f, "[")?;
    for (i, ty) in view_type_list(types).iter().enumerate() {
        if i > 0 {
            write!(f, ", ")?;
        }
        display_type(f, *ty)?;
    }
    write!(f, "]")
}

/// Display an interned `Type`. Names are carried on the interned type itself,
/// so no module lookup is needed.
fn display_type(f: &mut fmt::Formatter<'_>, ty: InternedType) -> fmt::Result {
    match view_type(ty) {
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
        Type::TypeParam { idx } => write!(f, "_{}", idx),
        Type::Vector { elem } => {
            write!(f, "vector<")?;
            display_type(f, *elem)?;
            write!(f, ">")
        },
        Type::ImmutRef { inner } => {
            write!(f, "&")?;
            display_type(f, *inner)
        },
        Type::MutRef { inner } => {
            write!(f, "&mut ")?;
            display_type(f, *inner)
        },
        Type::Nominal { name, .. } => {
            write!(f, "{}", view_name(*name))
        },
        Type::Function { args, results, .. } => {
            let args = view_type_list(*args);
            let results = view_type_list(*results);
            write!(f, "|")?;
            for (i, arg) in args.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                display_type(f, *arg)?;
            }
            write!(f, "|")?;
            for (i, r) in results.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                display_type(f, *r)?;
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
