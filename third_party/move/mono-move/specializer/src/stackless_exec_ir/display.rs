// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Display for the stackless IR. Types render from their interned form;
//! entity handles (functions, fields, variants) resolve via `CompiledModule`.

use super::{
    BinaryOp, CmpKind, FunctionIR, HomeIndex, ImmValue, Instr, ModuleIR, NamedSlot, SsaSlot,
    UnaryOp,
};
use mono_move_core::types::{display_type, display_type_list, InternedTypeList};
use move_binary_format::{
    access::ModuleAccess,
    file_format::{FieldHandleIndex, FunctionHandleIndex, SignatureToken, VariantFieldHandleIndex},
    CompiledModule,
};
use std::fmt;

impl fmt::Display for SsaSlot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SsaSlot::Home(i) => write!(f, "r{}", i.0),
            SsaSlot::ValueId(i) => write!(f, "v{}", i),
        }
    }
}

impl fmt::Display for NamedSlot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NamedSlot::Home(i) => write!(f, "r{}", i.0),
            NamedSlot::Transfer(i) => write!(f, "x{}", i.0),
        }
    }
}

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
        write!(f, "{}", NamedSlot::Home(HomeIndex(i as u16)))?;
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
    if func.num_transfer_positions > 0 {
        writeln!(
            f,
            "  slots: params({}), locals({}), temps({}), transfer({})",
            func.num_params, func.num_locals, num_temps, func.num_transfer_positions
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

fn slot_names<SlotForm: fmt::Display>(ss: &[SlotForm]) -> String {
    let parts: Vec<String> = ss.iter().map(|s| s.to_string()).collect();
    format!("[{}]", parts.join(", "))
}

/// Write `dest := ` prefix for a single destination slot.
fn write_dst<SlotForm: fmt::Display>(f: &mut fmt::Formatter<'_>, d: SlotForm) -> fmt::Result {
    write!(f, "{} := ", d)
}

/// Write `[dests] := ` prefix for multiple destination slots.
fn write_dsts<SlotForm: fmt::Display>(f: &mut fmt::Formatter<'_>, ds: &[SlotForm]) -> fmt::Result {
    write!(f, "{} := ", slot_names(ds))
}

fn func_name(module: &CompiledModule, idx: FunctionHandleIndex) -> String {
    let handle = module.function_handle_at(idx);
    module.identifier_at(handle.name).to_string()
}

/// Writes `<t0, t1, ...>` for a call/closure's type arguments, or nothing when
/// the list is empty (a non-generic target).
fn write_ty_args(f: &mut fmt::Formatter<'_>, ty_args: InternedTypeList) -> fmt::Result {
    if !ty_args.is_empty() {
        write!(f, "<")?;
        display_type_list(f, ty_args)?;
        write!(f, ">")?;
    }
    Ok(())
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

/// Render a fused field chain's path as `Owner0::f0.Owner1::f1...`.
fn field_path_name(module: &CompiledModule, path: &super::FieldPath) -> String {
    path.iter()
        .map(|step| field_name(module, step.1))
        .collect::<Vec<_>>()
        .join(".")
}

fn display_instr<SlotForm: Copy + fmt::Display>(
    f: &mut fmt::Formatter<'_>,
    module: &CompiledModule,
    instr: &Instr<SlotForm>,
) -> fmt::Result {
    match instr {
        // --- Loads: dst := instr literal ---
        Instr::LdConst { dst, const_idx } => {
            write_dst(f, *dst)?;
            write!(f, "ld_const #{}", const_idx.0)
        },
        Instr::LdImm { dst, imm } => {
            write_dst(f, *dst)?;
            write_load_imm(f, imm)
        },

        // --- Slot ops: dst := copy/move src ---
        Instr::Copy { dst, src } => {
            write_dst(f, *dst)?;
            write!(f, "copy {}", src)
        },
        Instr::Move { dst, src } => {
            write_dst(f, *dst)?;
            write!(f, "move {}", src)
        },

        // --- Unary: dst := op src ---
        Instr::UnaryOp { dst, op, src } => {
            write_dst(f, *dst)?;
            write_unary_op(f, op)?;
            write!(f, " {}", src)
        },
        // --- Binary: dst := op lhs, rhs ---
        Instr::BinaryOp { dst, op, lhs, rhs } => {
            write_dst(f, *dst)?;
            write!(f, "{} {}, {}", binary_op_name(op), lhs, rhs)
        },
        // --- Binary immediate: dst := op lhs, #imm ---
        Instr::BinaryOpImm { dst, op, lhs, imm } => {
            write_dst(f, *dst)?;
            write!(f, "{} {}, {}", binary_op_name(op), lhs, imm_value(imm))
        },

        // --- Struct ---
        Instr::Pack {
            dst,
            struct_ty,
            srcs,
        } => {
            write_dst(f, *dst)?;
            write!(f, "pack ")?;
            display_type(f, *struct_ty)?;
            write!(f, ", {}", slot_names(srcs))
        },
        Instr::Unpack {
            dsts,
            struct_ty,
            src,
        } => {
            write_dsts(f, dsts)?;
            write!(f, "unpack ")?;
            display_type(f, *struct_ty)?;
            write!(f, ", {}", src)
        },

        // --- Variant ---
        Instr::PackVariant {
            dst,
            enum_ty,
            variant,
            srcs,
        } => {
            write_dst(f, *dst)?;
            write!(f, "pack_variant ")?;
            display_type(f, *enum_ty)?;
            write!(f, "@{}, {}", variant, slot_names(srcs))
        },
        Instr::UnpackVariant {
            dsts,
            enum_ty,
            variant,
            src,
        } => {
            write_dsts(f, dsts)?;
            write!(f, "unpack_variant ")?;
            display_type(f, *enum_ty)?;
            write!(f, "@{}, {}", variant, src)
        },
        Instr::TestVariant {
            dst,
            enum_ty,
            variant,
            src,
        } => {
            write_dst(f, *dst)?;
            write!(f, "test_variant ")?;
            display_type(f, *enum_ty)?;
            write!(f, "@{}, {}", variant, src)
        },

        // --- References ---
        Instr::ImmBorrowLoc { dst, local } => {
            write_dst(f, *dst)?;
            write!(f, "imm_borrow_loc {}", local)
        },
        Instr::MutBorrowLoc { dst, local } => {
            write_dst(f, *dst)?;
            write!(f, "mut_borrow_loc {}", local)
        },
        Instr::ImmBorrowField {
            dst, field, src, ..
        } => {
            write_dst(f, *dst)?;
            write!(
                f,
                "imm_borrow_field {}, {}",
                field_name(module, *field),
                src
            )
        },
        Instr::MutBorrowField {
            dst, field, src, ..
        } => {
            write_dst(f, *dst)?;
            write!(
                f,
                "mut_borrow_field {}, {}",
                field_name(module, *field),
                src
            )
        },
        Instr::ImmBorrowVariantField {
            dst, field, src, ..
        } => {
            write_dst(f, *dst)?;
            write!(
                f,
                "imm_borrow_variant_field {}, {}",
                variant_field_name(module, *field),
                src
            )
        },
        Instr::MutBorrowVariantField {
            dst, field, src, ..
        } => {
            write_dst(f, *dst)?;
            write!(
                f,
                "mut_borrow_variant_field {}, {}",
                variant_field_name(module, *field),
                src
            )
        },
        Instr::ReadRef { dst, src } => {
            write_dst(f, *dst)?;
            write!(f, "read_ref {}", src)
        },
        // WriteRef has no destination (side-effect only)
        Instr::WriteRef { dst_ref, val } => {
            write!(f, "write_ref {}, {}", dst_ref, val)
        },

        // --- Fused field access ---
        Instr::ReadField {
            dst, field, src, ..
        } => {
            write_dst(f, *dst)?;
            write!(f, "read_field {}, {}", field_name(module, *field), src)
        },
        Instr::WriteField {
            dst_ref,
            field,
            val,
            ..
        } => {
            write!(
                f,
                "write_field {}, {}, {}",
                field_name(module, *field),
                dst_ref,
                val
            )
        },
        Instr::ReadVariantField {
            dst, field, src, ..
        } => {
            write_dst(f, *dst)?;
            write!(
                f,
                "read_variant_field {}, {}",
                variant_field_name(module, *field),
                src
            )
        },
        Instr::WriteVariantField {
            dst_ref,
            field,
            val,
            ..
        } => {
            write!(
                f,
                "write_variant_field {}, {}, {}",
                variant_field_name(module, *field),
                dst_ref,
                val
            )
        },

        // --- Fused inline-struct field access ---
        Instr::ImmBorrowLocField {
            dst, field, local, ..
        } => {
            write_dst(f, *dst)?;
            write!(
                f,
                "imm_borrow_loc_field {}, {}",
                field_name(module, *field),
                local
            )
        },
        Instr::MutBorrowLocField {
            dst, field, local, ..
        } => {
            write_dst(f, *dst)?;
            write!(
                f,
                "mut_borrow_loc_field {}, {}",
                field_name(module, *field),
                local
            )
        },
        Instr::ReadLocField {
            dst, field, local, ..
        } => {
            write_dst(f, *dst)?;
            write!(
                f,
                "read_loc_field {}, {}",
                field_name(module, *field),
                local
            )
        },
        Instr::WriteLocField {
            local, field, val, ..
        } => {
            write!(
                f,
                "write_loc_field {}, {}, {}",
                field_name(module, *field),
                local,
                val
            )
        },

        // --- Fused field chains ---
        Instr::ReadFieldChain { dst, path, src } => {
            write_dst(f, *dst)?;
            write!(
                f,
                "read_field_chain {}, {}",
                field_path_name(module, path),
                src
            )
        },
        Instr::WriteFieldChain { dst_ref, path, val } => {
            write!(
                f,
                "write_field_chain {}, {}, {}",
                field_path_name(module, path),
                dst_ref,
                val
            )
        },
        Instr::ImmBorrowFieldChain { dst, path, src } => {
            write_dst(f, *dst)?;
            write!(
                f,
                "imm_borrow_field_chain {}, {}",
                field_path_name(module, path),
                src
            )
        },
        Instr::MutBorrowFieldChain { dst, path, src } => {
            write_dst(f, *dst)?;
            write!(
                f,
                "mut_borrow_field_chain {}, {}",
                field_path_name(module, path),
                src
            )
        },
        Instr::ReadLocFieldChain { dst, path, local } => {
            write_dst(f, *dst)?;
            write!(
                f,
                "read_loc_field_chain {}, {}",
                field_path_name(module, path),
                local
            )
        },
        Instr::WriteLocFieldChain { local, path, val } => {
            write!(
                f,
                "write_loc_field_chain {}, {}, {}",
                field_path_name(module, path),
                local,
                val
            )
        },
        Instr::ImmBorrowLocFieldChain { dst, path, local } => {
            write_dst(f, *dst)?;
            write!(
                f,
                "imm_borrow_loc_field_chain {}, {}",
                field_path_name(module, path),
                local
            )
        },
        Instr::MutBorrowLocFieldChain { dst, path, local } => {
            write_dst(f, *dst)?;
            write!(
                f,
                "mut_borrow_loc_field_chain {}, {}",
                field_path_name(module, path),
                local
            )
        },

        // --- Globals ---
        Instr::Exists {
            dst,
            resource_ty,
            addr,
        } => {
            write_dst(f, *dst)?;
            write!(f, "exists ")?;
            display_type(f, *resource_ty)?;
            write!(f, ", {}", addr)
        },
        Instr::MoveFrom {
            dst,
            resource_ty,
            addr,
        } => {
            write_dst(f, *dst)?;
            write!(f, "move_from ")?;
            display_type(f, *resource_ty)?;
            write!(f, ", {}", addr)
        },
        // MoveTo has no destination (side-effect)
        Instr::MoveTo {
            resource_ty,
            signer,
            val,
        } => {
            write!(f, "move_to ")?;
            display_type(f, *resource_ty)?;
            write!(f, ", {}, {}", signer, val)
        },
        Instr::ImmBorrowGlobal {
            dst,
            resource_ty,
            addr,
        } => {
            write_dst(f, *dst)?;
            write!(f, "imm_borrow_global ")?;
            display_type(f, *resource_ty)?;
            write!(f, ", {}", addr)
        },
        Instr::MutBorrowGlobal {
            dst,
            resource_ty,
            addr,
        } => {
            write_dst(f, *dst)?;
            write!(f, "mut_borrow_global ")?;
            display_type(f, *resource_ty)?;
            write!(f, ", {}", addr)
        },

        // --- Calls ---
        Instr::Call { data } => {
            write_dsts(f, &data.rets)?;
            write!(f, "call {}", func_name(module, data.function_handle))?;
            write_ty_args(f, data.ty_args)?;
            write!(f, ", {}", slot_names(&data.args))
        },

        // --- Closures ---
        Instr::PackClosure { data } => {
            write_dst(f, data.dst)?;
            write!(
                f,
                "pack_closure {}",
                func_name(module, data.function_handle)
            )?;
            write_ty_args(f, data.ty_args)?;
            write!(f, ", {}, {}", data.mask, slot_names(&data.captured))
        },
        Instr::CallClosure { data } => {
            write_dsts(f, &data.rets)?;
            write!(f, "call_closure ")?;
            write!(f, "[")?;
            display_type(f, data.closure_ty)?;
            write!(f, "]")?;
            write!(f, ", {}", slot_names(&data.args))
        },

        // --- Vector ---
        Instr::VecPack { dst, elem_ty, srcs } => {
            write_dst(f, *dst)?;
            write!(f, "vec_pack ")?;
            display_type(f, *elem_ty)?;
            write!(f, ", {}, {}", srcs.len(), slot_names(srcs))
        },
        Instr::VecLen {
            dst,
            elem_ty,
            vec_ref,
        } => {
            write_dst(f, *dst)?;
            write!(f, "vec_len ")?;
            display_type(f, *elem_ty)?;
            write!(f, ", {}", vec_ref)
        },
        Instr::VecImmBorrow {
            dst,
            elem_ty,
            vec_ref,
            idx,
        } => {
            write_dst(f, *dst)?;
            write!(f, "vec_imm_borrow ")?;
            display_type(f, *elem_ty)?;
            write!(f, ", {}, {}", vec_ref, idx)
        },
        Instr::VecMutBorrow {
            dst,
            elem_ty,
            vec_ref,
            idx,
        } => {
            write_dst(f, *dst)?;
            write!(f, "vec_mut_borrow ")?;
            display_type(f, *elem_ty)?;
            write!(f, ", {}, {}", vec_ref, idx)
        },
        // VecPushBack has no destination
        Instr::VecPushBack {
            vec_ref,
            elem_ty,
            val,
        } => {
            write!(f, "vec_push_back ")?;
            display_type(f, *elem_ty)?;
            write!(f, ", {}, {}", vec_ref, val)
        },
        Instr::VecPopBack {
            dst,
            elem_ty,
            vec_ref,
        } => {
            write_dst(f, *dst)?;
            write!(f, "vec_pop_back ")?;
            display_type(f, *elem_ty)?;
            write!(f, ", {}", vec_ref)
        },
        Instr::VecUnpack { dsts, elem_ty, src } => {
            write_dsts(f, dsts)?;
            write!(f, "vec_unpack ")?;
            display_type(f, *elem_ty)?;
            write!(f, ", {}, {}", dsts.len(), src)
        },
        // VecSwap has no destination
        Instr::VecSwap {
            vec_ref,
            elem_ty,
            idx_a,
            idx_b,
        } => {
            write!(f, "vec_swap ")?;
            display_type(f, *elem_ty)?;
            write!(f, ", {}, {}, {}", vec_ref, idx_a, idx_b)
        },

        // --- Control flow (no destinations) ---
        Instr::Branch { target } => write!(f, "branch L{}", target.0),
        Instr::BrTrue { target, cond } => write!(f, "br_true L{}, {}", target.0, cond),
        Instr::BrFalse { target, cond } => {
            write!(f, "br_false L{}, {}", target.0, cond)
        },
        Instr::BrCmp {
            target,
            op,
            lhs,
            rhs,
        } => write!(f, "br_{} L{}, {}, {}", cmp_op_name(op), target.0, lhs, rhs),
        Instr::BrCmpImm {
            target,
            op,
            lhs,
            imm,
        } => write!(
            f,
            "br_{} L{}, {}, {}",
            cmp_op_name(op),
            target.0,
            lhs,
            imm_value(imm)
        ),
        Instr::Ret { srcs } => write!(f, "ret {}", slot_names(srcs)),
        Instr::Abort { code } => write!(f, "abort {}", code),
        Instr::AbortMsg { code, msg } => {
            write!(f, "abort_msg {}, {}", code, msg)
        },
        Instr::ForceGC => write!(f, "force_gc"),
    }
}

fn write_unary_op(f: &mut fmt::Formatter<'_>, op: &UnaryOp) -> fmt::Result {
    match op {
        UnaryOp::Cast(to) => write!(f, "cast_{to}"),
        UnaryOp::Not => f.write_str("not"),
        UnaryOp::Negate => f.write_str("negate"),
        UnaryOp::FreezeRef => f.write_str("freeze_ref"),
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
        BinaryOp::BitXor => "bit_xor",
        BinaryOp::Shl => "shl",
        BinaryOp::Shr => "shr",
        BinaryOp::Cmp(cmp) => cmp_op_name(cmp),
        BinaryOp::Or => "or",
        BinaryOp::And => "and",
    }
}

fn cmp_op_name(op: &CmpKind) -> &'static str {
    match op {
        CmpKind::Lt => "lt",
        CmpKind::Gt => "gt",
        CmpKind::Le => "le",
        CmpKind::Ge => "ge",
        CmpKind::Eq => "eq",
        CmpKind::Neq => "neq",
    }
}

/// Load mnemonic for an immediate: `ld_true`, `ld_false`, or `ld_<width> <value>`.
fn write_load_imm(f: &mut fmt::Formatter<'_>, imm: &ImmValue) -> fmt::Result {
    match imm {
        ImmValue::Bool(true) => write!(f, "ld_true"),
        ImmValue::Bool(false) => write!(f, "ld_false"),
        ImmValue::U8(value) => write!(f, "ld_u8 {}", value),
        ImmValue::U16(value) => write!(f, "ld_u16 {}", value),
        ImmValue::U32(value) => write!(f, "ld_u32 {}", value),
        ImmValue::U64(value) => write!(f, "ld_u64 {}", value),
        ImmValue::U128(value) => write!(f, "ld_u128 {}", value),
        ImmValue::U256(value) => write!(f, "ld_u256 {}", value),
        ImmValue::I8(value) => write!(f, "ld_i8 {}", value),
        ImmValue::I16(value) => write!(f, "ld_i16 {}", value),
        ImmValue::I32(value) => write!(f, "ld_i32 {}", value),
        ImmValue::I64(value) => write!(f, "ld_i64 {}", value),
        ImmValue::I128(value) => write!(f, "ld_i128 {}", value),
        ImmValue::I256(value) => write!(f, "ld_i256 {}", value),
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
        ImmValue::U128(v) => format!("#{}u128", **v),
        ImmValue::U256(v) => format!("#{}u256", **v),
        ImmValue::I8(v) => format!("#{}i8", v),
        ImmValue::I16(v) => format!("#{}i16", v),
        ImmValue::I32(v) => format!("#{}i32", v),
        ImmValue::I64(v) => format!("#{}i64", v),
        ImmValue::I128(v) => format!("#{}i128", **v),
        ImmValue::I256(v) => format!("#{}i256", **v),
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
