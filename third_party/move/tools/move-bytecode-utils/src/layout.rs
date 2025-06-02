// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

#![allow(deprecated)]

use crate::compiled_module_viewer::CompiledModuleView;
use anyhow::{anyhow, bail};
use move_binary_format::{
    access::ModuleAccess,
    file_format::{SignatureToken, StructDefinition, StructFieldInformation, StructHandleIndex},
    normalized::{Struct, Type},
    CompiledModule,
};
use move_core_types::{
    account_address::AccountAddress,
    identifier::{IdentStr, Identifier},
    language_storage::{ModuleId, StructTag, TypeTag},
    value::{MoveFieldLayout, MoveStructLayout, MoveTypeLayout},
};
use serde_reflection::{ContainerFormat, Format, Named, Registry};
use std::{
    borrow::Borrow,
    collections::BTreeMap,
    convert::TryInto,
    fmt::{Debug, Write},
};

/// Name of the Move `address` type in the serde registry
const ADDRESS: &str = "AccountAddress";

/// Name of the Move `signer` type in the serde registry
const SIGNER: &str = "Signer";

/// Name of the Move `u256` type in the serde registry
const U256_SERDE_NAME: &str = "u256";

/// Type for building a registry of serde-reflection friendly struct layouts for Move types.
/// The layouts created by this type are intended to be passed to the serde-generate tool to create
/// struct bindings for Move types in source languages that use Move-based services.
pub struct SerdeLayoutBuilder<'a, T> {
    registry: Registry,
    compiled_module_view: &'a T,
    config: SerdeLayoutConfig,
}

#[derive(Default)]
pub struct SerdeLayoutConfig {
    /// If separator is Some, replace all Move source syntax separators ("::" for address/struct/module name
    /// separation, "<", ">", and "," for generics separation) with this string.
    /// If separator is None, use the same syntax as Move source
    pub separator: Option<String>,
    /// If true, do not include addresses in fully qualified type names.
    /// If there is a name conflict (e.g., the registry we're building has both
    /// 0x1::M::T and 0x2::M::T), layout generation will fail when this option is true.
    pub omit_addresses: bool,
    /// If true, do not include phantom types in fully qualified type names, since they do not contribute to the layout
    /// E.g., if we have `struct S<phantom T> { u: 64 }` and try to generate bindings for this struct with `T = u8`,
    /// the name for `S` in the registry will be `S<u64>` when this option is false, and `S` when this option is true
    pub ignore_phantom_types: bool,
    /// The LayoutBuilder can operate in two modes: "deep" and "shallow".
    /// In shallow mode, generate a single layout for the struct or type passed in by the user
    /// (under the assumption that layouts for dependencies have been generated previously).
    /// In deep mode, it generate layouts for all of the (transitive) dependencies of the type passed
    /// in, as well as layouts for the Move ground types like `address` and `signer`. The result is a
    /// self-contained registry with no unbound typenames
    pub shallow: bool,
}

impl<'a, T: CompiledModuleView> SerdeLayoutBuilder<'a, T> {
    /// Create a `LayoutBuilder` with an empty registry and deep layout resolution
    pub fn new(compiled_module_view: &'a T) -> Self {
        Self {
            registry: Self::default_registry(),
            compiled_module_view,
            config: SerdeLayoutConfig::default(),
        }
    }

    /// Create a `LayoutBuilder` with an empty registry and shallow layout resolution
    pub fn new_with_config(compiled_module_view: &'a T, config: SerdeLayoutConfig) -> Self {
        Self {
            registry: Self::default_registry(),
            compiled_module_view,
            config,
        }
    }

    /// Return a registry containing layouts for all the Move ground types (e.g., address)
    pub fn default_registry() -> Registry {
        let mut registry = BTreeMap::new();
        // add Move ground types to registry (address, signer)
        let address_layout = Box::new(Format::TupleArray {
            content: Box::new(Format::U8),
            size: AccountAddress::LENGTH,
        });
        registry.insert(
            ADDRESS.to_string(),
            ContainerFormat::NewTypeStruct(address_layout.clone()),
        );
        registry.insert(
            SIGNER.to_string(),
            ContainerFormat::NewTypeStruct(address_layout),
        );

        registry
    }

    /// Get the registry of layouts generated so far
    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    /// Get the registry of layouts generated so far
    pub fn into_registry(self) -> Registry {
        self.registry
    }

    /// Add layouts for all types used in `t` to the registry
    pub fn build_type_layout(&mut self, t: TypeTag) -> anyhow::Result<Format> {
        self.build_normalized_type_layout(&Type::from(t), &Vec::new())
    }

    /// Add layouts for all types used in `t` to the registry
    pub fn build_struct_layout(&mut self, s: &StructTag) -> anyhow::Result<Format> {
        let serde_type_args = s
            .type_args
            .iter()
            .map(|t| self.build_type_layout(t.clone()))
            .collect::<anyhow::Result<Vec<Format>>>()?;
        self.build_struct_layout_(&s.module_id(), &s.name, &serde_type_args)
    }

    fn build_normalized_type_layout(
        &mut self,
        t: &Type,
        input_type_args: &[Format],
    ) -> anyhow::Result<Format> {
        use Type::*;
        Ok(match t {
            Bool => Format::Bool,
            U8 => Format::U8,
            U16 => Format::U16,
            U32 => Format::U32,
            U64 => Format::U64,
            U128 => Format::U128,
            U256 => Format::TypeName(U256_SERDE_NAME.to_string()),
            Address => Format::TypeName(ADDRESS.to_string()),
            Signer => Format::TypeName(SIGNER.to_string()),
            Struct {
                address,
                module,
                name,
                type_arguments,
            } => {
                let serde_type_args = type_arguments
                    .iter()
                    .map(|t| self.build_normalized_type_layout(t, input_type_args))
                    .collect::<anyhow::Result<Vec<Format>>>()?;
                let declaring_module = ModuleId::new(*address, module.clone());
                self.build_struct_layout_(&declaring_module, name, &serde_type_args)?
            },
            Vector(inner_t) => {
                if matches!(inner_t.as_ref(), U8) {
                    // specialize vector<u8> as bytes
                    Format::Bytes
                } else {
                    Format::Seq(Box::new(
                        self.build_normalized_type_layout(inner_t, input_type_args)?,
                    ))
                }
            },
            TypeParameter(i) => input_type_args[*i as usize].clone(),
            Reference(_) | MutableReference(_) => unreachable!(), // structs cannot store references
        })
    }

    fn build_struct_layout_(
        &mut self,
        module_id: &ModuleId,
        name: &Identifier,
        type_arguments: &[Format],
    ) -> anyhow::Result<Format> {
        // build a human-readable name for the struct type. this should do the same thing as
        // StructTag::display(), but it's not easy to use that code here

        let declaring_module = self
            .compiled_module_view
            .view_compiled_module(module_id)?
            .expect("Failed to resolve module");
        let def = declaring_module
            .borrow()
            .find_struct_def_by_name(name)
            .unwrap_or_else(|| {
                panic!(
                    "Could not find struct named {} in module {}",
                    name,
                    declaring_module.borrow().name()
                )
            });
        #[allow(deprecated)]
        let normalized_struct = Struct::new(declaring_module.borrow(), def)?.1;
        assert_eq!(
            normalized_struct.type_parameters.len(),
            type_arguments.len(),
            "Wrong number of type arguments for struct"
        );

        let generics: Vec<String> = type_arguments
            .iter()
            .zip(normalized_struct.type_parameters.iter())
            .filter_map(|(type_arg, type_param)| {
                if self.config.ignore_phantom_types && type_param.is_phantom {
                    // do not include phantom type arguments in the struct key, since they do not affect the struct layout
                    None
                } else {
                    Some(print_format_type(type_arg))
                }
            })
            .collect();
        let mut struct_key = String::new();
        if !self.config.omit_addresses {
            write!(
                struct_key,
                "{}{}",
                module_id.address(),
                self.config.separator.as_deref().unwrap_or("::")
            )
            .unwrap();
        }
        write!(
            struct_key,
            "{}{}{}",
            module_id.name(),
            self.config.separator.as_deref().unwrap_or("::"),
            name
        )
        .unwrap();
        if !generics.is_empty() {
            write!(
                struct_key,
                "{}{}{}",
                self.config.separator.as_deref().unwrap_or("<"),
                generics.join(self.config.separator.as_deref().unwrap_or(",")),
                self.config.separator.as_deref().unwrap_or(">")
            )
            .unwrap()
        }
        if self.config.shallow {
            return Ok(Format::TypeName(struct_key));
        }

        if let Some(old_struct) = self.registry.get(&struct_key) {
            if self.config.omit_addresses || self.config.separator.is_some() {
                // check for conflicts (e.g., 0x1::M::T and 0x2::M::T that both get stripped to M::T because
                // omit_addresses is on)
                if old_struct.clone()
                    != self.generate_serde_struct(normalized_struct, type_arguments)?
                {
                    panic!(
                        "Name conflict: multiple structs with name {}, but different addresses",
                        struct_key
                    )
                }
            }
        } else {
            // not found--generate and update registry
            let serde_struct = self.generate_serde_struct(normalized_struct, type_arguments)?;
            self.registry.insert(struct_key.clone(), serde_struct);
        }

        Ok(Format::TypeName(struct_key))
    }

    fn generate_serde_struct(
        &mut self,
        normalized_struct: Struct,
        type_arguments: &[Format],
    ) -> anyhow::Result<ContainerFormat> {
        let fields = normalized_struct
            .fields
            .iter()
            .map(|f| {
                self.build_normalized_type_layout(&f.type_, type_arguments)
                    .map(|value| Named {
                        name: f.name.to_string(),
                        value,
                    })
            })
            .collect::<anyhow::Result<Vec<Named<Format>>>>()?;
        Ok(ContainerFormat::Struct(fields))
    }
}

fn print_format_type(t: &Format) -> String {
    match t {
        Format::TypeName(s) => s.to_string(),
        Format::Bool => "bool".to_string(),
        Format::U8 => "u8".to_string(),
        Format::U16 => "u16".to_string(),
        Format::U32 => "u32".to_string(),
        Format::U64 => "u64".to_string(),
        Format::U128 => "u128".to_string(),
        Format::Bytes => "vector<u8>".to_string(),
        Format::Seq(inner) => format!("vector<{}>", print_format_type(inner)),
        v => unimplemented!("Printing format value {:?}", v),
    }
}

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
