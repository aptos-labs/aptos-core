// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    config::VMConfig,
    module_traversal::TraversalContext,
    storage::{loader::traits::StructDefinitionLoader, ty_tag_converter::TypeTagConverter},
    RuntimeEnvironment,
};
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::{
    identifier::Identifier,
    value::{IdentifierMappingKind, MoveFieldLayout, MoveStructLayout, MoveTypeLayout},
    vm_status::StatusCode,
};
use move_vm_metrics::{Timer, VM_TIMER};
use move_vm_types::{
    gas::DependencyGasMeter,
    loaded_data::{
        runtime_types::{StructIdentifier, StructLayout, Type},
        struct_name_indexing::StructNameIndex,
    },
};
use std::sync::Arc;

/// Stores type layout as well as a flag if it contains any delayed fields.
#[derive(Debug)]
pub struct LayoutWithDelayedFields {
    layout: MoveTypeLayout,
    contains_delayed_fields: bool,
}

impl LayoutWithDelayedFields {
    /// If layout contains delayed fields, returns [None]. If there are no delayed fields, the
    /// layout is returned.
    pub fn into_layout_when_has_no_delayed_fields(self) -> Option<MoveTypeLayout> {
        (!self.contains_delayed_fields).then_some(self.layout)
    }

    /// If layout does not contain delayed fields, returns [None]. If there are delayed fields, the
    /// layout is returned.
    pub fn layout_when_contains_delayed_fields(&self) -> Option<&MoveTypeLayout> {
        self.contains_delayed_fields.then_some(&self.layout)
    }

    /// Unpacks and returns the layout and delayed fields flag for the caller to handle.
    pub fn unpack(self) -> (MoveTypeLayout, bool) {
        (self.layout, self.contains_delayed_fields)
    }
}

/// Converts runtime types to type layouts. The layout construction may load modules, and so, the
/// functions may also charge gas.
pub(crate) struct LayoutConverter<'a, T> {
    struct_definition_loader: &'a T,
}

impl<'a, T> LayoutConverter<'a, T>
where
    T: StructDefinitionLoader,
{
    /// Creates a new layout converter with access to struct definition loader.
    pub(crate) fn new(struct_definition_loader: &'a T) -> Self {
        Self {
            struct_definition_loader,
        }
    }

    /// Returns true if lazy loading is enabled.
    pub(crate) fn is_lazy_loading_enabled(&self) -> bool {
        self.struct_definition_loader.is_lazy_loading_enabled()
    }

    /// Returns the layout of a type, as well as a flag if it contains delayed fields or not.
    pub(crate) fn type_to_type_layout_with_delayed_fields(
        &self,
        gas_meter: &mut impl DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        ty: &Type,
    ) -> PartialVMResult<LayoutWithDelayedFields> {
        self.type_to_type_layout_with_delayed_fields_impl::<false>(gas_meter, traversal_context, ty)
    }

    /// Returns the decorated layout of a type.
    ///
    /// Used only for string formatting natives, so avoid using as much as possible!
    pub(crate) fn type_to_annotated_type_layout_with_delayed_fields(
        &self,
        gas_meter: &mut impl DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        ty: &Type,
    ) -> PartialVMResult<LayoutWithDelayedFields> {
        self.type_to_type_layout_with_delayed_fields_impl::<true>(gas_meter, traversal_context, ty)
    }

    /// Returns the VM config used in the system.
    fn vm_config(&self) -> &VMConfig {
        self.runtime_environment().vm_config()
    }

    /// Returns the runtime environment used in the system.
    pub(crate) fn runtime_environment(&self) -> &RuntimeEnvironment {
        self.struct_definition_loader.runtime_environment()
    }

    /// Returns the struct name for the specified index.
    fn get_struct_name(&self, idx: &StructNameIndex) -> PartialVMResult<Arc<StructIdentifier>> {
        self.struct_definition_loader
            .runtime_environment()
            .struct_name_index_map()
            .idx_to_struct_name_ref(*idx)
    }

    /// If delayed field optimization is not enabled, returns [None]. Otherwise, if the struct is a
    /// delayed field, returns its kind (e.g., 0x1::aggregator_v2::Aggregator).
    fn get_delayed_field_kind_if_delayed_field_optimization_enabled(
        &self,
        idx: &StructNameIndex,
    ) -> PartialVMResult<Option<IdentifierMappingKind>> {
        if !self.vm_config().delayed_field_optimization_enabled {
            return Ok(None);
        }
        let struct_name = self.get_struct_name(idx)?;
        Ok(IdentifierMappingKind::from_ident(
            &struct_name.module,
            &struct_name.name,
        ))
    }

    /// Since layout is a tree data structure, we limit its size and depth during construction.
    /// This function checks that the number of nodes in the layout and its depth are within limits
    /// enforced by the VM config.
    fn check_type_layout_bounds(&self, node_count: u64, depth: u64) -> PartialVMResult<()> {
        if node_count > self.vm_config().layout_max_size {
            return Err(
                PartialVMError::new(StatusCode::TOO_MANY_TYPE_NODES).with_message(format!(
                    "Number of type nodes when constructing type layout exceeded the maximum of {}",
                    self.vm_config().layout_max_size
                )),
            );
        }
        if depth > self.vm_config().layout_max_depth {
            return Err(
                PartialVMError::new(StatusCode::VM_MAX_VALUE_DEPTH_REACHED).with_message(format!(
                    "Depth of a layout exceeded the maximum of {} during construction",
                    self.vm_config().layout_max_depth
                )),
            );
        }
        Ok(())
    }

    /// Entrypoint into layout construction. Additionally, measures the time taken to construct it.
    fn type_to_type_layout_with_delayed_fields_impl<const ANNOTATED: bool>(
        &self,
        gas_meter: &mut impl DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        ty: &Type,
    ) -> PartialVMResult<LayoutWithDelayedFields> {
        let _timer = VM_TIMER.timer_with_label("type_to_type_layout_with_delayed_fields");

        let mut count = 0;
        let (layout, contains_delayed_fields) = self.type_to_type_layout_impl::<ANNOTATED>(
            gas_meter,
            traversal_context,
            ty,
            &mut count,
            1,
        )?;
        Ok(LayoutWithDelayedFields {
            layout,
            contains_delayed_fields,
        })
    }

    /// Converts [Type] to [MoveTypeLayout]. Fails if there is not enough gas to account for module
    /// loading, the layout is too deep / too large, or some other miscellaneous failures, e.g. if
    /// struct definition is not found.
    fn type_to_type_layout_impl<const ANNOTATED: bool>(
        &self,
        gas_meter: &mut impl DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        ty: &Type,
        count: &mut u64,
        depth: u64,
    ) -> PartialVMResult<(MoveTypeLayout, bool)> {
        self.check_type_layout_bounds(*count, depth)?;

        *count += 1;
        Ok(match ty {
            Type::Bool => (MoveTypeLayout::Bool, false),
            Type::U8 => (MoveTypeLayout::U8, false),
            Type::U16 => (MoveTypeLayout::U16, false),
            Type::U32 => (MoveTypeLayout::U32, false),
            Type::U64 => (MoveTypeLayout::U64, false),
            Type::U128 => (MoveTypeLayout::U128, false),
            Type::U256 => (MoveTypeLayout::U256, false),
            Type::Address => (MoveTypeLayout::Address, false),
            Type::Signer => (MoveTypeLayout::Signer, false),
            Type::Function { .. } => (MoveTypeLayout::Function, false),
            Type::Vector(ty) => self
                .type_to_type_layout_impl::<ANNOTATED>(
                    gas_meter,
                    traversal_context,
                    ty,
                    count,
                    depth + 1,
                )
                .map(|(elem_layout, contains_delayed_fields)| {
                    let vec_layout = MoveTypeLayout::Vector(Box::new(elem_layout));
                    (vec_layout, contains_delayed_fields)
                })?,
            Type::Struct { idx, .. } => self.struct_to_type_layout::<ANNOTATED>(
                gas_meter,
                traversal_context,
                idx,
                &[],
                count,
                depth + 1,
            )?,
            Type::StructInstantiation { idx, ty_args, .. } => self
                .struct_to_type_layout::<ANNOTATED>(
                    gas_meter,
                    traversal_context,
                    idx,
                    ty_args,
                    count,
                    depth + 1,
                )?,
            Type::Reference(_) | Type::MutableReference(_) | Type::TyParam(_) => {
                return Err(
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .with_message(format!("No type layout for {:?}", ty)),
                );
            },
        })
    }

    /// Converts a sequence of types into a sequence of layouts.
    fn types_to_type_layouts<const ANNOTATED: bool>(
        &self,
        gas_meter: &mut impl DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        tys: &[Type],
        count: &mut u64,
        depth: u64,
    ) -> PartialVMResult<(Vec<MoveTypeLayout>, bool)> {
        let mut contains_delayed_fields = false;
        let layouts = tys
            .iter()
            .map(|ty| {
                let (layout, ty_contains_delayed_fields) = self
                    .type_to_type_layout_impl::<ANNOTATED>(
                        gas_meter,
                        traversal_context,
                        ty,
                        count,
                        depth,
                    )?;
                contains_delayed_fields |= ty_contains_delayed_fields;
                Ok(layout)
            })
            .collect::<PartialVMResult<Vec<_>>>()?;
        Ok((layouts, contains_delayed_fields))
    }

    /// Converts a struct type into the corresponding layout. If a decorated layout is required
    /// (annotated flag is set), field names are added to the layout. For delayed fields, marks
    /// layout as containing delayed fields.
    // TODO(lazy-loading):
    //   We should add struct cyclic checks here as well, but this has to be done carefully because
    //     struct A<T> {
    //       x: B<B<u8>>,
    //     }
    //  is ok, even though x's type uses cycles between Bs.
    fn struct_to_type_layout<const ANNOTATED: bool>(
        &self,
        gas_meter: &mut impl DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        idx: &StructNameIndex,
        ty_args: &[Type],
        count: &mut u64,
        depth: u64,
    ) -> PartialVMResult<(MoveTypeLayout, bool)> {
        let struct_definition = self.struct_definition_loader.load_struct_definition(
            gas_meter,
            traversal_context,
            idx,
        )?;

        let result = match &struct_definition.layout {
            // For enums, construct layouts for all possible variants. No special handling for
            // delayed fields is needed because enums cannot be delayed fields!
            StructLayout::Variants(variants) => {
                let mut variant_contains_delayed_fields = false;
                let variant_layouts = variants
                    .iter()
                    .map(|variant| {
                        // TODO(#13806):
                        //   Have annotated layouts for variant fields. Currently, we use runtime
                        //   layout here.
                        let (variant_field_layouts, variant_fields_contain_delayed_fields) =
                            self.types_to_type_layouts::<false>(
                                gas_meter,
                                traversal_context,
                                &self.apply_subst_for_field_tys(&variant.1, ty_args)?,
                                count,
                                depth,
                            )?;
                        variant_contains_delayed_fields |= variant_fields_contain_delayed_fields;
                        Ok(variant_field_layouts)
                    })
                    .collect::<PartialVMResult<Vec<_>>>()?;

                // TODO(#13806):
                //   Have annotated layouts for variants. Currently, we just return the raw layout
                //   for them.
                let variant_layout =
                    MoveTypeLayout::Struct(MoveStructLayout::RuntimeVariants(variant_layouts));
                (variant_layout, variant_contains_delayed_fields)
            },
            // For structs, we additionally check if struct is a delayed field, and if so, mark the
            // layout accordingly.
            StructLayout::Single(fields) => {
                let kind =
                    self.get_delayed_field_kind_if_delayed_field_optimization_enabled(idx)?;
                let (mut field_layouts, fields_contain_delayed_fields) = self
                    .types_to_type_layouts::<ANNOTATED>(
                        gas_meter,
                        traversal_context,
                        &self.apply_subst_for_field_tys(fields, ty_args)?,
                        count,
                        depth,
                    )?;

                match (kind, fields_contain_delayed_fields) {
                    (None, fields_contain_delayed_fields) => {
                        let struct_layout = MoveTypeLayout::Struct(
                            if ANNOTATED {
                                let ty_tag_converter = TypeTagConverter::new(
                                    self.struct_definition_loader.runtime_environment(),
                                );
                                let struct_tag =
                                    ty_tag_converter.struct_name_idx_to_struct_tag(idx, ty_args)?;
                                let field_layouts = fields
                                    .iter()
                                    .map(|(name, _)| name.clone())
                                    .zip(field_layouts)
                                    .map(|(name, layout)| MoveFieldLayout::new(name, layout))
                                    .collect();
                                MoveStructLayout::with_types(struct_tag, field_layouts)
                            } else {
                                MoveStructLayout::new(field_layouts)
                            },
                        );
                        (struct_layout, fields_contain_delayed_fields)
                    },
                    (Some(_), true) => {
                        let struct_name = self.get_struct_name(idx)?;
                        let msg = format!(
                            "Struct {}::{}::{} contains delayed fields, but is also a delayed field",
                            struct_name.module.address,
                            struct_name.module.name,
                            struct_name.name,
                        );
                        return Err(PartialVMError::new_invariant_violation(msg));
                    },
                    (Some(kind), false) => {
                        // Note: for delayed fields, simply never output annotated layout. The
                        // callers should not be able to handle it in any case.

                        use IdentifierMappingKind::*;
                        let layout = match &kind {
                            // For derived strings, replace the whole struct.
                            DerivedString => {
                                let inner_layout =
                                    MoveTypeLayout::Struct(MoveStructLayout::new(field_layouts));
                                MoveTypeLayout::Native(kind, Box::new(inner_layout))
                            },
                            // For aggregators and snapshots, we replace the layout of its first
                            // field only.
                            Aggregator | Snapshot => match field_layouts.first_mut() {
                                Some(field_layout) => {
                                    *field_layout = MoveTypeLayout::Native(
                                        kind,
                                        Box::new(field_layout.clone()),
                                    );
                                    MoveTypeLayout::Struct(MoveStructLayout::new(field_layouts))
                                },
                                None => {
                                    let struct_name = self.get_struct_name(idx)?;
                                    let msg = format!(
                                        "Struct {}::{}::{} must contain at least one field",
                                        struct_name.module.address,
                                        struct_name.module.name,
                                        struct_name.name,
                                    );
                                    return Err(PartialVMError::new_invariant_violation(msg));
                                },
                            },
                        };
                        (layout, true)
                    },
                }
            },
        };

        Ok(result)
    }

    /// Apples type substitution to struct or variant fields.
    fn apply_subst_for_field_tys(
        &self,
        field_tys: &[(Identifier, Type)],
        ty_args: &[Type],
    ) -> PartialVMResult<Vec<Type>> {
        let ty_builder = &self.vm_config().ty_builder;
        field_tys
            .iter()
            .map(|(_, ty)| ty_builder.create_ty_with_subst(ty, ty_args))
            .collect::<PartialVMResult<Vec<_>>>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        module_traversal::{TraversalContext, TraversalStorage},
        storage::loader::test_utils::MockStructDefinitionLoader,
    };
    use claims::assert_ok;
    use move_core_types::ability::AbilitySet;
    use move_vm_types::gas::UnmeteredGasMeter;
    use test_case::test_case;

    #[test_case(true)]
    #[test_case(false)]
    fn test_layout_with_delayed_fields(contains_delayed_fields: bool) {
        let layout = LayoutWithDelayedFields {
            // Dummy layout.
            layout: MoveTypeLayout::U8,
            contains_delayed_fields,
        };
        assert_eq!(
            layout.layout_when_contains_delayed_fields().is_some(),
            contains_delayed_fields
        );
        assert_eq!(
            layout.into_layout_when_has_no_delayed_fields().is_some(),
            !contains_delayed_fields
        );
    }

    #[test_case(true)]
    #[test_case(false)]
    fn test_simple_tys_to_layouts(annotated: bool) {
        let mut gas_meter = UnmeteredGasMeter;
        let traversal_storage = TraversalStorage::new();
        let mut traversal_context = TraversalContext::new(&traversal_storage);

        let loader = MockStructDefinitionLoader::default();
        let layout_converter = LayoutConverter::new(&loader);

        let test_cases = [
            (Type::Bool, MoveTypeLayout::Bool),
            (Type::U8, MoveTypeLayout::U8),
            (Type::U16, MoveTypeLayout::U16),
            (Type::U32, MoveTypeLayout::U32),
            (Type::U64, MoveTypeLayout::U64),
            (Type::U128, MoveTypeLayout::U128),
            (Type::U256, MoveTypeLayout::U256),
            (Type::Address, MoveTypeLayout::Address),
            (Type::Signer, MoveTypeLayout::Signer),
            (
                Type::Vector(triomphe::Arc::new(Type::U8)),
                MoveTypeLayout::Vector(Box::new(MoveTypeLayout::U8)),
            ),
            (
                Type::Function {
                    args: vec![Type::U8],
                    results: vec![Type::U256],
                    abilities: AbilitySet::EMPTY,
                },
                MoveTypeLayout::Function,
            ),
        ];

        for (ty, expected_layout) in test_cases {
            let result = if annotated {
                layout_converter.type_to_annotated_type_layout_with_delayed_fields(
                    &mut gas_meter,
                    &mut traversal_context,
                    &ty,
                )
            } else {
                layout_converter.type_to_type_layout_with_delayed_fields(
                    &mut gas_meter,
                    &mut traversal_context,
                    &ty,
                )
            };
            let layout = assert_ok!(result);

            assert!(!layout.contains_delayed_fields);
            assert_eq!(&layout.layout, &expected_layout);
        }
    }

    // TODO(lazy-loading): more unit tests!!!
}
