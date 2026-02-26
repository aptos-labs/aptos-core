// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{frame::Frame, LoadedFunction};
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
use std::{
    cell::RefCell,
    collections::BTreeMap,
    rc::{Rc, Weak},
};

/// Variants for each individual instruction cache. Should make sure
/// that the memory footprint of each variant is small. This is an
/// enum that is expected to grow in the future.
#[derive(Clone)]
pub(crate) enum PerInstructionCache {
    Nothing,
    // Instruction cache is part of the frame cache, so it has to store weak references to prevent
    // memory leaks for recursive functions.
    Call(Rc<LoadedFunction>, Weak<RefCell<FrameTypeCache>>),
    CallGeneric(Rc<LoadedFunction>, Weak<RefCell<FrameTypeCache>>),
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
    single_sig_token_type: BTreeMap<SignatureIndex, (Type, NumTypeNodes, usize)>,
    /// Stores a variant for each individual instruction in the
    /// function's bytecode. We keep the size of the variant to be
    /// small. The caches are indexed by the index of the given
    /// bytecode instruction in the function body.
    ///
    /// Important! - If entry is present for a given instruction, then
    /// we do NOT need to re-check for any errors that only depend on
    /// the argument of the bytecode instructions, for which it is
    /// guaranteed that everything will be exactly the same as when we
    /// did the insertion.
    pub(crate) per_instruction_cache: Vec<PerInstructionCache>,

    /// Caches function and its cache for non-generic handles. Uses weak reference for cache to
    /// prevent memory leaks for recursive functions.
    pub(crate) function_cache:
        BTreeMap<FunctionHandleIndex, (Rc<LoadedFunction>, Weak<RefCell<FrameTypeCache>>)>,
    /// Caches function and its cache for generic handles. Like function cache, uses weak reference
    /// for cache to prevent memory leaks for recursive functions.
    pub(crate) generic_function_cache:
        BTreeMap<FunctionInstantiationIndex, (Rc<LoadedFunction>, Weak<RefCell<FrameTypeCache>>)>,

    /// Cached instantiated local types for generic functions.
    pub(crate) instantiated_local_tys: Option<Rc<[Type]>>,
    /// Cached number of type nodes per instantiated local type for gas charging re-use.
    pub(crate) instantiated_local_ty_counts: Option<Rc<[NumTypeNodes]>>,
}

macro_rules! get_or_insert {
    ($map:expr, $idx:expr, $ty_func:tt) => {
        match $map.entry($idx) {
            std::collections::btree_map::Entry::Occupied(entry) => entry.into_mut(),
            std::collections::btree_map::Entry::Vacant(entry) => {
                let v = { $ty_func };
                entry.insert(v)
            },
        }
    };
}

impl FrameTypeCache {
    // note(inline): do not inline, increases size a lot, might even decrease the performance
    pub(crate) fn get_field_type_and_struct_type(
        &mut self,
        idx: FieldInstantiationIndex,
        frame: &Frame,
    ) -> PartialVMResult<((&Type, NumTypeNodes), (&Type, NumTypeNodes))> {
        let ((field_ty, field_ty_count), (struct_ty, struct_ty_count)) =
            get_or_insert!(&mut self.field_instantiation, idx, {
                let struct_type = frame.field_instantiation_to_struct(idx)?;
                let struct_ty_count = NumTypeNodes::new(struct_type.num_nodes() as u64);
                let field_ty = frame.get_generic_field_ty(idx)?;
                let field_ty_count = NumTypeNodes::new(field_ty.num_nodes() as u64);
                ((field_ty, field_ty_count), (struct_type, struct_ty_count))
            });
        Ok(((field_ty, *field_ty_count), (struct_ty, *struct_ty_count)))
    }

    // note(inline): do not inline, increases size a lot, might even decrease the performance
    pub(crate) fn get_variant_field_type_and_struct_type(
        &mut self,
        idx: VariantFieldInstantiationIndex,
        frame: &Frame,
    ) -> PartialVMResult<((&Type, NumTypeNodes), (&Type, NumTypeNodes))> {
        let ((field_ty, field_ty_count), (struct_ty, struct_ty_count)) =
            get_or_insert!(&mut self.variant_field_instantiation, idx, {
                let info = frame.variant_field_instantiation_info_at(idx);
                let struct_type = frame.create_struct_instantiation_ty(
                    &info.definition_struct_type,
                    &info.instantiation,
                )?;
                let struct_ty_count = NumTypeNodes::new(struct_type.num_nodes() as u64);
                let field_ty =
                    frame.instantiate_ty(&info.uninstantiated_field_ty, &info.instantiation)?;
                let field_ty_count = NumTypeNodes::new(field_ty.num_nodes() as u64);
                ((field_ty, field_ty_count), (struct_type, struct_ty_count))
            });
        Ok(((field_ty, *field_ty_count), (struct_ty, *struct_ty_count)))
    }

    // note(inline): do not inline, increases size a lot, might even decrease the performance
    pub(crate) fn get_struct_type(
        &mut self,
        idx: StructDefInstantiationIndex,
        frame: &Frame,
    ) -> PartialVMResult<(&Type, NumTypeNodes)> {
        let (ty, ty_count) = get_or_insert!(&mut self.struct_def_instantiation_type, idx, {
            let ty = frame.get_generic_struct_ty(idx)?;
            let ty_count = NumTypeNodes::new(ty.num_nodes() as u64);
            (ty, ty_count)
        });
        Ok((ty, *ty_count))
    }

    // note(inline): do not inline, increases size a lot, might even decrease the performance
    pub(crate) fn get_struct_variant_type(
        &mut self,
        idx: StructVariantInstantiationIndex,
        frame: &Frame,
    ) -> PartialVMResult<(&Type, NumTypeNodes)> {
        let (ty, ty_count) = get_or_insert!(&mut self.struct_variant_instantiation_type, idx, {
            let info = frame.get_struct_variant_instantiation_at(idx);
            let ty = frame.create_struct_instantiation_ty(
                &info.definition_struct_type,
                &info.instantiation,
            )?;
            let ty_count = NumTypeNodes::new(ty.num_nodes() as u64);
            (ty, ty_count)
        });
        Ok((ty, *ty_count))
    }

    // note(inline): do not inline, increases size a lot, might even decrease the performance
    pub(crate) fn get_struct_fields_types(
        &mut self,
        idx: StructDefInstantiationIndex,
        frame: &Frame,
    ) -> PartialVMResult<&[(Type, NumTypeNodes)]> {
        Ok(get_or_insert!(
            &mut self.struct_field_type_instantiation,
            idx,
            {
                frame
                    .instantiate_generic_struct_fields(idx)?
                    .into_iter()
                    .map(|ty| {
                        let num_nodes = NumTypeNodes::new(ty.num_nodes() as u64);
                        (ty, num_nodes)
                    })
                    .collect::<Vec<_>>()
            }
        ))
    }

    // note(inline): do not inline, increases size a lot, might even decrease the performance
    pub(crate) fn get_struct_variant_fields_types(
        &mut self,
        idx: StructVariantInstantiationIndex,
        frame: &Frame,
    ) -> PartialVMResult<&[(Type, NumTypeNodes)]> {
        Ok(get_or_insert!(
            &mut self.struct_variant_field_type_instantiation,
            idx,
            {
                frame
                    .instantiate_generic_struct_variant_fields(idx)?
                    .into_iter()
                    .map(|ty| {
                        let num_nodes = NumTypeNodes::new(ty.num_nodes() as u64);
                        (ty, num_nodes)
                    })
                    .collect::<Vec<_>>()
            }
        ))
    }

    // note(inline): do not inline, increases size a lot, might even decrease the performance
    pub(crate) fn get_signature_index_type(
        &mut self,
        idx: SignatureIndex,
        frame: &Frame,
    ) -> PartialVMResult<(&Type, NumTypeNodes, usize)> {
        let (ty, ty_count, depth) = get_or_insert!(&mut self.single_sig_token_type, idx, {
            let ty = frame.instantiate_single_type(idx)?;
            let (ty_count, depth) = ty.num_nodes_with_depth();
            let ty_count = NumTypeNodes::new(ty_count as u64);
            (ty, ty_count, depth)
        });
        Ok((ty, *ty_count, *depth))
    }

    pub(crate) fn make_rc() -> Rc<RefCell<Self>> {
        Rc::new(RefCell::<Self>::new(Default::default()))
    }

    pub(crate) fn make_rc_for_function(function: &LoadedFunction) -> Rc<RefCell<Self>> {
        let frame_cache = Rc::new(RefCell::<Self>::new(Default::default()));

        frame_cache
            .borrow_mut()
            .per_instruction_cache
            .resize(function.code_size(), PerInstructionCache::Nothing);
        frame_cache
    }
}
