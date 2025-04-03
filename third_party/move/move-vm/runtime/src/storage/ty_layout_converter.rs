// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    config::VMConfig, module_traversal::TraversalContext,
    storage::ty_tag_converter::TypeTagConverter, ModuleStorage,
};
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::{
    function::MoveFunctionLayout,
    gas_algebra::{InternalGas, NumBytes, NumModules},
    language_storage::StructTag,
    value::{IdentifierMappingKind, MoveFieldLayout, MoveStructLayout, MoveTypeLayout},
    vm_status::StatusCode,
};
use move_vm_metrics::{Timer, VM_TIMER};
use move_vm_types::{
    gas::GasMeter,
    loaded_data::{
        runtime_types::{StructLayout, StructType, Type},
        struct_name_indexing::{StructNameIndex, StructNameIndexMap},
    },
};
use std::sync::Arc;

/// Maximal nodes which are allowed when converting to layout. This includes the types of
/// fields for struct types.
const MAX_TYPE_TO_LAYOUT_NODES: u64 = 256;

/// Maximal depth of a value in terms of type depth.
const VALUE_DEPTH_MAX: u64 = 128;

/// A trait allowing to convert runtime types into other types used throughout the stack.
#[allow(private_bounds)]
pub(crate) trait LayoutConverter: LayoutConverterBase {
    /// Converts a runtime type to a type layout.
    fn type_to_type_layout(&mut self, ty: &Type) -> PartialVMResult<MoveTypeLayout> {
        let _timer = VM_TIMER.timer_with_label("Loader::type_to_type_layout");

        let mut count = 0;
        self.type_to_type_layout_impl(ty, &mut count, 1)
            .map(|(l, _)| l)
    }

    /// Converts a runtime type to a type layout.
    fn type_to_type_layout_with_identifier_mappings(
        &mut self,
        ty: &Type,
    ) -> PartialVMResult<(MoveTypeLayout, bool)> {
        let _timer = VM_TIMER.timer_with_label("Loader::type_to_type_layout");

        let mut count = 0;
        self.type_to_type_layout_impl(ty, &mut count, 1)
    }

    /// Converts a runtime type to a fully annotated type layout, containing information about
    /// field names.
    fn type_to_fully_annotated_layout(&mut self, ty: &Type) -> PartialVMResult<MoveTypeLayout> {
        let _timer = VM_TIMER.timer_with_label("Loader::type_to_type_layout");

        let mut count = 0;
        self.type_to_fully_annotated_layout_impl(ty, &mut count, 1)
    }
}

// This is not intended to be implemented or used externally, so put abstract and other functions
// into this crate trait.
pub(crate) trait LayoutConverterBase {
    fn vm_config(&self) -> &VMConfig;
    fn fetch_struct_ty_by_idx(&mut self, idx: StructNameIndex) -> PartialVMResult<Arc<StructType>>;
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
        &mut self,
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
            Type::Function {
                args,
                results,
                abilities,
            } => {
                *count += 1;
                let mut identifier_mapping = false;
                let mut to_list = |tys: &[Type]| {
                    tys.iter()
                        .map(|ety| {
                            self.type_to_type_layout_impl(ety, count, depth + 1)
                                .map(|(l, has)| {
                                    identifier_mapping |= has;
                                    l
                                })
                        })
                        .collect::<PartialVMResult<Vec<_>>>()
                };
                (
                    MoveTypeLayout::Function(MoveFunctionLayout(
                        to_list(args)?,
                        to_list(results)?,
                        *abilities,
                    )),
                    identifier_mapping,
                )
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
        &mut self,
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
        &mut self,
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
            Type::Function {
                args,
                results,
                abilities,
            } => {
                let mut to_list = |tys: &[Type]| {
                    tys.iter()
                        .map(|ety| self.type_to_fully_annotated_layout_impl(ety, count, depth + 1))
                        .collect::<PartialVMResult<Vec<_>>>()
                };
                MoveTypeLayout::Function(MoveFunctionLayout(
                    to_list(args)?,
                    to_list(results)?,
                    *abilities,
                ))
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
        &mut self,
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

pub(crate) struct UnmeteredLayoutConverter<'a> {
    storage: &'a dyn ModuleStorage,
}

impl<'a> UnmeteredLayoutConverter<'a> {
    pub fn new(storage: &'a dyn ModuleStorage) -> Self {
        Self { storage }
    }
}

impl<'a> LayoutConverterBase for UnmeteredLayoutConverter<'a> {
    fn vm_config(&self) -> &VMConfig {
        self.storage.runtime_environment().vm_config()
    }

    fn fetch_struct_ty_by_idx(&mut self, idx: StructNameIndex) -> PartialVMResult<Arc<StructType>> {
        let struct_name = self
            .storage
            .runtime_environment()
            .struct_name_index_map()
            .idx_to_struct_name_ref(idx)?;

        self.storage.unmetered_get_struct_definition(
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

impl<'a> LayoutConverter for UnmeteredLayoutConverter<'a> {}

pub(crate) struct MetredLazyLayoutConverter<'a, 'b, G, T> {
    gas_meter: &'a mut G,
    traversal_context: &'a mut TraversalContext<'b>,
    code_storage: &'a T,
}

impl<'a, 'b, G, T> MetredLazyLayoutConverter<'a, 'b, G, T>
where
    G: GasMeter,
    T: ModuleStorage,
{
    pub fn new(
        gas_meter: &'a mut G,
        traversal_context: &'a mut TraversalContext<'b>,
        code_storage: &'a T,
    ) -> Self {
        debug_assert!(
            code_storage
                .runtime_environment()
                .vm_config()
                .use_lazy_loading
        );
        Self {
            gas_meter,
            traversal_context,
            code_storage,
        }
    }
}

impl<'a, 'b, G, T> LayoutConverterBase for MetredLazyLayoutConverter<'a, 'b, G, T>
where
    G: GasMeter,
    T: ModuleStorage,
{
    fn vm_config(&self) -> &VMConfig {
        self.code_storage.runtime_environment().vm_config()
    }

    fn fetch_struct_ty_by_idx(&mut self, idx: StructNameIndex) -> PartialVMResult<Arc<StructType>> {
        let struct_name = self
            .code_storage
            .runtime_environment()
            .struct_name_index_map()
            .idx_to_struct_name_ref(idx)?;

        let module_id = self
            .traversal_context
            .referenced_module_ids
            .alloc(struct_name.module.clone());
        let addr = module_id.address();
        let name = module_id.name();

        if !addr.is_special()
            && self
                .traversal_context
                .visited
                .insert((addr, name), ())
                .is_none()
        {
            let size = self
                .code_storage
                .unmetered_get_existing_module_size(addr, name)
                .map_err(|err| err.to_partial())?;
            self.gas_meter
                .charge_dependency(false, addr, name, NumBytes::new(size as u64))?;
        }

        self.code_storage.unmetered_get_struct_definition(
            struct_name.module.address(),
            struct_name.module.name(),
            struct_name.name.as_ident_str(),
        )
    }

    fn struct_name_index_map(&self) -> &StructNameIndexMap {
        self.code_storage
            .runtime_environment()
            .struct_name_index_map()
    }

    fn struct_name_idx_to_struct_tag(
        &self,
        idx: StructNameIndex,
        ty_args: &[Type],
    ) -> PartialVMResult<StructTag> {
        let ty_tag_builder = TypeTagConverter::new(self.code_storage.runtime_environment());
        ty_tag_builder.struct_name_idx_to_struct_tag(&idx, ty_args)
    }
}

impl<'a, 'b, G, T> LayoutConverter for MetredLazyLayoutConverter<'a, 'b, G, T>
where
    G: GasMeter,
    T: ModuleStorage,
{
}

pub(crate) struct MetredLazyNativeLayoutConverter<'a, 'b, F> {
    traversal_context: &'a mut TraversalContext<'b>,
    code_storage: &'a dyn ModuleStorage,

    gas_used: InternalGas,
    gas_balance: InternalGas,
    total_dependency_size: NumBytes,
    max_total_dependency_size: NumBytes,
    num_dependencies: NumModules,
    max_num_dependencies: NumModules,

    cost_fn: F,
}

impl<'a, 'b, F> MetredLazyNativeLayoutConverter<'a, 'b, F>
where
    F: Fn(u64) -> InternalGas,
{
    pub fn new(
        traversal_context: &'a mut TraversalContext<'b>,
        code_storage: &'a dyn ModuleStorage,
        gas_used: InternalGas,
        gas_balance: InternalGas,
        total_dependency_size: NumBytes,
        max_total_dependency_size: NumBytes,
        num_dependencies: NumModules,
        max_num_dependencies: NumModules,
        cost_fn: F,
    ) -> Self {
        Self {
            traversal_context,
            code_storage,

            gas_used,
            gas_balance,
            total_dependency_size,
            max_total_dependency_size,
            num_dependencies,
            max_num_dependencies,

            cost_fn,
        }
    }
}

impl<'a, 'b, F> LayoutConverterBase for MetredLazyNativeLayoutConverter<'a, 'b, F>
where
    F: Fn(u64) -> InternalGas,
{
    fn vm_config(&self) -> &VMConfig {
        self.code_storage.runtime_environment().vm_config()
    }

    fn fetch_struct_ty_by_idx(&mut self, idx: StructNameIndex) -> PartialVMResult<Arc<StructType>> {
        let struct_name = self
            .code_storage
            .runtime_environment()
            .struct_name_index_map()
            .idx_to_struct_name_ref(idx)?;

        let module_id = self
            .traversal_context
            .referenced_module_ids
            .alloc(struct_name.module.clone());
        let addr = module_id.address();
        let name = module_id.name();

        if !addr.is_special()
            && self
                .traversal_context
                .visited
                .insert((addr, name), ())
                .is_none()
        {
            let size = self
                .code_storage
                .unmetered_get_existing_module_size(addr, name)
                .map(|v| v as u64)
                .map_err(|err| err.to_partial())?;

            // Simulate gas charging... Here we return partial VM errors, and the caller will remap
            // them to safe native results.
            let amount = (self.cost_fn)(size);
            self.gas_used += amount;
            if self.gas_used > self.gas_balance {
                return Err(PartialVMError::new(StatusCode::OUT_OF_GAS));
            }

            self.num_dependencies += 1.into();
            self.total_dependency_size += size.into();

            if self.num_dependencies > self.max_num_dependencies {
                return Err(PartialVMError::new(StatusCode::DEPENDENCY_LIMIT_REACHED));
            }
            if self.total_dependency_size > self.max_total_dependency_size {
                return Err(PartialVMError::new(StatusCode::DEPENDENCY_LIMIT_REACHED));
            }
        }

        self.code_storage.unmetered_get_struct_definition(
            struct_name.module.address(),
            struct_name.module.name(),
            struct_name.name.as_ident_str(),
        )
    }

    fn struct_name_index_map(&self) -> &StructNameIndexMap {
        self.code_storage
            .runtime_environment()
            .struct_name_index_map()
    }

    fn struct_name_idx_to_struct_tag(
        &self,
        idx: StructNameIndex,
        ty_args: &[Type],
    ) -> PartialVMResult<StructTag> {
        let ty_tag_builder = TypeTagConverter::new(self.code_storage.runtime_environment());
        ty_tag_builder.struct_name_idx_to_struct_tag(&idx, ty_args)
    }
}

impl<'a, 'b, F> LayoutConverter for MetredLazyNativeLayoutConverter<'a, 'b, F> where
    F: Fn(u64) -> InternalGas
{
}
