// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::ModuleStorage;
use move_binary_format::{
    errors::{PartialVMError, PartialVMResult},
    file_format::TypeParameterIndex,
};
use move_core_types::vm_status::StatusCode;
use move_vm_types::loaded_data::{
    runtime_types::{DepthFormula, StructLayout, Type},
    struct_name_indexing::StructNameIndex,
};
use std::collections::{BTreeMap, HashMap};

/// Calculates [DepthFormula] for struct types. Stores a cache of visited formulas.
pub(crate) struct DepthFormulaCalculator<'a, M> {
    module_storage: &'a M,
    visited_formulas: HashMap<StructNameIndex, DepthFormula>,
}

impl<'a, M> DepthFormulaCalculator<'a, M>
where
    M: ModuleStorage,
{
    pub(crate) fn new(module_storage: &'a M) -> Self {
        Self {
            module_storage,
            visited_formulas: HashMap::new(),
        }
    }

    pub(crate) fn calculate_depth_of_struct(
        &mut self,
        struct_name_idx: &StructNameIndex,
    ) -> PartialVMResult<DepthFormula> {
        if let Some(depth_formula) = self.visited_formulas.get(struct_name_idx) {
            return Ok(depth_formula.clone());
        }

        let struct_type = self
            .module_storage
            .fetch_struct_ty_by_idx(struct_name_idx)?;
        let formulas = match &struct_type.layout {
            StructLayout::Single(fields) => fields
                .iter()
                .map(|(_, field_ty)| self.calculate_depth_of_type(field_ty))
                .collect::<PartialVMResult<Vec<_>>>()?,
            StructLayout::Variants(variants) => variants
                .iter()
                .flat_map(|variant| variant.1.iter().map(|(_, ty)| ty))
                .map(|field_ty| self.calculate_depth_of_type(field_ty))
                .collect::<PartialVMResult<Vec<_>>>()?,
        };

        let formula = DepthFormula::normalize(formulas);
        if self
            .visited_formulas
            .insert(*struct_name_idx, formula.clone())
            .is_some()
        {
            // Same thread has put this entry previously, which means there is a recursion.
            let struct_name = self
                .module_storage
                .runtime_environment()
                .struct_name_index_map()
                .idx_to_struct_name_ref(*struct_name_idx)?;
            return Err(
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR).with_message(
                    format!(
                        "Depth formula for struct '{}' is already cached by the same thread",
                        struct_name.as_ref(),
                    ),
                ),
            );
        }
        Ok(formula)
    }

    fn calculate_depth_of_type(&mut self, ty: &Type) -> PartialVMResult<DepthFormula> {
        Ok(match ty {
            Type::Bool
            | Type::U8
            | Type::U64
            | Type::U128
            | Type::Address
            | Type::Signer
            | Type::U16
            | Type::U32
            | Type::U256 => DepthFormula::constant(1),
            Type::Vector(ty) => {
                let mut inner = self.calculate_depth_of_type(ty)?;
                inner.scale(1);
                inner
            },
            Type::Reference(ty) | Type::MutableReference(ty) => {
                let mut inner = self.calculate_depth_of_type(ty)?;
                inner.scale(1);
                inner
            },
            Type::TyParam(ty_idx) => DepthFormula::type_parameter(*ty_idx),
            Type::Struct { idx, .. } => {
                let mut struct_formula = self.calculate_depth_of_struct(idx)?;
                debug_assert!(struct_formula.terms.is_empty());
                struct_formula.scale(1);
                struct_formula
            },
            Type::StructInstantiation { idx, ty_args, .. } => {
                let ty_arg_map = ty_args
                    .iter()
                    .enumerate()
                    .map(|(idx, ty)| {
                        let var = idx as TypeParameterIndex;
                        Ok((var, self.calculate_depth_of_type(ty)?))
                    })
                    .collect::<PartialVMResult<BTreeMap<_, _>>>()?;
                let struct_formula = self.calculate_depth_of_struct(idx)?;
                let mut subst_struct_formula = struct_formula.subst(ty_arg_map)?;
                subst_struct_formula.scale(1);
                subst_struct_formula
            },
            Type::Function {
                args,
                results,
                abilities: _,
            } => {
                let mut inner = DepthFormula::normalize(
                    args.iter()
                        .chain(results)
                        .map(|arg_ty| self.calculate_depth_of_type(arg_ty))
                        .collect::<PartialVMResult<Vec<_>>>()?,
                );
                inner.scale(1);
                inner
            },
        })
    }
}
