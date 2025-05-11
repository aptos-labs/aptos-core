// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    module_traversal::TraversalContext,
    storage::{
        loader::traits::StructDefinitionLoader,
        ty_layout_converter::{LayoutConverter, LayoutWithDelayedFields},
    },
};
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_vm_types::{gas::GasMeter, loaded_data::runtime_types::Type};
use std::collections::{btree_map::Entry, BTreeMap};

/// Cache for all layouts available for native context. When VM calls into a native function, these
/// layouts are the only layouts accessible. The Move implementation must ensure layouts are pre-
/// loaded into cache before the native call.
#[derive(Default)]
pub(crate) struct NativeLayoutCache {
    /// Runtime layouts that are allowed to be used by natives.
    layouts: BTreeMap<Type, LayoutWithDelayedFields>,
    /// Annotated layouts that are allowed to be used by natives (e.g., `string_utils.move`).
    annotated_layouts: BTreeMap<Type, LayoutWithDelayedFields>,
}

impl NativeLayoutCache {
    /// Returns the runtime layout from the cache. If it does not exist, an invariant violation is
    /// returned.
    pub(crate) fn type_to_type_layout_with_delayed_field_check(
        &self,
        ty: &Type,
    ) -> PartialVMResult<LayoutWithDelayedFields> {
        self.layouts.get(ty).cloned().ok_or_else(|| {
            PartialVMError::new_invariant_violation(format!("Layout for {:?} is not cached!", ty))
        })
    }

    /// Returns the annotated layout from the cache. If it does not exist, an invariant violation
    /// is returned.
    pub(crate) fn type_to_annotated_type_layout_with_delayed_field_check(
        &self,
        ty: &Type,
    ) -> PartialVMResult<LayoutWithDelayedFields> {
        self.annotated_layouts.get(ty).cloned().ok_or_else(|| {
            PartialVMError::new_invariant_violation(format!(
                "Annotated layout for {:?} is not cached!",
                ty
            ))
        })
    }

    /// Converts a vector if types to layouts, and adds them in the cache if they are not there
    /// yet.
    pub(crate) fn insert(
        &mut self,
        layout_converter: &LayoutConverter<impl StructDefinitionLoader>,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        tys: Vec<Type>,
        annotated: bool,
    ) -> PartialVMResult<()> {
        for ty in tys {
            #[allow(clippy::collapsible_else_if)]
            if annotated {
                if let Entry::Vacant(entry) = self.annotated_layouts.entry(ty) {
                    let layout = layout_converter
                        .type_to_annotated_type_layout_with_delayed_field_check(
                            gas_meter,
                            traversal_context,
                            entry.key(),
                        )?;
                    entry.insert(layout);
                }
            } else {
                if let Entry::Vacant(entry) = self.layouts.entry(ty) {
                    let layout = layout_converter.type_to_type_layout_with_delayed_field_check(
                        gas_meter,
                        traversal_context,
                        entry.key(),
                    )?;
                    entry.insert(layout);
                }
            };
        }
        Ok(())
    }
}
