// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::loader::Resolver;
use crate::LoadedFunction;
use move_binary_format::{
    errors::*,
    file_format::{
        FieldInstantiationIndex, FunctionHandleIndex, FunctionInstantiationIndex, SignatureIndex,
        StructDefInstantiationIndex, StructVariantInstantiationIndex,
        VariantFieldInstantiationIndex,
    },
};
use move_core_types::gas_algebra::NumTypeNodes;
use move_vm_types::loaded_data::runtime_types::Type;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

#[derive(Clone)]
pub(crate) enum PerInstructionCache {
    Nothing,
    PackGeneric,
    Pack(u16),
}

#[derive(Default)]
pub(crate) struct FrameTypeCache {
    struct_field_type_instantiation:
        BTreeMap<StructDefInstantiationIndex, Vec<(Type, NumTypeNodes)>>,
    struct_variant_field_type_instantiation:
        BTreeMap<StructVariantInstantiationIndex, Vec<(Type, NumTypeNodes)>>,
    struct_def_instantiation_type: BTreeMap<StructDefInstantiationIndex, (Type, NumTypeNodes)>,
    struct_variant_instantiation_type:
        BTreeMap<StructVariantInstantiationIndex, (Type, NumTypeNodes)>,
    /// For a given field instantiation, the:
    ///    ((Type of the field, size of the field type) and (Type of its defining struct,
    ///    size of its defining struct)
    field_instantiation:
        BTreeMap<FieldInstantiationIndex, ((Type, NumTypeNodes), (Type, NumTypeNodes))>,
    /// Same as above, bot for variant field instantiations
    variant_field_instantiation:
        BTreeMap<VariantFieldInstantiationIndex, ((Type, NumTypeNodes), (Type, NumTypeNodes))>,
    single_sig_token_type: BTreeMap<SignatureIndex, (Type, NumTypeNodes)>,
    pub(crate) generic_sub_frame_cache:
        BTreeMap<FunctionInstantiationIndex, (Rc<LoadedFunction>, Rc<RefCell<FrameTypeCache>>)>,
    pub(crate) sub_frame_cache:
        BTreeMap<FunctionHandleIndex, (Rc<LoadedFunction>, Rc<RefCell<FrameTypeCache>>)>,
    pub(crate) per_instruction_cache: Vec<PerInstructionCache>,
}

impl FrameTypeCache {
    #[inline(always)]
    fn get_or<K: Copy + Ord + Eq, V, F>(
        map: &mut BTreeMap<K, V>,
        idx: K,
        ty_func: F,
    ) -> PartialVMResult<&V>
    where
        F: FnOnce(K) -> PartialVMResult<V>,
    {
        match map.entry(idx) {
            std::collections::btree_map::Entry::Occupied(entry) => Ok(entry.into_mut()),
            std::collections::btree_map::Entry::Vacant(entry) => {
                let v = ty_func(idx)?;
                Ok(entry.insert(v))
            },
        }
    }

    #[inline(always)]
    pub(crate) fn get_field_type_and_struct_type(
        &mut self,
        idx: FieldInstantiationIndex,
        resolver: &Resolver,
        ty_args: &[Type],
    ) -> PartialVMResult<((&Type, NumTypeNodes), (&Type, NumTypeNodes))> {
        let ((field_ty, field_ty_count), (struct_ty, struct_ty_count)) =
            Self::get_or(&mut self.field_instantiation, idx, |idx| {
                let struct_type = resolver.field_instantiation_to_struct(idx, ty_args)?;
                let struct_ty_count = NumTypeNodes::new(struct_type.num_nodes() as u64);
                let field_ty = resolver.get_generic_field_ty(idx, ty_args)?;
                let field_ty_count = NumTypeNodes::new(field_ty.num_nodes() as u64);
                Ok(((field_ty, field_ty_count), (struct_type, struct_ty_count)))
            })?;
        Ok(((field_ty, *field_ty_count), (struct_ty, *struct_ty_count)))
    }

    pub(crate) fn get_variant_field_type_and_struct_type(
        &mut self,
        idx: VariantFieldInstantiationIndex,
        resolver: &Resolver,
        ty_args: &[Type],
    ) -> PartialVMResult<((&Type, NumTypeNodes), (&Type, NumTypeNodes))> {
        let ((field_ty, field_ty_count), (struct_ty, struct_ty_count)) =
            Self::get_or(&mut self.variant_field_instantiation, idx, |idx| {
                let info = resolver.variant_field_instantiation_info_at(idx);
                let struct_type = resolver.create_struct_instantiation_ty(
                    &info.definition_struct_type,
                    &info.instantiation,
                    ty_args,
                )?;
                let struct_ty_count = NumTypeNodes::new(struct_type.num_nodes() as u64);
                let field_ty = resolver.instantiate_ty(
                    &info.uninstantiated_field_ty,
                    ty_args,
                    &info.instantiation,
                )?;
                let field_ty_count = NumTypeNodes::new(field_ty.num_nodes() as u64);
                Ok(((field_ty, field_ty_count), (struct_type, struct_ty_count)))
            })?;
        Ok(((field_ty, *field_ty_count), (struct_ty, *struct_ty_count)))
    }

    #[inline(always)]
    pub(crate) fn get_struct_type(
        &mut self,
        idx: StructDefInstantiationIndex,
        resolver: &Resolver,
        ty_args: &[Type],
    ) -> PartialVMResult<(&Type, NumTypeNodes)> {
        let (ty, ty_count) = Self::get_or(&mut self.struct_def_instantiation_type, idx, |idx| {
            let ty = resolver.get_generic_struct_ty(idx, ty_args)?;
            let ty_count = NumTypeNodes::new(ty.num_nodes() as u64);
            Ok((ty, ty_count))
        })?;
        Ok((ty, *ty_count))
    }

    #[inline(always)]
    pub(crate) fn get_struct_variant_type(
        &mut self,
        idx: StructVariantInstantiationIndex,
        resolver: &Resolver,
        ty_args: &[Type],
    ) -> PartialVMResult<(&Type, NumTypeNodes)> {
        let (ty, ty_count) =
            Self::get_or(&mut self.struct_variant_instantiation_type, idx, |idx| {
                let info = resolver.get_struct_variant_instantiation_at(idx);
                let ty = resolver.create_struct_instantiation_ty(
                    &info.definition_struct_type,
                    &info.instantiation,
                    ty_args,
                )?;
                let ty_count = NumTypeNodes::new(ty.num_nodes() as u64);
                Ok((ty, ty_count))
            })?;
        Ok((ty, *ty_count))
    }

    #[inline(always)]
    pub(crate) fn get_struct_fields_types(
        &mut self,
        idx: StructDefInstantiationIndex,
        resolver: &Resolver,
        ty_args: &[Type],
    ) -> PartialVMResult<&[(Type, NumTypeNodes)]> {
        Ok(Self::get_or(
            &mut self.struct_field_type_instantiation,
            idx,
            |idx| {
                Ok(resolver
                    .instantiate_generic_struct_fields(idx, ty_args)?
                    .into_iter()
                    .map(|ty| {
                        let num_nodes = NumTypeNodes::new(ty.num_nodes() as u64);
                        (ty, num_nodes)
                    })
                    .collect::<Vec<_>>())
            },
        )?)
    }

    #[inline(always)]
    pub(crate) fn get_struct_variant_fields_types(
        &mut self,
        idx: StructVariantInstantiationIndex,
        resolver: &Resolver,
        ty_args: &[Type],
    ) -> PartialVMResult<&[(Type, NumTypeNodes)]> {
        Ok(Self::get_or(
            &mut self.struct_variant_field_type_instantiation,
            idx,
            |idx| {
                Ok(resolver
                    .instantiate_generic_struct_variant_fields(idx, ty_args)?
                    .into_iter()
                    .map(|ty| {
                        let num_nodes = NumTypeNodes::new(ty.num_nodes() as u64);
                        (ty, num_nodes)
                    })
                    .collect::<Vec<_>>())
            },
        )?)
    }

    #[inline(always)]
    pub(crate) fn get_signature_index_type(
        &mut self,
        idx: SignatureIndex,
        resolver: &Resolver,
        ty_args: &[Type],
    ) -> PartialVMResult<(&Type, NumTypeNodes)> {
        let (ty, ty_count) = Self::get_or(&mut self.single_sig_token_type, idx, |idx| {
            let ty = resolver.instantiate_single_type(idx, ty_args)?;
            let ty_count = NumTypeNodes::new(ty.num_nodes() as u64);
            Ok((ty, ty_count))
        })?;
        Ok((ty, *ty_count))
    }
}
