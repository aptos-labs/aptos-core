// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Exports compiled Move modules and scripts as JSON for the Lean
//! bytecode-verifier specification (`third_party/move/lean/bytecode-verifier`).
//!
//! The CLI (`src/main.rs`):
//!
//! ```text
//! move-bytecode-lean-export [--output FILE] INPUT...
//! ```
//!
//! An input ending in `.mv` is decoded by the production deserializer with its
//! default configuration; an input ending in `.masm` is assembled by
//! `move-asm`, with the modules of earlier inputs as its context. The output is
//! one JSON document:
//!
//! ```text
//! {"format": "move-bytecode-lean/1", "modules": [...], "scripts": [...]}
//! ```
//!
//! Field names and enum tags follow `move_binary_format::file_format`. Values
//! that may exceed 2^53 (literals of `u64` and wider types, closure masks,
//! vector counts) are decimal strings. The Lean decoder is `Import/Json.lean`.
//!
//! `--instruction-catalog` instead prints every instruction once, with distinct
//! operands, as `{"format": ..., "instructions": [...]}`. The Lean tests decode
//! it and compare it with their own list, so both sides agree on every tag.

use anyhow::{bail, Context, Result};
use either::Either;
use move_binary_format::file_format::{
    AccessKind, AccessSpecifier, AddressSpecifier, Bytecode, CodeUnit, CompiledModule,
    CompiledScript, Constant, ConstantPoolIndex, FieldDefinition, FieldHandleIndex,
    FieldInstantiationIndex, FunctionAttribute, FunctionDefinition, FunctionHandle,
    FunctionHandleIndex, FunctionInstantiationIndex, ModuleHandle, ResourceSpecifier,
    SignatureIndex, SignatureToken, StructDefInstantiationIndex, StructDefinition,
    StructDefinitionIndex, StructFieldInformation, StructHandle, StructVariantHandleIndex,
    StructVariantInstantiationIndex, VariantFieldHandleIndex, VariantFieldInstantiationIndex,
    Visibility,
};
use move_core_types::{
    ability::AbilitySet,
    account_address::AccountAddress,
    function::ClosureMask,
    int256::{I256, U256},
    metadata::Metadata,
};
use serde_json::{json, Value};
use std::{fs, path::Path};

pub const FORMAT: &str = "move-bytecode-lean/1";

fn ability_set(a: AbilitySet) -> Value {
    json!(a.into_u8())
}

fn address(a: &AccountAddress) -> Value {
    let hex: String = a
        .into_bytes()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect();
    json!(format!("0x{}", hex))
}

fn token(t: &SignatureToken) -> Value {
    use SignatureToken as S;
    match t {
        S::Bool => json!("Bool"),
        S::U8 => json!("U8"),
        S::U16 => json!("U16"),
        S::U32 => json!("U32"),
        S::U64 => json!("U64"),
        S::U128 => json!("U128"),
        S::U256 => json!("U256"),
        S::I8 => json!("I8"),
        S::I16 => json!("I16"),
        S::I32 => json!("I32"),
        S::I64 => json!("I64"),
        S::I128 => json!("I128"),
        S::I256 => json!("I256"),
        S::Address => json!("Address"),
        S::Signer => json!("Signer"),
        S::Vector(elem) => json!({ "Vector": token(elem) }),
        S::Function(params, results, abilities) => {
            json!({ "Function": [tokens(params), tokens(results), ability_set(*abilities)] })
        },
        S::Struct(handle) => json!({ "Struct": handle.0 }),
        S::StructInstantiation(handle, args) => {
            json!({ "StructInstantiation": [handle.0, tokens(args)] })
        },
        S::Reference(inner) => json!({ "Reference": token(inner) }),
        S::MutableReference(inner) => json!({ "MutableReference": token(inner) }),
        S::TypeParameter(index) => json!({ "TypeParameter": index }),
    }
}

fn tokens(ts: &[SignatureToken]) -> Value {
    Value::Array(ts.iter().map(token).collect())
}

fn module_handle(h: &ModuleHandle) -> Value {
    json!({ "address": h.address.0, "name": h.name.0 })
}

fn struct_handle(h: &StructHandle) -> Value {
    let type_parameters: Vec<Value> = h
        .type_parameters
        .iter()
        .map(|p| json!({ "constraints": ability_set(p.constraints), "is_phantom": p.is_phantom }))
        .collect();
    json!({
        "module": h.module.0,
        "name": h.name.0,
        "abilities": ability_set(h.abilities),
        "type_parameters": type_parameters,
    })
}

fn attribute(a: &FunctionAttribute) -> Value {
    match a {
        FunctionAttribute::Persistent => json!("Persistent"),
        FunctionAttribute::ModuleLock => json!("ModuleLock"),
        FunctionAttribute::Pack => json!("Pack"),
        FunctionAttribute::PackVariant(v) => json!({ "PackVariant": v }),
        FunctionAttribute::Unpack => json!("Unpack"),
        FunctionAttribute::UnpackVariant(v) => json!({ "UnpackVariant": v }),
        FunctionAttribute::TestVariant(v) => json!({ "TestVariant": v }),
        FunctionAttribute::BorrowFieldImmutable(o) => json!({ "BorrowFieldImmutable": o }),
        FunctionAttribute::BorrowFieldMutable(o) => json!({ "BorrowFieldMutable": o }),
    }
}

fn access_specifier(s: &AccessSpecifier) -> Value {
    let kind = match s.kind {
        AccessKind::Reads => "Reads",
        AccessKind::Writes => "Writes",
    };
    let resource = match &s.resource {
        ResourceSpecifier::Any => json!("Any"),
        ResourceSpecifier::DeclaredAtAddress(a) => json!({ "DeclaredAtAddress": a.0 }),
        ResourceSpecifier::DeclaredInModule(m) => json!({ "DeclaredInModule": m.0 }),
        ResourceSpecifier::Resource(h) => json!({ "Resource": h.0 }),
        ResourceSpecifier::ResourceInstantiation(h, s) => {
            json!({ "ResourceInstantiation": [h.0, s.0] })
        },
    };
    let address = match &s.address {
        AddressSpecifier::Any => json!("Any"),
        AddressSpecifier::Literal(a) => json!({ "Literal": a.0 }),
        AddressSpecifier::Parameter(local, inst) => {
            json!({ "Parameter": [local, inst.map(|i| i.0)] })
        },
    };
    json!({ "kind": kind, "negated": s.negated, "resource": resource, "address": address })
}

fn access_specifiers(specs: &Option<Vec<AccessSpecifier>>) -> Value {
    match specs {
        None => Value::Null,
        Some(specs) => Value::Array(specs.iter().map(access_specifier).collect()),
    }
}

fn function_handle(h: &FunctionHandle) -> Value {
    let type_parameters: Vec<Value> = h.type_parameters.iter().map(|a| ability_set(*a)).collect();
    let attributes: Vec<Value> = h.attributes.iter().map(attribute).collect();
    json!({
        "module": h.module.0,
        "name": h.name.0,
        "parameters": h.parameters.0,
        "return_": h.return_.0,
        "type_parameters": type_parameters,
        "access_specifiers": access_specifiers(&h.access_specifiers),
        "attributes": attributes,
    })
}

pub fn instr(b: &Bytecode) -> Value {
    use Bytecode as B;
    match b {
        B::Pop => json!("Pop"),
        B::Ret => json!("Ret"),
        B::BrTrue(o) => json!({ "BrTrue": o }),
        B::BrFalse(o) => json!({ "BrFalse": o }),
        B::Branch(o) => json!({ "Branch": o }),
        B::LdU8(v) => json!({ "LdU8": v.to_string() }),
        B::LdU64(v) => json!({ "LdU64": v.to_string() }),
        B::LdU128(v) => json!({ "LdU128": v.to_string() }),
        B::CastU8 => json!("CastU8"),
        B::CastU64 => json!("CastU64"),
        B::CastU128 => json!("CastU128"),
        B::LdConst(i) => json!({ "LdConst": i.0 }),
        B::LdTrue => json!("LdTrue"),
        B::LdFalse => json!("LdFalse"),
        B::CopyLoc(i) => json!({ "CopyLoc": i }),
        B::MoveLoc(i) => json!({ "MoveLoc": i }),
        B::StLoc(i) => json!({ "StLoc": i }),
        B::Call(h) => json!({ "Call": h.0 }),
        B::CallGeneric(i) => json!({ "CallGeneric": i.0 }),
        B::Pack(d) => json!({ "Pack": d.0 }),
        B::PackGeneric(i) => json!({ "PackGeneric": i.0 }),
        B::PackVariant(h) => json!({ "PackVariant": h.0 }),
        B::PackVariantGeneric(i) => json!({ "PackVariantGeneric": i.0 }),
        B::Unpack(d) => json!({ "Unpack": d.0 }),
        B::UnpackGeneric(i) => json!({ "UnpackGeneric": i.0 }),
        B::UnpackVariant(h) => json!({ "UnpackVariant": h.0 }),
        B::UnpackVariantGeneric(i) => json!({ "UnpackVariantGeneric": i.0 }),
        B::TestVariant(h) => json!({ "TestVariant": h.0 }),
        B::TestVariantGeneric(i) => json!({ "TestVariantGeneric": i.0 }),
        B::ReadRef => json!("ReadRef"),
        B::WriteRef => json!("WriteRef"),
        B::FreezeRef => json!("FreezeRef"),
        B::MutBorrowLoc(i) => json!({ "MutBorrowLoc": i }),
        B::ImmBorrowLoc(i) => json!({ "ImmBorrowLoc": i }),
        B::MutBorrowField(h) => json!({ "MutBorrowField": h.0 }),
        B::MutBorrowVariantField(h) => json!({ "MutBorrowVariantField": h.0 }),
        B::MutBorrowFieldGeneric(i) => json!({ "MutBorrowFieldGeneric": i.0 }),
        B::MutBorrowVariantFieldGeneric(i) => json!({ "MutBorrowVariantFieldGeneric": i.0 }),
        B::ImmBorrowField(h) => json!({ "ImmBorrowField": h.0 }),
        B::ImmBorrowVariantField(h) => json!({ "ImmBorrowVariantField": h.0 }),
        B::ImmBorrowFieldGeneric(i) => json!({ "ImmBorrowFieldGeneric": i.0 }),
        B::ImmBorrowVariantFieldGeneric(i) => json!({ "ImmBorrowVariantFieldGeneric": i.0 }),
        B::MutBorrowGlobal(d) => json!({ "MutBorrowGlobal": d.0 }),
        B::MutBorrowGlobalGeneric(i) => json!({ "MutBorrowGlobalGeneric": i.0 }),
        B::ImmBorrowGlobal(d) => json!({ "ImmBorrowGlobal": d.0 }),
        B::ImmBorrowGlobalGeneric(i) => json!({ "ImmBorrowGlobalGeneric": i.0 }),
        B::Add => json!("Add"),
        B::Sub => json!("Sub"),
        B::Mul => json!("Mul"),
        B::Mod => json!("Mod"),
        B::Div => json!("Div"),
        B::BitOr => json!("BitOr"),
        B::BitAnd => json!("BitAnd"),
        B::Xor => json!("Xor"),
        B::Or => json!("Or"),
        B::And => json!("And"),
        B::Not => json!("Not"),
        B::Eq => json!("Eq"),
        B::Neq => json!("Neq"),
        B::Lt => json!("Lt"),
        B::Gt => json!("Gt"),
        B::Le => json!("Le"),
        B::Ge => json!("Ge"),
        B::Abort => json!("Abort"),
        B::Nop => json!("Nop"),
        B::Exists(d) => json!({ "Exists": d.0 }),
        B::ExistsGeneric(i) => json!({ "ExistsGeneric": i.0 }),
        B::MoveFrom(d) => json!({ "MoveFrom": d.0 }),
        B::MoveFromGeneric(i) => json!({ "MoveFromGeneric": i.0 }),
        B::MoveTo(d) => json!({ "MoveTo": d.0 }),
        B::MoveToGeneric(i) => json!({ "MoveToGeneric": i.0 }),
        B::Shl => json!("Shl"),
        B::Shr => json!("Shr"),
        B::VecPack(s, n) => json!({ "VecPack": [s.0, n.to_string()] }),
        B::VecLen(s) => json!({ "VecLen": s.0 }),
        B::VecImmBorrow(s) => json!({ "VecImmBorrow": s.0 }),
        B::VecMutBorrow(s) => json!({ "VecMutBorrow": s.0 }),
        B::VecPushBack(s) => json!({ "VecPushBack": s.0 }),
        B::VecPopBack(s) => json!({ "VecPopBack": s.0 }),
        B::VecUnpack(s, n) => json!({ "VecUnpack": [s.0, n.to_string()] }),
        B::VecSwap(s) => json!({ "VecSwap": s.0 }),
        B::PackClosure(h, mask) => json!({ "PackClosure": [h.0, mask.bits().to_string()] }),
        B::PackClosureGeneric(i, mask) => {
            json!({ "PackClosureGeneric": [i.0, mask.bits().to_string()] })
        },
        B::CallClosure(s) => json!({ "CallClosure": s.0 }),
        B::LdU16(v) => json!({ "LdU16": v.to_string() }),
        B::LdU32(v) => json!({ "LdU32": v.to_string() }),
        B::LdU256(v) => json!({ "LdU256": v.to_string() }),
        B::CastU16 => json!("CastU16"),
        B::CastU32 => json!("CastU32"),
        B::CastU256 => json!("CastU256"),
        B::LdI8(v) => json!({ "LdI8": v.to_string() }),
        B::LdI16(v) => json!({ "LdI16": v.to_string() }),
        B::LdI32(v) => json!({ "LdI32": v.to_string() }),
        B::LdI64(v) => json!({ "LdI64": v.to_string() }),
        B::LdI128(v) => json!({ "LdI128": v.to_string() }),
        B::LdI256(v) => json!({ "LdI256": v.to_string() }),
        B::CastI8 => json!("CastI8"),
        B::CastI16 => json!("CastI16"),
        B::CastI32 => json!("CastI32"),
        B::CastI64 => json!("CastI64"),
        B::CastI128 => json!("CastI128"),
        B::CastI256 => json!("CastI256"),
        B::Negate => json!("Negate"),
        B::AbortMsg => json!("AbortMsg"),
    }
}

fn code_unit(c: &CodeUnit) -> Value {
    let code: Vec<Value> = c.code.iter().map(instr).collect();
    json!({ "locals": c.locals.0, "code": code })
}

fn field_def(f: &FieldDefinition) -> Value {
    json!({ "name": f.name.0, "signature": token(&f.signature.0) })
}

fn struct_def(d: &StructDefinition) -> Value {
    let info = match &d.field_information {
        StructFieldInformation::Native => json!("Native"),
        StructFieldInformation::Declared(fields) => {
            json!({ "Declared": fields.iter().map(field_def).collect::<Vec<_>>() })
        },
        StructFieldInformation::DeclaredVariants(variants) => {
            let variants: Vec<Value> = variants
                .iter()
                .map(|v| {
                    json!({
                        "name": v.name.0,
                        "fields": v.fields.iter().map(field_def).collect::<Vec<_>>(),
                    })
                })
                .collect();
            json!({ "DeclaredVariants": variants })
        },
    };
    json!({ "struct_handle": d.struct_handle.0, "field_information": info })
}

fn function_def(d: &FunctionDefinition) -> Value {
    let visibility = match d.visibility {
        Visibility::Private => "Private",
        Visibility::Public => "Public",
        Visibility::Friend => "Friend",
    };
    let acquires: Vec<u16> = d.acquires_global_resources.iter().map(|s| s.0).collect();
    json!({
        "function": d.function.0,
        "visibility": visibility,
        "is_entry": d.is_entry,
        "acquires_global_resources": acquires,
        "code": d.code.as_ref().map(code_unit),
    })
}

fn constant(c: &Constant) -> Value {
    json!({ "type_": token(&c.type_), "data": c.data })
}

fn metadata(m: &Metadata) -> Value {
    json!({ "key": m.key, "value": m.value })
}

fn list<T>(items: &[T], f: impl Fn(&T) -> Value) -> Value {
    Value::Array(items.iter().map(f).collect())
}

pub fn module(m: &CompiledModule) -> Value {
    json!({
        "version": m.version,
        "self_module_handle_idx": m.self_module_handle_idx.0,
        "module_handles": list(&m.module_handles, module_handle),
        "struct_handles": list(&m.struct_handles, struct_handle),
        "function_handles": list(&m.function_handles, function_handle),
        "field_handles": list(&m.field_handles, |h| json!({ "owner": h.owner.0, "field": h.field })),
        "friend_decls": list(&m.friend_decls, module_handle),
        "struct_def_instantiations": list(&m.struct_def_instantiations, |i| {
            json!({ "def": i.def.0, "type_parameters": i.type_parameters.0 })
        }),
        "function_instantiations": list(&m.function_instantiations, |i| {
            json!({ "handle": i.handle.0, "type_parameters": i.type_parameters.0 })
        }),
        "field_instantiations": list(&m.field_instantiations, |i| {
            json!({ "handle": i.handle.0, "type_parameters": i.type_parameters.0 })
        }),
        "signatures": list(&m.signatures, |s| tokens(&s.0)),
        "identifiers": list(&m.identifiers, |i| json!(i.as_str())),
        "address_identifiers": list(&m.address_identifiers, address),
        "constant_pool": list(&m.constant_pool, constant),
        "metadata": list(&m.metadata, metadata),
        "struct_defs": list(&m.struct_defs, struct_def),
        "function_defs": list(&m.function_defs, function_def),
        "struct_variant_handles": list(&m.struct_variant_handles, |h| {
            json!({ "struct_index": h.struct_index.0, "variant": h.variant })
        }),
        "struct_variant_instantiations": list(&m.struct_variant_instantiations, |i| {
            json!({ "handle": i.handle.0, "type_parameters": i.type_parameters.0 })
        }),
        "variant_field_handles": list(&m.variant_field_handles, |h| {
            json!({ "struct_index": h.struct_index.0, "variants": h.variants, "field": h.field })
        }),
        "variant_field_instantiations": list(&m.variant_field_instantiations, |i| {
            json!({ "handle": i.handle.0, "type_parameters": i.type_parameters.0 })
        }),
    })
}

pub fn script(s: &CompiledScript) -> Value {
    let type_parameters: Vec<Value> = s.type_parameters.iter().map(|a| ability_set(*a)).collect();
    json!({
        "version": s.version,
        "module_handles": list(&s.module_handles, module_handle),
        "struct_handles": list(&s.struct_handles, struct_handle),
        "function_handles": list(&s.function_handles, function_handle),
        "function_instantiations": list(&s.function_instantiations, |i| {
            json!({ "handle": i.handle.0, "type_parameters": i.type_parameters.0 })
        }),
        "signatures": list(&s.signatures, |sig| tokens(&sig.0)),
        "identifiers": list(&s.identifiers, |i| json!(i.as_str())),
        "address_identifiers": list(&s.address_identifiers, address),
        "constant_pool": list(&s.constant_pool, constant),
        "metadata": list(&s.metadata, metadata),
        "code": code_unit(&s.code),
        "type_parameters": type_parameters,
        "parameters": s.parameters.0,
        "access_specifiers": access_specifiers(&s.access_specifiers),
    })
}

/// Every instruction once, in `Bytecode` declaration order, with distinct
/// operands, so that a decoder which swaps tags or operands is caught.
pub fn instruction_catalog() -> Vec<Bytecode> {
    use Bytecode as B;
    vec![
        B::Pop,
        B::Ret,
        B::BrTrue(1),
        B::BrFalse(2),
        B::Branch(3),
        B::LdU8(u8::MAX),
        B::LdU64(u64::MAX),
        B::LdU128(u128::MAX),
        B::CastU8,
        B::CastU64,
        B::CastU128,
        B::LdConst(ConstantPoolIndex(4)),
        B::LdTrue,
        B::LdFalse,
        B::CopyLoc(5),
        B::MoveLoc(6),
        B::StLoc(7),
        B::Call(FunctionHandleIndex(8)),
        B::CallGeneric(FunctionInstantiationIndex(9)),
        B::Pack(StructDefinitionIndex(10)),
        B::PackGeneric(StructDefInstantiationIndex(11)),
        B::PackVariant(StructVariantHandleIndex(12)),
        B::PackVariantGeneric(StructVariantInstantiationIndex(13)),
        B::Unpack(StructDefinitionIndex(14)),
        B::UnpackGeneric(StructDefInstantiationIndex(15)),
        B::UnpackVariant(StructVariantHandleIndex(16)),
        B::UnpackVariantGeneric(StructVariantInstantiationIndex(17)),
        B::TestVariant(StructVariantHandleIndex(18)),
        B::TestVariantGeneric(StructVariantInstantiationIndex(19)),
        B::ReadRef,
        B::WriteRef,
        B::FreezeRef,
        B::MutBorrowLoc(20),
        B::ImmBorrowLoc(21),
        B::MutBorrowField(FieldHandleIndex(22)),
        B::MutBorrowVariantField(VariantFieldHandleIndex(23)),
        B::MutBorrowFieldGeneric(FieldInstantiationIndex(24)),
        B::MutBorrowVariantFieldGeneric(VariantFieldInstantiationIndex(25)),
        B::ImmBorrowField(FieldHandleIndex(26)),
        B::ImmBorrowVariantField(VariantFieldHandleIndex(27)),
        B::ImmBorrowFieldGeneric(FieldInstantiationIndex(28)),
        B::ImmBorrowVariantFieldGeneric(VariantFieldInstantiationIndex(29)),
        B::MutBorrowGlobal(StructDefinitionIndex(30)),
        B::MutBorrowGlobalGeneric(StructDefInstantiationIndex(31)),
        B::ImmBorrowGlobal(StructDefinitionIndex(32)),
        B::ImmBorrowGlobalGeneric(StructDefInstantiationIndex(33)),
        B::Add,
        B::Sub,
        B::Mul,
        B::Mod,
        B::Div,
        B::BitOr,
        B::BitAnd,
        B::Xor,
        B::Or,
        B::And,
        B::Not,
        B::Eq,
        B::Neq,
        B::Lt,
        B::Gt,
        B::Le,
        B::Ge,
        B::Abort,
        B::Nop,
        B::Exists(StructDefinitionIndex(34)),
        B::ExistsGeneric(StructDefInstantiationIndex(35)),
        B::MoveFrom(StructDefinitionIndex(36)),
        B::MoveFromGeneric(StructDefInstantiationIndex(37)),
        B::MoveTo(StructDefinitionIndex(38)),
        B::MoveToGeneric(StructDefInstantiationIndex(39)),
        B::Shl,
        B::Shr,
        B::VecPack(SignatureIndex(40), u64::MAX),
        B::VecLen(SignatureIndex(41)),
        B::VecImmBorrow(SignatureIndex(42)),
        B::VecMutBorrow(SignatureIndex(43)),
        B::VecPushBack(SignatureIndex(44)),
        B::VecPopBack(SignatureIndex(45)),
        B::VecUnpack(SignatureIndex(46), 47),
        B::VecSwap(SignatureIndex(48)),
        B::PackClosure(FunctionHandleIndex(49), ClosureMask::new(u64::MAX)),
        B::PackClosureGeneric(FunctionInstantiationIndex(50), ClosureMask::new(5)),
        B::CallClosure(SignatureIndex(51)),
        B::LdU16(u16::MAX),
        B::LdU32(u32::MAX),
        B::LdU256(U256::MAX),
        B::CastU16,
        B::CastU32,
        B::CastU256,
        B::LdI8(i8::MIN),
        B::LdI16(i16::MIN),
        B::LdI32(i32::MIN),
        B::LdI64(i64::MIN),
        B::LdI128(i128::MIN),
        B::LdI256(I256::MIN),
        B::CastI8,
        B::CastI16,
        B::CastI32,
        B::CastI64,
        B::CastI128,
        B::CastI256,
        B::Negate,
        B::AbortMsg,
    ]
}

/// Loads `.mv` and `.masm` inputs in order; a `.masm` input is assembled with
/// the modules of earlier inputs as its context.
pub fn load_inputs(inputs: &[String]) -> Result<(Vec<CompiledModule>, Vec<CompiledScript>)> {
    let mut modules: Vec<CompiledModule> = vec![];
    let mut scripts: Vec<CompiledScript> = vec![];
    for input in inputs {
        let path = Path::new(input);
        match path.extension().and_then(|e| e.to_str()) {
            Some("mv") => {
                let bytes = fs::read(path).with_context(|| format!("reading {}", input))?;
                match CompiledModule::deserialize(&bytes) {
                    Ok(m) => modules.push(m),
                    Err(module_error) => match CompiledScript::deserialize(&bytes) {
                        Ok(s) => scripts.push(s),
                        Err(_) => bail!("{}: not a module or a script: {:?}", input, module_error),
                    },
                }
            },
            Some("masm") => {
                let source =
                    fs::read_to_string(path).with_context(|| format!("reading {}", input))?;
                let options = move_asm::assembler::Options::default();
                let unit = move_asm::assembler::assemble(&options, &source, modules.iter())
                    .map_err(|diags| {
                        anyhow::anyhow!(move_asm::assembler::diag_to_string(input, &source, diags))
                    })?;
                match unit {
                    Either::Left(m) => modules.push(m),
                    Either::Right(s) => scripts.push(s),
                }
            },
            _ => bail!("{}: expected a `.mv` or `.masm` file", input),
        }
    }
    Ok((modules, scripts))
}

/// Assembles one `.masm` module source, with `deps` as its context.
pub fn assemble_module(
    name: &str,
    source: &str,
    deps: &[CompiledModule],
) -> Result<CompiledModule> {
    let options = move_asm::assembler::Options::default();
    let unit = move_asm::assembler::assemble(&options, source, deps.iter()).map_err(|diags| {
        anyhow::anyhow!(move_asm::assembler::diag_to_string(name, source, diags))
    })?;
    match unit {
        Either::Left(m) => Ok(m),
        Either::Right(_) => bail!("{}: expected a module, found a script", name),
    }
}

/// The exporter document for a set of modules and scripts.
pub fn document(modules: &[CompiledModule], scripts: &[CompiledScript]) -> Value {
    json!({
        "format": FORMAT,
        "modules": modules.iter().map(module).collect::<Vec<_>>(),
        "scripts": scripts.iter().map(script).collect::<Vec<_>>(),
    })
}

pub fn write_output(output: Option<String>, document: Value) -> Result<()> {
    let text = serde_json::to_string_pretty(&document)? + "\n";
    match output {
        Some(path) => fs::write(&path, text).with_context(|| format!("writing {}", path))?,
        None => print!("{}", text),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::BTreeSet, mem::discriminant};

    /// The catalog lists every instruction exactly once, so a new instruction
    /// cannot be added without extending the Lean decoder and its test.
    #[test]
    fn catalog_covers_every_instruction_once() {
        let catalog = instruction_catalog();
        assert_eq!(catalog.len(), Bytecode::VARIANT_COUNT);
        let distinct: BTreeSet<String> = catalog
            .iter()
            .map(|b| format!("{:?}", discriminant(b)))
            .collect();
        assert_eq!(distinct.len(), Bytecode::VARIANT_COUNT);
    }
}
