// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

#![allow(deprecated)]

use crate::compiled_module_viewer::CompiledModuleView;
use anyhow::{anyhow, bail};
use move_binary_format::{
    access::ModuleAccess,
    file_format::{SignatureToken, StructDefinition, StructFieldInformation, StructHandleIndex},
    CompiledModule,
};
use move_core_types::{
    identifier::{IdentStr, Identifier},
    language_storage::{ModuleId, StructTag, TypeTag, LEGACY_OPTION_VEC},
    value::{MoveFieldLayout, MoveStructLayout, MoveTypeLayout},
};
use std::{borrow::Borrow, convert::TryInto, fmt::Debug};

pub enum TypeLayoutBuilder {}
pub enum StructLayoutBuilder {}

#[derive(Copy, Clone, Debug)]
enum LayoutType {
    WithTypes,
    WithFields,
    Runtime,
}

impl TypeLayoutBuilder {
    /// Construct a WithTypes `TypeLayout` with fields from `t`.
    /// Panics if `resolver` cannot resolve a module whose types are referenced directly or
    /// transitively by `t`
    pub fn build_with_types(
        t: &TypeTag,
        compiled_module_view: &impl CompiledModuleView,
    ) -> anyhow::Result<MoveTypeLayout> {
        Self::build(t, compiled_module_view, LayoutType::WithTypes)
    }

    /// Construct a WithFields `TypeLayout` with fields from `t`.
    /// Panics if `resolver` cannot resolve a module whose types are referenced directly or
    /// transitively by `t`.
    pub fn build_with_fields(
        t: &TypeTag,
        compiled_module_view: &impl CompiledModuleView,
    ) -> anyhow::Result<MoveTypeLayout> {
        Self::build(t, compiled_module_view, LayoutType::WithFields)
    }

    /// Construct a runtime `TypeLayout` from `t`.
    /// Panics if `resolver` cannot resolve a module whose types are referenced directly or
    /// transitively by `t`.
    pub fn build_runtime(
        t: &TypeTag,
        compiled_module_view: &impl CompiledModuleView,
    ) -> anyhow::Result<MoveTypeLayout> {
        Self::build(t, compiled_module_view, LayoutType::Runtime)
    }

    fn build(
        t: &TypeTag,
        compiled_module_view: &impl CompiledModuleView,
        layout_type: LayoutType,
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
            I8 => MoveTypeLayout::I8,
            I16 => MoveTypeLayout::I16,
            I32 => MoveTypeLayout::I32,
            I64 => MoveTypeLayout::I64,
            I128 => MoveTypeLayout::I128,
            I256 => MoveTypeLayout::I256,
            Address => MoveTypeLayout::Address,
            Signer => bail!("Type layouts cannot contain signer"),
            Vector(elem_t) => MoveTypeLayout::Vector(Box::new(Self::build(
                elem_t,
                compiled_module_view,
                layout_type,
            )?)),
            Struct(s) => MoveTypeLayout::Struct(StructLayoutBuilder::build(
                s,
                compiled_module_view,
                layout_type,
            )?),
            Function(_) => MoveTypeLayout::Function,
        })
    }

    fn build_from_signature_token(
        m: &CompiledModule,
        s: &SignatureToken,
        type_arguments: &[MoveTypeLayout],
        compiled_module_view: &impl CompiledModuleView,
        layout_type: LayoutType,
    ) -> anyhow::Result<MoveTypeLayout> {
        use SignatureToken::*;
        Ok(match s {
            Function(..) => bail!("function types NYI for MoveTypeLayout"),
            Vector(t) => MoveTypeLayout::Vector(Box::new(Self::build_from_signature_token(
                m,
                t,
                type_arguments,
                compiled_module_view,
                layout_type,
            )?)),
            Struct(shi) => MoveTypeLayout::Struct(StructLayoutBuilder::build_from_handle_idx(
                m,
                *shi,
                vec![],
                compiled_module_view,
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
                            compiled_module_view,
                            layout_type,
                        )
                    })
                    .collect::<anyhow::Result<Vec<_>>>()?;
                MoveTypeLayout::Struct(StructLayoutBuilder::build_from_handle_idx(
                    m,
                    *shi,
                    actual_layouts,
                    compiled_module_view,
                    layout_type,
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
            I8 => MoveTypeLayout::I8,
            I16 => MoveTypeLayout::I16,
            I32 => MoveTypeLayout::I32,
            I64 => MoveTypeLayout::I64,
            I128 => MoveTypeLayout::I128,
            I256 => MoveTypeLayout::I256,
            Address => MoveTypeLayout::Address,
            Signer => bail!("Type layouts cannot contain signer"),
            Reference(_) | MutableReference(_) => bail!("Type layouts cannot contain references"),
        })
    }
}

impl StructLayoutBuilder {
    pub fn build_runtime(
        s: &StructTag,
        compiled_module_view: &impl CompiledModuleView,
    ) -> anyhow::Result<MoveStructLayout> {
        Self::build(s, compiled_module_view, LayoutType::Runtime)
    }

    pub fn build_with_fields(
        s: &StructTag,
        compiled_module_view: &impl CompiledModuleView,
    ) -> anyhow::Result<MoveStructLayout> {
        Self::build(s, compiled_module_view, LayoutType::WithFields)
    }

    /// Construct an expanded `TypeLayout` from `s`.
    /// Panics if `module_viewer` cannot resolve a module whose types are referenced directly or
    /// transitively by `s`.
    fn build(
        s: &StructTag,
        compiled_module_view: &impl CompiledModuleView,
        layout_type: LayoutType,
    ) -> anyhow::Result<MoveStructLayout> {
        let type_arguments = s
            .type_args
            .iter()
            .map(|t| TypeLayoutBuilder::build(t, compiled_module_view, layout_type))
            .collect::<anyhow::Result<Vec<MoveTypeLayout>>>()?;
        Self::build_from_name(
            &s.module_id(),
            &s.name,
            type_arguments,
            compiled_module_view,
            layout_type,
        )
    }

    fn build_from_definition(
        m: &CompiledModule,
        s: &StructDefinition,
        type_arguments: Vec<MoveTypeLayout>,
        compiled_module_view: &impl CompiledModuleView,
        layout_type: LayoutType,
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
                            layout_type,
                        )
                    })
                    .collect::<anyhow::Result<Vec<MoveTypeLayout>>>()?;
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
                    LayoutType::WithTypes => {
                        let mid = m.self_id();
                        let type_args = type_arguments
                            .iter()
                            .map(|t| t.try_into())
                            .collect::<anyhow::Result<Vec<TypeTag>>>()?;
                        let type_ = StructTag {
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
                        MoveStructLayout::WithTypes { type_, fields }
                    },
                })
            },
            StructFieldInformation::DeclaredVariants(variant_definitions) => {
                if m.self_id().is_option() {
                    match layout_type {
                        LayoutType::WithTypes => {
                            let mid = m.self_id();
                            let type_args = type_arguments
                                .iter()
                                .map(|t| t.try_into())
                                .collect::<anyhow::Result<Vec<TypeTag>>>()?;
                            let type_ = StructTag {
                                address: *mid.address(),
                                module: mid.name().to_owned(),
                                name: m.identifier_at(s_handle.name).to_owned(),
                                type_args,
                            };
                            if variant_definitions.len() != 2 {
                                bail!("Option must have exactly two variants");
                            }
                            let variant = &variant_definitions[1];
                            let name = m.identifier_at(variant.name).to_owned();
                            if name.as_str() == "Some" {
                                if variant.fields.len() != 1 {
                                    bail!("Variant `Some` must have exactly one field");
                                }
                                let layout = TypeLayoutBuilder::build_from_signature_token(
                                    m,
                                    &variant.fields[0].signature.0,
                                    &type_arguments,
                                    compiled_module_view,
                                    layout_type,
                                )?;
                                let vector_layout = MoveTypeLayout::Vector(Box::new(layout));
                                let identifier = Identifier::new(LEGACY_OPTION_VEC)?;
                                let fields = vec![MoveFieldLayout::new(identifier, vector_layout)];
                                return Ok(MoveStructLayout::WithTypes { type_, fields });
                            } else {
                                bail!("Variant name must be `Some`");
                            }
                        },
                        _ => {
                            bail!("enum variants not yet supported by layouts");
                        },
                    }
                }
                bail!("enum variants not yet supported by layouts")
            },
        }
    }

    fn build_from_name(
        declaring_module: &ModuleId,
        name: &IdentStr,
        type_arguments: Vec<MoveTypeLayout>,
        module_viewer: &impl CompiledModuleView,
        layout_type: LayoutType,
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
        Self::build_from_definition(
            module.borrow(),
            def,
            type_arguments,
            module_viewer,
            layout_type,
        )
    }

    fn build_from_handle_idx(
        m: &CompiledModule,
        s: StructHandleIndex,
        type_arguments: Vec<MoveTypeLayout>,
        compiled_module_view: &impl CompiledModuleView,
        layout_type: LayoutType,
    ) -> anyhow::Result<MoveStructLayout> {
        if let Some(def) = m.find_struct_def(s) {
            // declared internally
            Self::build_from_definition(m, def, type_arguments, compiled_module_view, layout_type)
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
                layout_type,
            )
        }
    }
}
