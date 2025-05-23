// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::compiled_module_viewer::CompiledModuleView;
use anyhow::{anyhow, bail};
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
use std::{borrow::Borrow, convert::TryInto};

pub enum TypeLayoutBuilder {}
pub enum StructLayoutBuilder {}

impl TypeLayoutBuilder {
    /// Construct a WithTypes `TypeLayout` with fields from `t`.
    /// Panics if `resolver` cannot resolve a module whose types are referenced directly or
    /// transitively by `t`
    pub fn build_with_types(
        t: &TypeTag,
        compiled_module_view: &impl CompiledModuleView,
    ) -> anyhow::Result<MoveTypeLayout> {
        Self::build(t, compiled_module_view)
    }

    fn build(
        t: &TypeTag,
        compiled_module_view: &impl CompiledModuleView,
    ) -> anyhow::Result<MoveTypeLayout> {
        use TypeTag::*;
        Ok(match t {
            Bool => MoveTypeLayout::Bool,
            U8 => MoveTypeLayout::U8,
            U16 => MoveTypeLayout::U16,
            U32 => MoveTypeLayout::U32,
            U64 => MoveTypeLayout::U64,
            U128 => MoveTypeLayout::U128,
            U256 => MoveTypeLayout::U256,
            Address => MoveTypeLayout::Address,
            Signer => bail!("Type layouts cannot contain signer"),
            Vector(elem_t) => {
                MoveTypeLayout::Vector(Box::new(Self::build(elem_t, compiled_module_view)?))
            },
            Struct(s) => {
                MoveTypeLayout::Struct(StructLayoutBuilder::build(s, compiled_module_view)?)
            },
            Function(_) => MoveTypeLayout::Function,
        })
    }

    fn build_from_signature_token(
        m: &CompiledModule,
        s: &SignatureToken,
        type_arguments: &[MoveTypeLayout],
        compiled_module_view: &impl CompiledModuleView,
    ) -> anyhow::Result<MoveTypeLayout> {
        use SignatureToken::*;
        Ok(match s {
            Function(..) => bail!("function types NYI for MoveTypeLayout"),
            Vector(t) => MoveTypeLayout::Vector(Box::new(Self::build_from_signature_token(
                m,
                t,
                type_arguments,
                compiled_module_view,
            )?)),
            Struct(shi) => MoveTypeLayout::Struct(StructLayoutBuilder::build_from_handle_idx(
                m,
                *shi,
                vec![],
                compiled_module_view,
            )?),
            StructInstantiation(shi, type_actuals) => {
                let actual_layouts = type_actuals
                    .iter()
                    .map(|t| {
                        Self::build_from_signature_token(m, t, type_arguments, compiled_module_view)
                    })
                    .collect::<anyhow::Result<Vec<_>>>()?;
                MoveTypeLayout::Struct(StructLayoutBuilder::build_from_handle_idx(
                    m,
                    *shi,
                    actual_layouts,
                    compiled_module_view,
                )?)
            },
            TypeParameter(i) => type_arguments[*i as usize].clone(),
            Bool => MoveTypeLayout::Bool,
            U8 => MoveTypeLayout::U8,
            U16 => MoveTypeLayout::U16,
            U32 => MoveTypeLayout::U32,
            U64 => MoveTypeLayout::U64,
            U128 => MoveTypeLayout::U128,
            U256 => MoveTypeLayout::U256,
            Address => MoveTypeLayout::Address,
            Signer => bail!("Type layouts cannot contain signer"),
            Reference(_) | MutableReference(_) => bail!("Type layouts cannot contain references"),
        })
    }
}

impl StructLayoutBuilder {
    /// Construct an expanded `TypeLayout` from `s`.
    /// Panics if `module_viewer` cannot resolve a module whose types are referenced directly or
    /// transitively by `s`.
    fn build(
        s: &StructTag,
        compiled_module_view: &impl CompiledModuleView,
    ) -> anyhow::Result<MoveStructLayout> {
        let type_arguments = s
            .type_args
            .iter()
            .map(|t| TypeLayoutBuilder::build(t, compiled_module_view))
            .collect::<anyhow::Result<Vec<MoveTypeLayout>>>()?;
        Self::build_from_name(
            &s.module_id(),
            &s.name,
            type_arguments,
            compiled_module_view,
        )
    }

    fn build_from_definition(
        m: &CompiledModule,
        s: &StructDefinition,
        type_arguments: Vec<MoveTypeLayout>,
        compiled_module_view: &impl CompiledModuleView,
    ) -> anyhow::Result<MoveStructLayout> {
        let s_handle = m.struct_handle_at(s.struct_handle);
        if s_handle.type_parameters.len() != type_arguments.len() {
            bail!("Wrong number of type arguments for struct")
        }
        match &s.field_information {
            StructFieldInformation::Native => {
                bail!("Can't extract fields for native struct")
            },
            StructFieldInformation::Declared(fields) => {
                let layouts = fields
                    .iter()
                    .map(|f| {
                        TypeLayoutBuilder::build_from_signature_token(
                            m,
                            &f.signature.0,
                            &type_arguments,
                            compiled_module_view,
                        )
                    })
                    .collect::<anyhow::Result<Vec<MoveTypeLayout>>>()?;

                let mid = m.self_id();
                let type_args = type_arguments
                    .iter()
                    .map(|t| t.try_into())
                    .collect::<anyhow::Result<Vec<TypeTag>>>()?;
                let struct_tag = StructTag {
                    address: *mid.address(),
                    module: mid.name().to_owned(),
                    name: m.identifier_at(s_handle.name).to_owned(),
                    type_args,
                };
                let fields = fields
                    .iter()
                    .map(|f| m.identifier_at(f.name).to_owned())
                    .zip(layouts)
                    .map(|(name, layout)| MoveFieldLayout::new(name, layout))
                    .collect();
                Ok(MoveStructLayout::Decorated { struct_tag, fields })
            },
            StructFieldInformation::DeclaredVariants(..) => {
                bail!("enum variants not yet supported by layouts")
            },
        }
    }

    fn build_from_name(
        declaring_module: &ModuleId,
        name: &IdentStr,
        type_arguments: Vec<MoveTypeLayout>,
        module_viewer: &impl CompiledModuleView,
    ) -> anyhow::Result<MoveStructLayout> {
        let module = match module_viewer.view_compiled_module(declaring_module) {
            Err(_) | Ok(None) => bail!("Could not find module"),
            Ok(Some(m)) => m,
        };
        let def = module
            .borrow()
            .find_struct_def_by_name(name)
            .ok_or_else(|| {
                anyhow!(
                    "Could not find struct named {} in module {}",
                    name,
                    declaring_module
                )
            })?;
        Self::build_from_definition(module.borrow(), def, type_arguments, module_viewer)
    }

    fn build_from_handle_idx(
        m: &CompiledModule,
        s: StructHandleIndex,
        type_arguments: Vec<MoveTypeLayout>,
        compiled_module_view: &impl CompiledModuleView,
    ) -> anyhow::Result<MoveStructLayout> {
        if let Some(def) = m.find_struct_def(s) {
            // declared internally
            Self::build_from_definition(m, def, type_arguments, compiled_module_view)
        } else {
            let handle = m.struct_handle_at(s);
            let name = m.identifier_at(handle.name);
            let declaring_module = m.module_id_for_handle(m.module_handle_at(handle.module));
            // declared externally
            Self::build_from_name(
                &declaring_module,
                name,
                type_arguments,
                compiled_module_view,
            )
        }
    }
}
