// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::module_cache::GetModule;
use anyhow::{anyhow, bail, Result};
use move_binary_format::{
    access::ModuleAccess,
    file_format::{SignatureToken, StructDefinition, StructFieldInformation, StructHandleIndex},
    CompiledModule,
};
use move_core_types::{
    identifier::IdentStr,
    language_storage::{ModuleId, StructTag, TypeTag},
    value::{MoveFieldLayout, MoveStructLayout, MoveTypeLayout},
};
use std::{borrow::Borrow, fmt::Debug};

pub enum TypeLayoutBuilder {}
pub enum StructLayoutBuilder {}

#[derive(Copy, Clone, Debug)]
enum LayoutType {
    WithFields,
    Runtime,
}

impl TypeLayoutBuilder {
    /// Construct `TypeLayout` with fields from `t`.
    /// Panics if `resolver` cannot resolve a module whose types are referenced directly or
    /// transitively by `t`.
    pub fn build_with_fields(t: &TypeTag, resolver: &impl GetModule) -> Result<MoveTypeLayout> {
        Self::build(t, resolver, LayoutType::WithFields)
    }

    /// Construct a runtime `TypeLayout` from `t`.
    /// Panics if `resolver` cannot resolve a module whose types are referenced directly or
    /// transitively by `t`.
    pub fn build_runtime(t: &TypeTag, resolver: &impl GetModule) -> Result<MoveTypeLayout> {
        Self::build(t, resolver, LayoutType::Runtime)
    }

    fn build(
        t: &TypeTag,
        resolver: &impl GetModule,
        layout_type: LayoutType,
    ) -> Result<MoveTypeLayout> {
        use TypeTag::*;
        Ok(match t {
            Bool => MoveTypeLayout::Bool,
            U8 => MoveTypeLayout::U8,
            U64 => MoveTypeLayout::U64,
            U128 => MoveTypeLayout::U128,
            Address => MoveTypeLayout::Address,
            Signer => bail!("Type layouts cannot contain signer"),
            Vector(elem_t) => {
                MoveTypeLayout::Vector(Box::new(Self::build(elem_t, resolver, layout_type)?))
            }
            Struct(s) => {
                MoveTypeLayout::Struct(StructLayoutBuilder::build(s, resolver, layout_type)?)
            }
        })
    }

    fn build_from_signature_token(
        m: &CompiledModule,
        s: &SignatureToken,
        type_arguments: &[MoveTypeLayout],
        resolver: &impl GetModule,
        layout_type: LayoutType,
    ) -> Result<MoveTypeLayout> {
        use SignatureToken::*;
        Ok(match s {
            Vector(t) => MoveTypeLayout::Vector(Box::new(Self::build_from_signature_token(
                m,
                t,
                type_arguments,
                resolver,
                layout_type,
            )?)),
            Struct(shi) => MoveTypeLayout::Struct(StructLayoutBuilder::build_from_handle_idx(
                m,
                *shi,
                vec![],
                resolver,
                layout_type,
            )?),
            StructInstantiation(shi, type_actuals) => {
                let actual_layouts = type_actuals
                    .iter()
                    .map(|t| {
                        Self::build_from_signature_token(
                            m,
                            t,
                            type_arguments,
                            resolver,
                            layout_type,
                        )
                    })
                    .collect::<Result<Vec<_>>>()?;
                MoveTypeLayout::Struct(StructLayoutBuilder::build_from_handle_idx(
                    m,
                    *shi,
                    actual_layouts,
                    resolver,
                    layout_type,
                )?)
            }
            TypeParameter(i) => type_arguments[*i as usize].clone(),
            Bool => MoveTypeLayout::Bool,
            U8 => MoveTypeLayout::U8,
            U64 => MoveTypeLayout::U64,
            U128 => MoveTypeLayout::U128,
            Address => MoveTypeLayout::Address,
            Signer => bail!("Type layouts cannot contain signer"),
            Reference(_) | MutableReference(_) => bail!("Type layouts cannot contain references"),
        })
    }
}

impl StructLayoutBuilder {
    pub fn build_runtime(s: &StructTag, resolver: &impl GetModule) -> Result<MoveStructLayout> {
        Self::build(s, resolver, LayoutType::Runtime)
    }

    pub fn build_with_fields(s: &StructTag, resolver: &impl GetModule) -> Result<MoveStructLayout> {
        Self::build(s, resolver, LayoutType::WithFields)
    }

    /// Construct an expanded `TypeLayout` from `s`.
    /// Panics if `resolver` cannot resolved a module whose types are referenced directly or
    /// transitively by `s`.
    fn build(
        s: &StructTag,
        resolver: &impl GetModule,
        layout_type: LayoutType,
    ) -> Result<MoveStructLayout> {
        let type_arguments = s
            .type_params
            .iter()
            .map(|t| TypeLayoutBuilder::build(t, resolver, layout_type))
            .collect::<Result<Vec<MoveTypeLayout>>>()?;
        Self::build_from_name(
            &s.module_id(),
            &s.name,
            type_arguments,
            resolver,
            layout_type,
        )
    }

    fn build_from_definition(
        m: &CompiledModule,
        s: &StructDefinition,
        type_arguments: Vec<MoveTypeLayout>,
        resolver: &impl GetModule,
        layout_type: LayoutType,
    ) -> Result<MoveStructLayout> {
        assert_eq!(
            m.struct_handle_at(s.struct_handle).type_parameters.len(),
            type_arguments.len(),
            "Wrong number of type arguments for struct",
        );
        match &s.field_information {
            StructFieldInformation::Native => {
                bail!("Can't extract fields for native struct")
            }
            StructFieldInformation::Declared(fields) => {
                let layouts = fields
                    .iter()
                    .map(|f| {
                        TypeLayoutBuilder::build_from_signature_token(
                            m,
                            &f.signature.0,
                            &type_arguments,
                            resolver,
                            layout_type,
                        )
                    })
                    .collect::<Result<Vec<MoveTypeLayout>>>()?;
                Ok(match layout_type {
                    LayoutType::Runtime => MoveStructLayout::Runtime(layouts),
                    LayoutType::WithFields => MoveStructLayout::WithFields(
                        fields
                            .iter()
                            .map(|f| m.identifier_at(f.name).to_owned())
                            .zip(layouts)
                            .map(|(name, layout)| MoveFieldLayout::new(name, layout))
                            .collect(),
                    ),
                })
            }
        }
    }

    fn build_from_name(
        declaring_module: &ModuleId,
        name: &IdentStr,
        type_arguments: Vec<MoveTypeLayout>,
        resolver: &impl GetModule,
        layout_type: LayoutType,
    ) -> Result<MoveStructLayout> {
        let module = resolver
            .get_module_by_id(declaring_module)
            .map_err(|_| anyhow!("Error while resolving module {}", declaring_module))?
            .ok_or_else(|| anyhow!("Failed to get module {}", declaring_module))?;
        let m = module.borrow();
        let def = m
            .struct_defs
            .iter()
            .find(|def| {
                let handle = m.struct_handle_at(def.struct_handle);
                name == m.identifier_at(handle.name)
            })
            .ok_or_else(|| {
                anyhow!(
                    "Could not find struct named {} in module {}",
                    name,
                    declaring_module
                )
            })?;
        Self::build_from_definition(m, def, type_arguments, resolver, layout_type)
    }

    fn build_from_handle_idx(
        m: &CompiledModule,
        s: StructHandleIndex,
        type_arguments: Vec<MoveTypeLayout>,
        resolver: &impl GetModule,
        layout_type: LayoutType,
    ) -> Result<MoveStructLayout> {
        if let Some(def) = m.find_struct_def(s) {
            // declared internally
            Self::build_from_definition(m, def, type_arguments, resolver, layout_type)
        } else {
            let handle = m.struct_handle_at(s);
            let name = m.identifier_at(handle.name);
            let declaring_module = m.module_id_for_handle(m.module_handle_at(handle.module));
            // declared externally
            Self::build_from_name(
                &declaring_module,
                name,
                type_arguments,
                resolver,
                layout_type,
            )
        }
    }
}
