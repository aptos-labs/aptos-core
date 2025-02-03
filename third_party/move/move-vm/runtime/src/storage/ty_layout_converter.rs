// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    config::VMConfig,
    loader::{LegacyModuleStorageAdapter, Loader, PseudoGasContext, VALUE_DEPTH_MAX},
    storage::ty_tag_converter::TypeTagConverter,
    ModuleStorage,
};
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::{
    language_storage::StructTag,
    value::{IdentifierMappingKind, MoveFieldLayout, MoveStructLayout, MoveTypeLayout},
    vm_status::StatusCode,
};
use move_vm_metrics::{Timer, VM_TIMER};
use move_vm_types::loaded_data::{
    runtime_types::{StructLayout, StructType, Type},
    struct_name_indexing::{StructNameIndex, StructNameIndexMap},
};
use std::sync::Arc;

/// Maximal nodes which are allowed when converting to layout. This includes the types of
/// fields for struct types.
const MAX_TYPE_TO_LAYOUT_NODES: u64 = 256;

/// A trait allowing to convert runtime types into other types used throughout the stack.
#[allow(private_bounds)]
pub trait LayoutConverter: LayoutConverterBase {
    /// Converts a runtime type to a type layout.
    fn type_to_type_layout(&self, ty: &Type) -> PartialVMResult<MoveTypeLayout> {
        let _timer = VM_TIMER.timer_with_label("Loader::type_to_type_layout");

        let mut count = 0;
        self.type_to_type_layout_impl(ty, &mut count, 1)
            .map(|(l, _)| l)
    }

    /// Converts a runtime type to a type layout.
    fn type_to_type_layout_with_identifier_mappings(
        &self,
        ty: &Type,
    ) -> PartialVMResult<(MoveTypeLayout, bool)> {
        let _timer = VM_TIMER.timer_with_label("Loader::type_to_type_layout");

        let mut count = 0;
        self.type_to_type_layout_impl(ty, &mut count, 1)
    }

    /// Converts a runtime type to a fully annotated type layout, containing information about
    /// field names.
    fn type_to_fully_annotated_layout(&self, ty: &Type) -> PartialVMResult<MoveTypeLayout> {
        let _timer = VM_TIMER.timer_with_label("Loader::type_to_type_layout");

        let mut count = 0;
        self.type_to_fully_annotated_layout_impl(ty, &mut count, 1)
    }
}

// This is not intended to be implemented or used externally, so put abstract and other functions
// into this crate trait.
pub(crate) trait LayoutConverterBase {
    fn vm_config(&self) -> &VMConfig;
    fn fetch_struct_ty_by_idx(&self, idx: StructNameIndex) -> PartialVMResult<Arc<StructType>>;
    fn struct_name_index_map(&self) -> &StructNameIndexMap;

    /// Required for annotated layout.
    fn struct_name_idx_to_struct_tag(
        &self,
        idx: StructNameIndex,
        ty_args: &[Type],
    ) -> PartialVMResult<StructTag>;

    // -------------------------------------------------------------------------------------
    // Layout

    fn check_type_layout_bounds(&self, node_count: u64, depth: u64) -> PartialVMResult<()> {
        if node_count > MAX_TYPE_TO_LAYOUT_NODES {
            return Err(
                PartialVMError::new(StatusCode::TOO_MANY_TYPE_NODES).with_message(format!(
                    "Number of type nodes when constructing type layout exceeded the maximum of {}",
                    MAX_TYPE_TO_LAYOUT_NODES
                )),
            );
        }
        if depth > VALUE_DEPTH_MAX {
            return Err(
                PartialVMError::new(StatusCode::VM_MAX_VALUE_DEPTH_REACHED).with_message(format!(
                    "Depth of a layout exceeded the maximum of {} during construction",
                    VALUE_DEPTH_MAX
                )),
            );
        }
        Ok(())
    }

    fn type_to_type_layout_impl(
        &self,
        ty: &Type,
        count: &mut u64,
        depth: u64,
    ) -> PartialVMResult<(MoveTypeLayout, bool)> {
        self.check_type_layout_bounds(*count, depth)?;
        Ok(match ty {
            Type::Bool => {
                *count += 1;
                (MoveTypeLayout::Bool, false)
            },
            Type::U8 => {
                *count += 1;
                (MoveTypeLayout::U8, false)
            },
            Type::U16 => {
                *count += 1;
                (MoveTypeLayout::U16, false)
            },
            Type::U32 => {
                *count += 1;
                (MoveTypeLayout::U32, false)
            },
            Type::U64 => {
                *count += 1;
                (MoveTypeLayout::U64, false)
            },
            Type::U128 => {
                *count += 1;
                (MoveTypeLayout::U128, false)
            },
            Type::U256 => {
                *count += 1;
                (MoveTypeLayout::U256, false)
            },
            Type::Address => {
                *count += 1;
                (MoveTypeLayout::Address, false)
            },
            Type::Signer => {
                *count += 1;
                (MoveTypeLayout::Signer, false)
            },
            Type::Vector(ty) => {
                *count += 1;
                let (layout, has_identifier_mappings) =
                    self.type_to_type_layout_impl(ty, count, depth + 1)?;
                (
                    MoveTypeLayout::Vector(Box::new(layout)),
                    has_identifier_mappings,
                )
            },
            Type::Struct { idx, .. } => {
                *count += 1;
                let (layout, has_identifier_mappings) =
                    self.struct_name_to_type_layout(*idx, &[], count, depth + 1)?;
                (layout, has_identifier_mappings)
            },
            Type::StructInstantiation { idx, ty_args, .. } => {
                *count += 1;
                self.struct_name_to_type_layout(*idx, ty_args, count, depth + 1)?
            },
            Type::Reference(_) | Type::MutableReference(_) | Type::TyParam(_) => {
                return Err(
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .with_message(format!("No type layout for {:?}", ty)),
                );
            },
        })
    }

    fn struct_name_to_type_layout(
        &self,
        struct_name_idx: StructNameIndex,
        ty_args: &[Type],
        count: &mut u64,
        depth: u64,
    ) -> PartialVMResult<(MoveTypeLayout, bool)> {
        let struct_type = self.fetch_struct_ty_by_idx(struct_name_idx)?;

        let mut has_identifier_mappings = false;

        let layout = match &struct_type.layout {
            StructLayout::Single(fields) => {
                // Some types can have fields which are lifted at serialization or deserialization
                // times. Right now these are Aggregator and AggregatorSnapshot.
                let maybe_mapping = self.get_identifier_mapping_kind(struct_name_idx)?;
                let field_tys = fields
                    .iter()
                    .map(|(_, ty)| {
                        self.vm_config()
                            .ty_builder
                            .create_ty_with_subst(ty, ty_args)
                    })
                    .collect::<PartialVMResult<Vec<_>>>()?;
                let (mut field_layouts, field_has_identifier_mappings): (
                    Vec<MoveTypeLayout>,
                    Vec<bool>,
                ) = field_tys
                    .iter()
                    .map(|ty| self.type_to_type_layout_impl(ty, count, depth))
                    .collect::<PartialVMResult<Vec<_>>>()?
                    .into_iter()
                    .unzip();

                has_identifier_mappings =
                    maybe_mapping.is_some() || field_has_identifier_mappings.into_iter().any(|b| b);

                let layout = if Some(IdentifierMappingKind::DerivedString) == maybe_mapping {
                    // For DerivedString, the whole object should be lifted.
                    MoveTypeLayout::Native(
                        IdentifierMappingKind::DerivedString,
                        Box::new(MoveTypeLayout::Struct(MoveStructLayout::new(field_layouts))),
                    )
                } else {
                    // For aggregators / snapshots, the first field should be lifted.
                    if let Some(kind) = &maybe_mapping {
                        if let Some(l) = field_layouts.first_mut() {
                            *l = MoveTypeLayout::Native(kind.clone(), Box::new(l.clone()));
                        }
                    }
                    MoveTypeLayout::Struct(MoveStructLayout::new(field_layouts))
                };
                layout
            },
            StructLayout::Variants(variants) => {
                // We do not support variants to have direct identifier mappings,
                // but their inner types may.
                let variant_layouts = variants
                    .iter()
                    .map(|variant| {
                        variant
                            .1
                            .iter()
                            .map(|(_, ty)| {
                                let ty = self
                                    .vm_config()
                                    .ty_builder
                                    .create_ty_with_subst(ty, ty_args)?;
                                let (ty, has_id_mappings) =
                                    self.type_to_type_layout_impl(&ty, count, depth)?;
                                has_identifier_mappings |= has_id_mappings;
                                Ok(ty)
                            })
                            .collect::<PartialVMResult<Vec<_>>>()
                    })
                    .collect::<PartialVMResult<Vec<_>>>()?;
                MoveTypeLayout::Struct(MoveStructLayout::RuntimeVariants(variant_layouts))
            },
        };

        Ok((layout, has_identifier_mappings))
    }

    fn get_identifier_mapping_kind(
        &self,
        idx: StructNameIndex,
    ) -> PartialVMResult<Option<IdentifierMappingKind>> {
        if !self.vm_config().delayed_field_optimization_enabled {
            return Ok(None);
        }
        let struct_name = self.struct_name_index_map().idx_to_struct_name(idx)?;
        Ok(IdentifierMappingKind::from_ident(
            &struct_name.module,
            &struct_name.name,
        ))
    }

    // -------------------------------------------------------------------------------------
    // Decorated Layout

    fn type_to_fully_annotated_layout_impl(
        &self,
        ty: &Type,
        count: &mut u64,
        depth: u64,
    ) -> PartialVMResult<MoveTypeLayout> {
        if *count > MAX_TYPE_TO_LAYOUT_NODES {
            return Err(
                PartialVMError::new(StatusCode::TOO_MANY_TYPE_NODES).with_message(format!(
                    "Number of type nodes when constructing type layout exceeded the maximum of {}",
                    MAX_TYPE_TO_LAYOUT_NODES
                )),
            );
        }
        if depth > VALUE_DEPTH_MAX {
            return Err(
                PartialVMError::new(StatusCode::VM_MAX_VALUE_DEPTH_REACHED).with_message(format!(
                    "Depth of a layout exceeded the maximum of {} during construction",
                    VALUE_DEPTH_MAX
                )),
            );
        }
        Ok(match ty {
            Type::Bool => MoveTypeLayout::Bool,
            Type::U8 => MoveTypeLayout::U8,
            Type::U16 => MoveTypeLayout::U16,
            Type::U32 => MoveTypeLayout::U32,
            Type::U64 => MoveTypeLayout::U64,
            Type::U128 => MoveTypeLayout::U128,
            Type::U256 => MoveTypeLayout::U256,
            Type::Address => MoveTypeLayout::Address,
            Type::Signer => MoveTypeLayout::Signer,
            Type::Vector(ty) => MoveTypeLayout::Vector(Box::new(
                self.type_to_fully_annotated_layout_impl(ty, count, depth + 1)?,
            )),
            Type::Struct { idx, .. } => {
                self.struct_name_to_fully_annotated_layout(*idx, &[], count, depth + 1)?
            },
            Type::StructInstantiation { idx, ty_args, .. } => {
                self.struct_name_to_fully_annotated_layout(*idx, ty_args, count, depth + 1)?
            },
            Type::Reference(_) | Type::MutableReference(_) | Type::TyParam(_) => {
                return Err(
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .with_message(format!("No type layout for {:?}", ty)),
                );
            },
        })
    }

    fn struct_name_to_fully_annotated_layout(
        &self,
        struct_name_idx: StructNameIndex,
        ty_args: &[Type],
        count: &mut u64,
        depth: u64,
    ) -> PartialVMResult<MoveTypeLayout> {
        let struct_type = self.fetch_struct_ty_by_idx(struct_name_idx)?;

        // TODO(#13806): have annotated layouts for variants. Currently, we just return the raw
        //   layout for them.
        if matches!(struct_type.layout, StructLayout::Variants(_)) {
            return self
                .struct_name_to_type_layout(struct_name_idx, ty_args, count, depth)
                .map(|(l, _)| l);
        }

        let struct_tag = self.struct_name_idx_to_struct_tag(struct_name_idx, ty_args)?;
        let fields = struct_type.fields(None)?;

        let field_layouts = fields
            .iter()
            .map(|(n, ty)| {
                let ty = self
                    .vm_config()
                    .ty_builder
                    .create_ty_with_subst(ty, ty_args)?;
                let l = self.type_to_fully_annotated_layout_impl(&ty, count, depth)?;
                Ok(MoveFieldLayout::new(n.clone(), l))
            })
            .collect::<PartialVMResult<Vec<_>>>()?;
        let struct_layout =
            MoveTypeLayout::Struct(MoveStructLayout::with_types(struct_tag, field_layouts));

        Ok(struct_layout)
    }
}

// --------------------------------------------------------------------------------------------
// Layout converter based on ModuleStorage

pub struct StorageLayoutConverter<'a> {
    storage: &'a dyn ModuleStorage,
}

impl<'a> StorageLayoutConverter<'a> {
    pub fn new(storage: &'a dyn ModuleStorage) -> Self {
        Self { storage }
    }
}

impl<'a> LayoutConverterBase for StorageLayoutConverter<'a> {
    fn vm_config(&self) -> &VMConfig {
        self.storage.runtime_environment().vm_config()
    }

    fn fetch_struct_ty_by_idx(&self, idx: StructNameIndex) -> PartialVMResult<Arc<StructType>> {
        let struct_name = self.struct_name_index_map().idx_to_struct_name_ref(idx)?;
        self.storage.fetch_struct_ty(
            struct_name.module.address(),
            struct_name.module.name(),
            struct_name.name.as_ident_str(),
        )
    }

    fn struct_name_index_map(&self) -> &StructNameIndexMap {
        self.storage.runtime_environment().struct_name_index_map()
    }

    fn struct_name_idx_to_struct_tag(
        &self,
        idx: StructNameIndex,
        ty_args: &[Type],
    ) -> PartialVMResult<StructTag> {
        let ty_tag_builder = TypeTagConverter::new(self.storage.runtime_environment());
        ty_tag_builder.struct_name_idx_to_struct_tag(&idx, ty_args)
    }
}

impl<'a> LayoutConverter for StorageLayoutConverter<'a> {}

// --------------------------------------------------------------------------------------------
// Layout converter based on `Loader`

// This should go away once we eliminated loader v1.

pub(crate) struct LoaderLayoutConverter<'a> {
    loader: &'a Loader,
    module_store: &'a LegacyModuleStorageAdapter,
    module_storage: &'a dyn ModuleStorage,
}

impl<'a> LoaderLayoutConverter<'a> {
    pub fn new(
        loader: &'a Loader,
        module_store: &'a LegacyModuleStorageAdapter,
        module_storage: &'a dyn ModuleStorage,
    ) -> Self {
        Self {
            loader,
            module_store,
            module_storage,
        }
    }
}

impl<'a> LayoutConverterBase for LoaderLayoutConverter<'a> {
    fn vm_config(&self) -> &VMConfig {
        self.loader.vm_config()
    }

    fn fetch_struct_ty_by_idx(&self, idx: StructNameIndex) -> PartialVMResult<Arc<StructType>> {
        match self.loader {
            Loader::V1(..) => {
                self.loader
                    .fetch_struct_ty_by_idx(idx, self.module_store, self.module_storage)
            },
            Loader::V2(..) => {
                let struct_name = self.struct_name_index_map().idx_to_struct_name_ref(idx)?;
                self.module_storage.fetch_struct_ty(
                    struct_name.module.address(),
                    struct_name.module.name(),
                    struct_name.name.as_ident_str(),
                )
            },
        }
    }

    fn struct_name_index_map(&self) -> &StructNameIndexMap {
        self.loader.struct_name_index_map(self.module_storage)
    }

    fn struct_name_idx_to_struct_tag(
        &self,
        idx: StructNameIndex,
        ty_args: &[Type],
    ) -> PartialVMResult<StructTag> {
        match self.loader {
            Loader::V1(loader) => {
                let mut gas_context = PseudoGasContext::new(self.vm_config());
                let arg_tags = ty_args
                    .iter()
                    .map(|t| loader.type_to_type_tag_impl(t, &mut gas_context))
                    .collect::<PartialVMResult<Vec<_>>>()?;
                loader.name_cache.idx_to_struct_tag(idx, arg_tags)
            },
            Loader::V2(..) => TypeTagConverter::new(self.module_storage.runtime_environment())
                .struct_name_idx_to_struct_tag(&idx, ty_args),
        }
    }
}

impl<'a> LayoutConverter for LoaderLayoutConverter<'a> {}
