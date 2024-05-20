// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::fat_type::{FatStructType, FatType, WrappedAbilitySet};
use anyhow::{anyhow, bail};
pub use limit::Limiter;
use move_binary_format::{
    access::ModuleAccess,
    errors::{Location, PartialVMError},
    file_format::{
        Ability, AbilitySet, SignatureToken, StructDefinitionIndex, StructFieldInformation,
        StructHandleIndex,
    },
    views::FunctionHandleView,
    CompiledModule,
};
use move_bytecode_utils::{compiled_module_viewer::CompiledModuleView, layout::TypeLayoutBuilder};
use move_core_types::{
    account_address::AccountAddress,
    identifier::{IdentStr, Identifier},
    language_storage::{ModuleId, StructTag, TypeTag},
    u256,
    value::{MoveStruct, MoveTypeLayout, MoveValue},
    vm_status::VMStatus,
};
use serde::ser::{SerializeMap, SerializeSeq};
use std::{
    borrow::Borrow,
    convert::{TryFrom, TryInto},
    fmt::{Display, Formatter},
};

mod fat_type;
mod limit;

#[derive(Clone, Debug)]
pub struct AnnotatedMoveStruct {
    pub abilities: AbilitySet,
    pub ty_tag: StructTag,
    pub value: Vec<(Identifier, AnnotatedMoveValue)>,
}

/// AnnotatedMoveValue is a fully expanded version of on chain Move data. This should only be used
/// for debugging/client purpose right now and just for a better visualization of on chain data. In
/// the long run, we would like to transform this struct to a Json value so that we can have a cross
/// platform interpretation of the on chain data.
#[derive(Clone, Debug)]
pub enum AnnotatedMoveValue {
    U8(u8),
    U64(u64),
    U128(u128),
    Bool(bool),
    Address(AccountAddress),
    Vector(TypeTag, Vec<AnnotatedMoveValue>),
    Bytes(Vec<u8>),
    Struct(AnnotatedMoveStruct),
    // NOTE: Added in bytecode version v6, do not reorder!
    U16(u16),
    U32(u32),
    U256(u256::U256),
}

impl AnnotatedMoveValue {
    pub fn ty_tag(&self) -> TypeTag {
        use AnnotatedMoveValue::*;
        match self {
            U8(_) => TypeTag::U8,
            U16(_) => TypeTag::U16,
            U32(_) => TypeTag::U32,
            U64(_) => TypeTag::U64,
            U128(_) => TypeTag::U128,
            U256(_) => TypeTag::U256,
            Bool(_) => TypeTag::Bool,
            Address(_) => TypeTag::Address,
            Vector(t, _) => t.clone(),
            Bytes(_) => TypeTag::Vector(Box::new(TypeTag::U8)),
            Struct(s) => TypeTag::Struct(Box::new(s.ty_tag.clone())),
        }
    }
}

pub struct MoveValueAnnotator<V> {
    module_viewer: V,
}

impl<V: CompiledModuleView> MoveValueAnnotator<V> {
    pub fn new(module_viewer: V) -> Self {
        Self { module_viewer }
    }

    pub fn get_type_layout_runtime(&self, type_tag: &TypeTag) -> anyhow::Result<MoveTypeLayout> {
        TypeLayoutBuilder::build_runtime(type_tag, &self.module_viewer)
    }

    pub fn get_type_layout_with_fields(
        &self,
        type_tag: &TypeTag,
    ) -> anyhow::Result<MoveTypeLayout> {
        TypeLayoutBuilder::build_with_fields(type_tag, &self.module_viewer)
    }

    pub fn get_type_layout_with_types(&self, type_tag: &TypeTag) -> anyhow::Result<MoveTypeLayout> {
        TypeLayoutBuilder::build_with_types(type_tag, &self.module_viewer)
    }

    pub fn view_module(&self, id: &ModuleId) -> anyhow::Result<Option<V::Item>> {
        self.module_viewer.view_compiled_module(id)
    }

    pub fn view_existing_module(&self, id: &ModuleId) -> anyhow::Result<V::Item> {
        match self.view_module(id)? {
            Some(module) => Ok(module),
            None => bail!("Module {:?} can't be found", id),
        }
    }

    pub fn view_function_arguments(
        &self,
        module: &ModuleId,
        function: &IdentStr,
        ty_args: &[TypeTag],
        args: &[Vec<u8>],
    ) -> anyhow::Result<Vec<AnnotatedMoveValue>> {
        let mut limit = Limiter::default();
        let types: Vec<FatType> = self
            .resolve_function_arguments(module, function)?
            .into_iter()
            .filter(|t| match t {
                FatType::Signer => false,
                FatType::Reference(inner) => !matches!(&**inner, FatType::Signer),
                FatType::Bool
                | FatType::U8
                | FatType::U64
                | FatType::U128
                | FatType::Address
                | FatType::Vector(_)
                | FatType::Struct(_)
                | FatType::MutableReference(_)
                | FatType::TyParam(_)
                | FatType::U16
                | FatType::U32
                | FatType::U256 => true,
            })
            .collect();
        anyhow::ensure!(
            types.len() == args.len(),
            "unexpected error: argument types({}) and values({}) are not matched",
            types.len(),
            args.len(),
        );

        // Make an approximation at the fat types for the type arguments
        let ty_args: Vec<FatType> = ty_args.iter().map(|inner| inner.into()).collect();

        types
            .iter()
            .enumerate()
            .map(|(i, ty)| {
                ty.subst(&ty_args, &mut limit)
                    .map_err(anyhow::Error::from)
                    .and_then(|fat_type| {
                        self.view_value_by_fat_type(&fat_type, &args[i], &mut limit)
                    })
            })
            .collect::<anyhow::Result<Vec<AnnotatedMoveValue>>>()
    }

    fn resolve_function_arguments(
        &self,
        module: &ModuleId,
        function: &IdentStr,
    ) -> anyhow::Result<Vec<FatType>> {
        let mut limit = Limiter::default();
        let m = self.view_existing_module(module)?;
        let m = m.borrow();
        for def in m.function_defs.iter() {
            let fhandle = m.function_handle_at(def.function);
            let fhandle_view = FunctionHandleView::new(m, fhandle);
            if fhandle_view.name() == function {
                return fhandle_view
                    .parameters()
                    .0
                    .iter()
                    .map(|signature| self.resolve_signature(m, signature, &mut limit))
                    .collect::<anyhow::Result<_>>();
            }
        }
        Err(anyhow!("Function {:?} not found in {:?}", function, module))
    }

    pub fn view_resource(
        &self,
        tag: &StructTag,
        blob: &[u8],
    ) -> anyhow::Result<AnnotatedMoveStruct> {
        self.view_resource_with_limit(tag, blob, &mut Limiter::default())
    }

    pub fn view_resource_with_limit(
        &self,
        tag: &StructTag,
        blob: &[u8],
        limit: &mut Limiter,
    ) -> anyhow::Result<AnnotatedMoveStruct> {
        let ty = self.resolve_struct(tag)?;
        let struct_def = (&ty).try_into().map_err(into_vm_status)?;
        let move_struct = MoveStruct::simple_deserialize(blob, &struct_def)?;
        self.annotate_struct(&move_struct, &ty, limit)
    }

    pub fn move_struct_fields(
        &self,
        tag: &StructTag,
        blob: &[u8],
    ) -> anyhow::Result<Vec<(Identifier, MoveValue)>> {
        let ty = self.resolve_struct(tag)?;
        let struct_def = (&ty).try_into().map_err(into_vm_status)?;
        Ok(match MoveStruct::simple_deserialize(blob, &struct_def)? {
            MoveStruct::Runtime(runtime) => self
                .get_field_names(&ty)?
                .into_iter()
                .zip(runtime)
                .collect(),
            MoveStruct::WithFields(fields) | MoveStruct::WithTypes { fields, .. } => fields,
        })
    }

    fn resolve_struct(&self, struct_tag: &StructTag) -> anyhow::Result<FatStructType> {
        self.resolve_struct_impl(struct_tag, &mut Limiter::default())
    }

    fn resolve_struct_impl(
        &self,
        struct_tag: &StructTag,
        limit: &mut Limiter,
    ) -> anyhow::Result<FatStructType> {
        let module_id = ModuleId::new(struct_tag.address, struct_tag.module.clone());
        let module = self.view_existing_module(&module_id)?;
        let module = module.borrow();

        let struct_def = find_struct_def_in_module(module, struct_tag.name.as_ident_str())?;
        let ty_args = struct_tag
            .type_args
            .iter()
            .map(|ty| self.resolve_type_impl(ty, limit))
            .collect::<anyhow::Result<Vec<_>>>()?;
        let ty_body = self.resolve_struct_definition(module, struct_def, limit)?;
        ty_body.subst(&ty_args, limit).map_err(|e: PartialVMError| {
            anyhow!("StructTag {:?} cannot be resolved: {:?}", struct_tag, e)
        })
    }

    fn resolve_struct_definition(
        &self,
        module: &CompiledModule,
        idx: StructDefinitionIndex,
        limit: &mut Limiter,
    ) -> anyhow::Result<FatStructType> {
        let struct_def = module.struct_def_at(idx);
        let struct_handle = module.struct_handle_at(struct_def.struct_handle);
        let address = *module.address();
        let module_name = module.name().to_owned();
        let name = module.identifier_at(struct_handle.name).to_owned();
        let abilities = struct_handle.abilities;
        let ty_args = (0..struct_handle.type_parameters.len())
            .map(FatType::TyParam)
            .collect();

        limit.charge(std::mem::size_of::<AccountAddress>())?;
        limit.charge(module_name.as_bytes().len())?;
        limit.charge(name.as_bytes().len())?;

        match &struct_def.field_information {
            StructFieldInformation::Native => Err(anyhow!("Unexpected Native Struct")),
            StructFieldInformation::Declared(defs) => Ok(FatStructType {
                address,
                module: module_name,
                name,
                abilities: WrappedAbilitySet(abilities),
                ty_args,
                layout: defs
                    .iter()
                    .map(|field_def| self.resolve_signature(module, &field_def.signature.0, limit))
                    .collect::<anyhow::Result<_>>()?,
            }),
        }
    }

    fn resolve_signature(
        &self,
        module: &CompiledModule,
        sig: &SignatureToken,
        limit: &mut Limiter,
    ) -> anyhow::Result<FatType> {
        Ok(match sig {
            SignatureToken::Bool => FatType::Bool,
            SignatureToken::U8 => FatType::U8,
            SignatureToken::U16 => FatType::U16,
            SignatureToken::U32 => FatType::U32,
            SignatureToken::U64 => FatType::U64,
            SignatureToken::U128 => FatType::U128,
            SignatureToken::U256 => FatType::U256,
            SignatureToken::Address => FatType::Address,
            SignatureToken::Signer => FatType::Signer,
            SignatureToken::Vector(ty) => {
                FatType::Vector(Box::new(self.resolve_signature(module, ty, limit)?))
            },
            SignatureToken::Struct(idx) => {
                FatType::Struct(Box::new(self.resolve_struct_handle(module, *idx, limit)?))
            },
            SignatureToken::StructInstantiation(idx, toks) => {
                let struct_ty = self.resolve_struct_handle(module, *idx, limit)?;
                let args = toks
                    .iter()
                    .map(|tok| self.resolve_signature(module, tok, limit))
                    .collect::<anyhow::Result<Vec<_>>>()?;
                FatType::Struct(Box::new(
                    struct_ty
                        .subst(&args, limit)
                        .map_err(|status| anyhow!("Substitution failure: {:?}", status))?,
                ))
            },
            SignatureToken::TypeParameter(idx) => FatType::TyParam(*idx as usize),
            SignatureToken::MutableReference(_) => return Err(anyhow!("Unexpected Reference")),
            SignatureToken::Reference(inner) => match **inner {
                SignatureToken::Signer => FatType::Reference(Box::new(FatType::Signer)),
                _ => return Err(anyhow!("Unexpected Reference")),
            },
        })
    }

    fn resolve_struct_handle(
        &self,
        module: &CompiledModule,
        idx: StructHandleIndex,
        limit: &mut Limiter,
    ) -> anyhow::Result<FatStructType> {
        let struct_handle = module.struct_handle_at(idx);
        let target_module = {
            let module_handle = module.module_handle_at(struct_handle.module);
            let module_id = ModuleId::new(
                *module.address_identifier_at(module_handle.address),
                module.identifier_at(module_handle.name).to_owned(),
            );
            self.view_existing_module(&module_id)?
        };
        let target_module = target_module.borrow();
        let target_idx =
            find_struct_def_in_module(target_module, module.identifier_at(struct_handle.name))?;
        self.resolve_struct_definition(target_module, target_idx, limit)
    }

    fn resolve_type_impl(
        &self,
        type_tag: &TypeTag,
        limit: &mut Limiter,
    ) -> anyhow::Result<FatType> {
        Ok(match type_tag {
            TypeTag::Address => FatType::Address,
            TypeTag::Signer => FatType::Signer,
            TypeTag::Bool => FatType::Bool,
            TypeTag::Struct(st) => FatType::Struct(Box::new(self.resolve_struct_impl(st, limit)?)),
            TypeTag::U8 => FatType::U8,
            TypeTag::U16 => FatType::U16,
            TypeTag::U32 => FatType::U32,
            TypeTag::U64 => FatType::U64,
            TypeTag::U256 => FatType::U256,
            TypeTag::U128 => FatType::U128,
            TypeTag::Vector(ty) => FatType::Vector(Box::new(self.resolve_type_impl(ty, limit)?)),
        })
    }

    pub fn view_value(&self, ty_tag: &TypeTag, blob: &[u8]) -> anyhow::Result<AnnotatedMoveValue> {
        let mut limit = Limiter::default();
        let ty = self.resolve_type_impl(ty_tag, &mut limit)?;
        self.view_value_by_fat_type(&ty, blob, &mut limit)
    }

    fn view_value_by_fat_type(
        &self,
        ty: &FatType,
        blob: &[u8],
        limit: &mut Limiter,
    ) -> anyhow::Result<AnnotatedMoveValue> {
        let layout = ty.try_into().map_err(into_vm_status)?;
        let move_value = MoveValue::simple_deserialize(blob, &layout)?;
        self.annotate_value(&move_value, ty, limit)
    }

    fn annotate_struct(
        &self,
        move_struct: &MoveStruct,
        ty: &FatStructType,
        limit: &mut Limiter,
    ) -> anyhow::Result<AnnotatedMoveStruct> {
        let struct_tag = ty
            .struct_tag(limit)
            .map_err(|e| e.finish(Location::Undefined).into_vm_status())?;
        let field_names = self.get_field_names(ty)?;
        for names in field_names.iter() {
            limit.charge(names.as_bytes().len())?;
        }
        let mut annotated_fields = vec![];
        for (ty, v) in ty.layout.iter().zip(move_struct.fields().iter()) {
            annotated_fields.push(self.annotate_value(v, ty, limit)?);
        }
        Ok(AnnotatedMoveStruct {
            abilities: ty.abilities.0,
            ty_tag: struct_tag,
            value: field_names.into_iter().zip(annotated_fields).collect(),
        })
    }

    fn get_field_names(&self, ty: &FatStructType) -> anyhow::Result<Vec<Identifier>> {
        let module_id = ModuleId::new(ty.address, ty.module.clone());
        let module = self.view_existing_module(&module_id)?;
        let module = module.borrow();
        let struct_def_idx = find_struct_def_in_module(module, ty.name.as_ident_str())?;
        let struct_def = module.struct_def_at(struct_def_idx);

        match &struct_def.field_information {
            StructFieldInformation::Native => Err(anyhow!("Unexpected Native Struct")),
            StructFieldInformation::Declared(defs) => Ok(defs
                .iter()
                .map(|field_def| module.identifier_at(field_def.name).to_owned())
                .collect()),
        }
    }

    fn annotate_value(
        &self,
        value: &MoveValue,
        ty: &FatType,
        limit: &mut Limiter,
    ) -> anyhow::Result<AnnotatedMoveValue> {
        Ok(match (value, ty) {
            (MoveValue::Bool(b), FatType::Bool) => AnnotatedMoveValue::Bool(*b),
            (MoveValue::U8(i), FatType::U8) => AnnotatedMoveValue::U8(*i),
            (MoveValue::U16(i), FatType::U16) => AnnotatedMoveValue::U16(*i),
            (MoveValue::U32(i), FatType::U32) => AnnotatedMoveValue::U32(*i),
            (MoveValue::U64(i), FatType::U64) => AnnotatedMoveValue::U64(*i),
            (MoveValue::U128(i), FatType::U128) => AnnotatedMoveValue::U128(*i),
            (MoveValue::U256(i), FatType::U256) => AnnotatedMoveValue::U256(*i),
            (MoveValue::Address(a), FatType::Address) => AnnotatedMoveValue::Address(*a),
            (MoveValue::Vector(a), FatType::Vector(ty)) => match ty.as_ref() {
                FatType::U8 => AnnotatedMoveValue::Bytes(
                    a.iter()
                        .map(|v| match v {
                            MoveValue::U8(i) => Ok(*i),
                            _ => Err(anyhow!("unexpected value type")),
                        })
                        .collect::<anyhow::Result<_>>()?,
                ),
                _ => AnnotatedMoveValue::Vector(
                    ty.type_tag(limit).unwrap(),
                    a.iter()
                        .map(|v| self.annotate_value(v, ty.as_ref(), limit))
                        .collect::<anyhow::Result<_>>()?,
                ),
            },
            (MoveValue::Struct(s), FatType::Struct(ty)) => {
                AnnotatedMoveValue::Struct(self.annotate_struct(s, ty.as_ref(), limit)?)
            },
            (MoveValue::U8(_), _)
            | (MoveValue::U64(_), _)
            | (MoveValue::U128(_), _)
            | (MoveValue::Bool(_), _)
            | (MoveValue::Address(_), _)
            | (MoveValue::Vector(_), _)
            | (MoveValue::Struct(_), _)
            | (MoveValue::Signer(_), _)
            | (MoveValue::U16(_), _)
            | (MoveValue::U32(_), _)
            | (MoveValue::U256(_), _) => {
                return Err(anyhow!(
                    "Cannot annotate value {:?} with type {:?}",
                    value,
                    ty
                ));
            },
        })
    }
}

fn find_struct_def_in_module(
    module: &CompiledModule,
    name: &IdentStr,
) -> anyhow::Result<StructDefinitionIndex> {
    for (i, defs) in module.struct_defs().iter().enumerate() {
        let st_handle = module.struct_handle_at(defs.struct_handle);
        if module.identifier_at(st_handle.name) == name {
            return Ok(StructDefinitionIndex::new(i as u16));
        }
    }
    Err(anyhow!(
        "Struct {:?} not found in {:?}",
        name,
        module.self_id()
    ))
}

fn into_vm_status(e: PartialVMError) -> VMStatus {
    e.finish(Location::Undefined).into_vm_status()
}

fn write_indent(f: &mut Formatter, indent: u64) -> std::fmt::Result {
    for _i in 0..indent {
        write!(f, " ")?;
    }
    Ok(())
}

fn pretty_print_value(
    f: &mut Formatter,
    value: &AnnotatedMoveValue,
    indent: u64,
) -> std::fmt::Result {
    match value {
        AnnotatedMoveValue::Bool(b) => write!(f, "{}", b),
        AnnotatedMoveValue::U8(v) => write!(f, "{}u8", v),
        AnnotatedMoveValue::U16(v) => write!(f, "{}u16", v),
        AnnotatedMoveValue::U32(v) => write!(f, "{}u32", v),
        AnnotatedMoveValue::U64(v) => write!(f, "{}", v),
        AnnotatedMoveValue::U128(v) => write!(f, "{}u128", v),
        AnnotatedMoveValue::U256(v) => write!(f, "{}u256", v),
        AnnotatedMoveValue::Address(a) => write!(f, "{}", a.short_str_lossless()),
        AnnotatedMoveValue::Vector(_, v) => {
            writeln!(f, "[")?;
            for value in v.iter() {
                write_indent(f, indent + 4)?;
                pretty_print_value(f, value, indent + 4)?;
                writeln!(f, ",")?;
            }
            write_indent(f, indent)?;
            write!(f, "]")
        },
        AnnotatedMoveValue::Bytes(v) => write!(f, "{}", hex::encode(v)),
        AnnotatedMoveValue::Struct(s) => pretty_print_struct(f, s, indent),
    }
}

fn pretty_print_struct(
    f: &mut Formatter,
    value: &AnnotatedMoveStruct,
    indent: u64,
) -> std::fmt::Result {
    pretty_print_ability_modifiers(f, value.abilities)?;
    writeln!(f, "{} {{", value.ty_tag)?;
    for (field_name, v) in value.value.iter() {
        write_indent(f, indent + 4)?;
        write!(f, "{}: ", field_name)?;
        pretty_print_value(f, v, indent + 4)?;
        writeln!(f)?;
    }
    write_indent(f, indent)?;
    write!(f, "}}")
}

fn pretty_print_ability_modifiers(f: &mut Formatter, abilities: AbilitySet) -> std::fmt::Result {
    for ability in abilities {
        match ability {
            Ability::Copy => write!(f, "copy ")?,
            Ability::Drop => write!(f, "drop ")?,
            Ability::Store => write!(f, "store ")?,
            Ability::Key => write!(f, "key ")?,
        }
    }
    Ok(())
}

impl serde::Serialize for AnnotatedMoveStruct {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_map(Some(self.value.len()))?;
        for (f, v) in &self.value {
            s.serialize_entry(f, v)?
        }
        s.end()
    }
}

impl serde::Serialize for AnnotatedMoveValue {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use AnnotatedMoveValue::*;
        match self {
            U8(n) => serializer.serialize_u8(*n),
            U16(n) => serializer.serialize_u16(*n),
            U32(n) => serializer.serialize_u32(*n),
            U64(n) => serializer.serialize_u64(*n),
            U128(n) => {
                // TODO: we could use serializer.serialize_u128 here, but it requires the serde_json
                // arbitrary_precision, which breaks some existing json-rpc test. figure
                // out what's going on or come up with a better workaround
                if let Ok(i) = u64::try_from(*n) {
                    serializer.serialize_u64(i)
                } else {
                    serializer.serialize_bytes(&n.to_le_bytes())
                }
            },
            U256(n) => {
                // Copying logic & reasoning from above because if u128 is needs arb precision, u256 should too
                if let Ok(i) = u64::try_from(*n) {
                    serializer.serialize_u64(i)
                } else {
                    serializer.serialize_bytes(&n.to_le_bytes())
                }
            },
            Bool(b) => serializer.serialize_bool(*b),
            Address(a) => a.short_str_lossless().serialize(serializer),
            Vector(t, vals) => {
                assert_ne!(t, &TypeTag::U8);
                let mut vec = serializer.serialize_seq(Some(vals.len()))?;
                for v in vals {
                    vec.serialize_element(v)?;
                }
                vec.end()
            },
            Bytes(v) => {
                // try to deserialize as utf8, fall back to hex with if we can't
                let utf8_str = std::str::from_utf8(v);
                if let Ok(s) = utf8_str {
                    if s.chars().any(|c| c.is_ascii_control()) {
                        // has control characters; probably bytes
                        serializer.serialize_str(&hex::encode(v))
                    } else {
                        serializer.serialize_str(s)
                    }
                } else {
                    serializer.serialize_str(&hex::encode(v))
                }
            },
            Struct(s) => s.serialize(serializer),
        }
    }
}

impl Display for AnnotatedMoveValue {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        pretty_print_value(f, self, 0)
    }
}

impl Display for AnnotatedMoveStruct {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        pretty_print_struct(f, self, 0)
    }
}
