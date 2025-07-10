// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::fat_type::{
    FatFunctionType, FatStructLayout, FatStructType, FatType, WrappedAbilitySet,
};
use anyhow::{anyhow, bail};
pub use limit::Limiter;
use move_binary_format::{
    access::{ModuleAccess, ScriptAccess},
    binary_views::BinaryIndexedView,
    errors::{Location, PartialVMError},
    file_format::{
        CompiledScript, FieldDefinition, SignatureToken, StructDefinitionIndex,
        StructFieldInformation, StructHandleIndex,
    },
    views::FunctionHandleView,
    CompiledModule,
};
use move_bytecode_utils::{compiled_module_viewer::CompiledModuleView, layout::TypeLayoutBuilder};
use move_core_types::{
    ability::{Ability, AbilitySet},
    account_address::AccountAddress,
    function::{ClosureMask, MoveClosure},
    identifier::{IdentStr, Identifier},
    language_storage::{FunctionParamOrReturnTag, ModuleId, StructTag, TypeTag},
    transaction_argument::{convert_txn_args, TransactionArgument},
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
    pub variant_info: Option<(u16, Identifier)>,
    pub value: Vec<(Identifier, AnnotatedMoveValue)>,
}

/// Used to represent raw struct data, with struct name and field names. This stems
/// from closure capture values for which only the serialization layout is known.
#[derive(Clone, Debug)]
pub struct RawMoveStruct {
    pub variant_info: Option<u16>,
    pub field_values: Vec<AnnotatedMoveValue>,
}

/// Used to represent an annotated closure. The `captured` values will have only
/// `RawMoveStruct` information and are not fully decorated.
#[derive(Clone, Debug)]
pub struct AnnotatedMoveClosure {
    pub module_id: ModuleId,
    pub fun_id: Identifier,
    pub ty_args: Vec<TypeTag>,
    pub mask: ClosureMask,
    pub captured: Vec<AnnotatedMoveValue>,
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
    // NOTE: Added in v8
    Closure(AnnotatedMoveClosure),
    RawStruct(RawMoveStruct),
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

    pub fn view_script_arguments(
        &self,
        script_bytes: &[u8],
        args: &[TransactionArgument],
        ty_args: &[TypeTag],
    ) -> anyhow::Result<Vec<AnnotatedMoveValue>> {
        let mut limit = Limiter::default();
        let compiled_script = CompiledScript::deserialize(script_bytes)
            .map_err(|err| anyhow!("Failed to deserialzie script: {:?}", err))?;
        let param_tys = compiled_script
            .signature_at(compiled_script.parameters)
            .0
            .iter()
            .map(|tok| {
                self.resolve_signature(BinaryIndexedView::Script(&compiled_script), tok, &mut limit)
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        let args_bytes = convert_txn_args(args);
        self.view_arguments_impl(&param_tys, ty_args, &args_bytes, &mut limit)
    }

    pub fn view_function_arguments(
        &self,
        module: &ModuleId,
        function: &IdentStr,
        ty_args: &[TypeTag],
        args: &[Vec<u8>],
    ) -> anyhow::Result<Vec<AnnotatedMoveValue>> {
        let mut limit = Limiter::default();
        let param_tys = self.resolve_function_arguments(module, function, &mut limit)?;
        self.view_arguments_impl(&param_tys, ty_args, args, &mut limit)
    }

    fn view_arguments_impl(
        &self,
        param_tys: &[FatType],
        ty_args: &[TypeTag],
        args: &[Vec<u8>],
        limit: &mut Limiter,
    ) -> anyhow::Result<Vec<AnnotatedMoveValue>> {
        let types: Vec<&FatType> = param_tys
            .iter()
            .filter(|t| match t {
                FatType::Signer
                | FatType::Function(_)
                | FatType::Runtime(_)
                | FatType::RuntimeVariants(_) => false,
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
                ty.subst(&ty_args, limit)
                    .map_err(anyhow::Error::from)
                    .and_then(|fat_type| self.view_value_by_fat_type(&fat_type, &args[i], limit))
            })
            .collect::<anyhow::Result<Vec<AnnotatedMoveValue>>>()
    }

    fn resolve_function_arguments(
        &self,
        module: &ModuleId,
        function: &IdentStr,
        limit: &mut Limiter,
    ) -> anyhow::Result<Vec<FatType>> {
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
                    .map(|signature| {
                        self.resolve_signature(BinaryIndexedView::Module(m), signature, limit)
                    })
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
    ) -> anyhow::Result<(Option<Identifier>, Vec<(Identifier, MoveValue)>)> {
        let ty = self.resolve_struct(tag)?;
        let struct_def = (&ty).try_into().map_err(into_vm_status)?;
        Ok(match MoveStruct::simple_deserialize(blob, &struct_def)? {
            MoveStruct::Runtime(values) => {
                let (tag, field_names) = self.get_field_information(&ty, None)?;
                debug_assert_eq!(tag, None);
                (None, field_names.into_iter().zip(values).collect())
            },
            MoveStruct::RuntimeVariant(tag, values) => {
                let (variant_info, field_names) = self.get_field_information(&ty, Some(tag))?;
                (
                    variant_info.map(|(_, name)| name),
                    field_names.into_iter().zip(values).collect(),
                )
            },
            MoveStruct::WithFields(fields)
            | MoveStruct::WithTypes {
                _fields: fields, ..
            } => (None, fields),
            MoveStruct::WithVariantFields(name, _, fields) => (Some(name), fields),
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

        let make_fields =
            |fields: &[FieldDefinition], limit: &mut Limiter| -> anyhow::Result<Vec<FatType>> {
                fields
                    .iter()
                    .map(|field_def| {
                        self.resolve_signature(
                            BinaryIndexedView::Module(module),
                            &field_def.signature.0,
                            limit,
                        )
                    })
                    .collect::<anyhow::Result<_>>()
            };

        match &struct_def.field_information {
            StructFieldInformation::Native => Err(anyhow!("Unexpected Native Struct")),
            StructFieldInformation::Declared(fields) => Ok(FatStructType {
                address,
                module: module_name,
                name,
                abilities: WrappedAbilitySet(abilities),
                ty_args,
                layout: FatStructLayout::Singleton(make_fields(fields, limit)?),
            }),
            StructFieldInformation::DeclaredVariants(variants) => Ok(FatStructType {
                address,
                module: module_name,
                name,
                abilities: WrappedAbilitySet(abilities),
                ty_args,
                layout: FatStructLayout::Variants(
                    variants
                        .iter()
                        .map(|variant| make_fields(&variant.fields, limit))
                        .collect::<anyhow::Result<_>>()?,
                ),
            }),
        }
    }

    fn resolve_signature(
        &self,
        module: BinaryIndexedView,
        sig: &SignatureToken,
        limit: &mut Limiter,
    ) -> anyhow::Result<FatType> {
        let resolve_slice = |toks: &[SignatureToken], limit: &mut Limiter| {
            toks.iter()
                .map(|tok| self.resolve_signature(module, tok, limit))
                .collect::<anyhow::Result<Vec<_>>>()
        };
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
            SignatureToken::Function(args, results, abilities) => {
                FatType::Function(Box::new(FatFunctionType {
                    args: resolve_slice(args, limit)?,
                    results: resolve_slice(results, limit)?,
                    abilities: *abilities,
                }))
            },
            SignatureToken::Struct(idx) => {
                FatType::Struct(Box::new(self.resolve_struct_handle(module, *idx, limit)?))
            },
            SignatureToken::StructInstantiation(idx, toks) => {
                let struct_ty = self.resolve_struct_handle(module, *idx, limit)?;
                let args = resolve_slice(toks, limit)?;
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
        module: BinaryIndexedView,
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
<<<<<<< HEAD
            TypeTag::Function(..) => {
                // TODO(#15664) implement functions for fat types"
                todo!("functions for fat types")
=======
            TypeTag::Function(function_tag) => {
                let mut convert_tags = |tags: &[FunctionParamOrReturnTag]| {
                    tags.iter()
                        .map(|t| {
                            use FunctionParamOrReturnTag::*;
                            Ok(match t {
                                Reference(t) => {
                                    FatType::Reference(Box::new(self.resolve_type_impl(t, limit)?))
                                },
                                MutableReference(t) => FatType::MutableReference(Box::new(
                                    self.resolve_type_impl(t, limit)?,
                                )),
                                Value(t) => self.resolve_type_impl(t, limit)?,
                            })
                        })
                        .collect::<anyhow::Result<Vec<_>>>()
                };
                FatType::Function(Box::new(FatFunctionType {
                    args: convert_tags(&function_tag.args)?,
                    results: convert_tags(&function_tag.results)?,
                    abilities: function_tag.abilities,
                }))
>>>>>>> dee41276c9 ([api] Addressing function value todos (#17047))
            },
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
        let (variant_tag, field_values) = move_struct.optional_variant_and_fields();
        let (variant_info, field_names) = self.get_field_information(ty, variant_tag)?;
        if let Some((_, name)) = &variant_info {
            limit.charge(name.as_bytes().len())?;
        }
        for name in field_names.iter() {
            limit.charge(name.as_bytes().len())?;
        }

        let annotate_values = |values: &[MoveValue], tys: &[FatType], limit: &mut Limiter| {
            values
                .iter()
                .zip(tys)
                .zip(field_names)
                .map(|((v, ty), n)| self.annotate_value(v, ty, limit).map(|v| (n, v)))
                .collect::<anyhow::Result<Vec<_>>>()
        };

        match &ty.layout {
            FatStructLayout::Singleton(field_tys) => Ok(AnnotatedMoveStruct {
                abilities: ty.abilities.0,
                ty_tag: struct_tag,
                variant_info: None,
                value: annotate_values(field_values, field_tys, limit)?,
            }),
            FatStructLayout::Variants(variants) => match variant_tag {
                Some(tag) if (tag as usize) < variants.len() => {
                    let field_tys = &variants[tag as usize];
                    Ok(AnnotatedMoveStruct {
                        abilities: ty.abilities.0,
                        ty_tag: struct_tag,
                        variant_info,
                        value: annotate_values(field_values, field_tys, limit)?,
                    })
                },
                _ => bail!("type and value mismatch: malformed variant tag"),
            },
        }
    }

    fn get_field_information(
        &self,
        ty: &FatStructType,
        variant: Option<u16>,
    ) -> anyhow::Result<(Option<(u16, Identifier)>, Vec<Identifier>)> {
        let module_id = ModuleId::new(ty.address, ty.module.clone());
        let module = self.view_existing_module(&module_id)?;
        let module = module.borrow();
        let struct_def_idx = find_struct_def_in_module(module, ty.name.as_ident_str())?;
        let struct_def = module.struct_def_at(struct_def_idx);

        let ident_at = |name| module.identifier_at(name).to_owned();

        match (variant, &struct_def.field_information) {
            (_, StructFieldInformation::Native) => Err(anyhow!("Unexpected Native Struct")),
            (None, StructFieldInformation::Declared(defs)) => Ok((
                None,
                defs.iter()
                    .map(|field_def| ident_at(field_def.name))
                    .collect(),
            )),
            (Some(tag), StructFieldInformation::DeclaredVariants(variants))
                if (tag as usize) < variants.len() =>
            {
                let variant = &variants[tag as usize];
                Ok((
                    Some((tag, ident_at(variant.name))),
                    variant
                        .fields
                        .iter()
                        .map(|field_def| ident_at(field_def.name).to_owned())
                        .collect(),
                ))
            },
            _ => bail!("inconsistent layout information"),
        }
    }

    fn annotate_raw_struct(
        &self,
        move_struct: &MoveStruct,
        ty: &FatType,
        limit: &mut Limiter,
    ) -> anyhow::Result<RawMoveStruct> {
        let annotate_values = |values: &[MoveValue], tys: &[FatType], limit: &mut Limiter| {
            values
                .iter()
                .zip(tys)
                .map(|(v, ty)| self.annotate_value(v, ty, limit))
                .collect::<anyhow::Result<Vec<_>>>()
        };
        match (move_struct, ty) {
            (MoveStruct::Runtime(values), FatType::Runtime(tys)) if values.len() == tys.len() => {
                Ok(RawMoveStruct {
                    variant_info: None,
                    field_values: annotate_values(values, tys, limit)?,
                })
            },
            (MoveStruct::RuntimeVariant(tag, values), FatType::RuntimeVariants(vars))
                if (*tag as usize) < vars.len() && values.len() == vars[*tag as usize].len() =>
            {
                Ok(RawMoveStruct {
                    variant_info: Some(*tag),
                    field_values: annotate_values(values, &vars[*tag as usize], limit)?,
                })
            },
            _ => bail!("type and value mismatch: inconsistent raw struct information"),
        }
    }

    fn annotate_closure(
        &self,
        move_closure: &MoveClosure,
        _ty: &FatFunctionType,
        limit: &mut Limiter,
    ) -> anyhow::Result<AnnotatedMoveClosure> {
        let MoveClosure {
            module_id,
            fun_id,
            ty_args,
            mask,
            captured,
        } = move_closure;
        let captured = captured
            .iter()
            .map(|(layout, value)| {
                let fat_type = FatType::from_runtime_layout(layout, limit)
                    .map_err(|e| anyhow!("failed to annotate captured value: {}", e))?;
                self.annotate_value(value, &fat_type, limit)
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        Ok(AnnotatedMoveClosure {
            module_id: module_id.clone(),
            fun_id: fun_id.clone(),
            ty_args: ty_args.to_vec(),
            mask: *mask,
            captured,
        })
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
            (MoveValue::Struct(s), FatType::Runtime(_) | FatType::RuntimeVariants(_)) => {
                AnnotatedMoveValue::RawStruct(self.annotate_raw_struct(s, ty, limit)?)
            },
            (MoveValue::Closure(c), FatType::Function(ty)) => {
                AnnotatedMoveValue::Closure(self.annotate_closure(c, ty, limit)?)
            },
            (MoveValue::U8(_), _)
            | (MoveValue::U64(_), _)
            | (MoveValue::U128(_), _)
            | (MoveValue::Bool(_), _)
            | (MoveValue::Address(_), _)
            | (MoveValue::Vector(_), _)
            | (MoveValue::Struct(_), _)
            | (MoveValue::Closure(_), _)
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
        AnnotatedMoveValue::RawStruct(s) => pretty_print_raw_struct(f, s, indent),
        AnnotatedMoveValue::Closure(c) => pretty_print_closure(f, c, indent),
    }
}

fn pretty_print_struct(
    f: &mut Formatter,
    value: &AnnotatedMoveStruct,
    indent: u64,
) -> std::fmt::Result {
    pretty_print_ability_modifiers(f, value.abilities)?;
    write!(f, "{}", value.ty_tag)?;
    if let Some((_, name)) = &value.variant_info {
        write!(f, "::{}", name)?;
    }
    writeln!(f, " {{")?;
    for (field_name, v) in value.value.iter() {
        write_indent(f, indent + 4)?;
        write!(f, "{}: ", field_name)?;
        pretty_print_value(f, v, indent + 4)?;
        writeln!(f)?;
    }
    write_indent(f, indent)?;
    write!(f, "}}")
}

fn pretty_print_raw_struct(
    f: &mut Formatter,
    value: &RawMoveStruct,
    indent: u64,
) -> std::fmt::Result {
    let RawMoveStruct {
        variant_info,
        field_values: value,
    } = value;
    if let Some(var) = variant_info {
        write!(f, "#{}", var)?
    }
    writeln!(f, "{{")?;
    for elem in value {
        write_indent(f, indent + 4)?;
        pretty_print_value(f, elem, indent + 4)?;
        writeln!(f)?
    }
    write!(f, "}}")
}

fn pretty_print_closure(
    f: &mut Formatter,
    value: &AnnotatedMoveClosure,
    indent: u64,
) -> std::fmt::Result {
    let AnnotatedMoveClosure {
        module_id,
        fun_id,
        ty_args,
        mask,
        captured,
        ..
    } = value;
    write!(
        f,
        "0x{}::{}::{}",
        module_id.address.short_str_lossless(),
        module_id.name,
        fun_id
    )?;
    if !ty_args.is_empty() {
        let mut last_sep = "<";
        for ty in ty_args {
            f.write_str(last_sep)?;
            last_sep = ", ";
            write!(f, "{}", ty)?
        }
        write!(f, ">")?
    }
    write!(f, "(")?;
    let mut iter_captured = captured.iter();
    let mut first = true;
    for i in 0..mask.max_captured().unwrap_or_default() {
        if first {
            first = false
        } else {
            write!(f, ", ")?
        }
        if mask.is_captured(i) {
            if let Some(val) = iter_captured.next() {
                pretty_print_value(f, val, indent)?
            } else {
                write!(f, "<invalid closure mask>")?
            }
        } else {
            write!(f, "_")?
        }
    }
    write!(f, ")")
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
        let mut s;
        if let Some((_, name)) = &self.variant_info {
            s = serializer.serialize_map(Some(self.value.len() + 1))?;
            s.serialize_entry("$variant", name)?;
        } else {
            s = serializer.serialize_map(Some(self.value.len()))?;
        }
        for (f, v) in &self.value {
            s.serialize_entry(f, v)?
        }
        s.end()
    }
}

impl serde::Serialize for RawMoveStruct {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s;
        if let Some(tag) = &self.variant_info {
            s = serializer.serialize_map(Some(self.field_values.len() + 1))?;
            s.serialize_entry("$variant_tag", tag)?;
        } else {
            s = serializer.serialize_map(Some(self.field_values.len()))?;
        }
        for (i, v) in self.field_values.iter().enumerate() {
            s.serialize_entry(&i, v)?
        }
        s.end()
    }
}

impl serde::Serialize for AnnotatedMoveClosure {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let AnnotatedMoveClosure {
            module_id,
            fun_id,
            ty_args,
            mask,
            captured,
            ..
        } = self;
        let mut count = 2;
        if !ty_args.is_empty() {
            count += 1
        }
        if !captured.is_empty() {
            count += 1
        }
        let mut s = serializer.serialize_map(Some(count))?;
        s.serialize_entry(
            "$fun_name",
            &format!(
                "0x{}::{}::{}",
                module_id.address.short_str_lossless(),
                module_id.name,
                fun_id
            ),
        )?;
        if !ty_args.is_empty() {
            s.serialize_entry("$ty_args", ty_args)?;
        }
        s.serialize_entry("$mask", &mask.to_string())?;
        if !captured.is_empty() {
            s.serialize_entry("$captured", captured)?
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
            RawStruct(s) => s.serialize(serializer),
            Closure(c) => c.serialize(serializer),
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
