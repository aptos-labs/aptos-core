// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

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
    collections::{btree_map::Entry, BTreeMap},
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

struct CachedFieldTypes {
    types: Vec<Type>,
    // TODO: remove individual counts.
    counts: Vec<NumTypeNodes>,
    counts_sum: NumTypeNodes,
}

impl CachedFieldTypes {
    fn for_struct_fields(idx: StructDefInstantiationIndex, frame: &Frame) -> PartialVMResult<Self> {
        Ok(Self::new(frame.instantiate_generic_struct_fields(idx)?))
    }

    fn for_struct_variant_fields(
        idx: StructVariantInstantiationIndex,
        frame: &Frame,
    ) -> PartialVMResult<Self> {
        Ok(Self::new(
            frame.instantiate_generic_struct_variant_fields(idx)?,
        ))
    }

    fn new(types: Vec<Type>) -> Self {
        let mut counts = Vec::with_capacity(types.len());
        let mut counts_sum = NumTypeNodes::new(0);
        for ty in &types {
            let count = NumTypeNodes::new(ty.num_nodes() as u64);
            counts_sum += count;
            counts.push(count);
        }
        Self {
            types,
            counts,
            counts_sum,
        }
    }
}

struct CachedStructType {
    ty: Type,
    count: NumTypeNodes,
}

impl CachedStructType {
    fn for_struct(idx: StructDefInstantiationIndex, frame: &Frame) -> PartialVMResult<Self> {
        let ty = frame.get_generic_struct_ty(idx)?;
        let count = NumTypeNodes::new(ty.num_nodes() as u64);
        Ok(Self { ty, count })
    }

    fn for_struct_variant(
        idx: StructVariantInstantiationIndex,
        frame: &Frame,
    ) -> PartialVMResult<Self> {
        let info = frame.get_struct_variant_instantiation_at(idx);
        let ty = frame
            .create_struct_instantiation_ty(&info.definition_struct_type, &info.instantiation)?;
        let count = NumTypeNodes::new(ty.num_nodes() as u64);
        Ok(Self { ty, count })
    }
}

struct CachedFieldAndStructType {
    field_ty: Type,
    struct_ty: Type,
    // TODO: remove individual counts.
    field_count: NumTypeNodes,
    struct_count: NumTypeNodes,
    counts_sum: NumTypeNodes,
}

impl CachedFieldAndStructType {
    fn for_struct(idx: FieldInstantiationIndex, frame: &Frame) -> PartialVMResult<Self> {
        let struct_ty = frame.field_instantiation_to_struct(idx)?;
        let field_ty = frame.get_generic_field_ty(idx)?;
        Ok(Self::new(field_ty, struct_ty))
    }

    fn for_struct_variant(
        idx: VariantFieldInstantiationIndex,
        frame: &Frame,
    ) -> PartialVMResult<Self> {
        let info = frame.variant_field_instantiation_info_at(idx);
        let struct_ty = frame
            .create_struct_instantiation_ty(&info.definition_struct_type, &info.instantiation)?;
        let field_ty = frame.instantiate_ty(&info.uninstantiated_field_ty, &info.instantiation)?;
        Ok(Self::new(field_ty, struct_ty))
    }

    fn new(field_ty: Type, struct_ty: Type) -> Self {
        let field_count = NumTypeNodes::new(field_ty.num_nodes() as u64);
        let struct_count = NumTypeNodes::new(struct_ty.num_nodes() as u64);
        let counts_sum = field_count + struct_count;
        Self {
            field_ty,
            struct_ty,
            field_count,
            struct_count,
            counts_sum,
        }
    }
}

struct CachedSignatureType {
    ty: Type,
    count: NumTypeNodes,
    depth: usize,
}

impl CachedSignatureType {
    fn new(idx: SignatureIndex, frame: &Frame) -> PartialVMResult<Self> {
        let ty = frame.instantiate_single_type(idx)?;
        let (num_nodes, depth) = ty.num_nodes_with_max_depth();
        let count = NumTypeNodes::new(num_nodes as u64);
        Ok(Self { ty, count, depth })
    }
}

pub(crate) struct FrameTypeCache {
    /// Maps struct definition to runtime type and its size.
    struct_defs: BTreeMap<StructDefInstantiationIndex, CachedStructType>,
    /// Maps struct definition to its field runtime types and their sizes.
    struct_def_fields: BTreeMap<StructDefInstantiationIndex, CachedFieldTypes>,
    /// Maps field to its type and struct type that defines it. For both, type
    /// size is also stored.
    struct_fields: BTreeMap<FieldInstantiationIndex, CachedFieldAndStructType>,

    /// Maps variant definition to runtime type and its size.
    struct_variant_defs: BTreeMap<StructVariantInstantiationIndex, CachedStructType>,
    /// Maps variant definition to its field runtime types and their sizes.
    struct_variant_def_fields: BTreeMap<StructVariantInstantiationIndex, CachedFieldTypes>,
    /// Maps variant field to its type and struct variant type that defines it.
    /// For both, type size is also stored.
    struct_variant_fields: BTreeMap<VariantFieldInstantiationIndex, CachedFieldAndStructType>,

    /// Maps signature (used for vector element types) to runtime type, and its
    /// derived information: type size and type depth.
    signatures: BTreeMap<SignatureIndex, CachedSignatureType>,

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
    /// Cached number of type nodes per local type for gas charging re-use.
    pub(crate) local_ty_counts: Option<Rc<[NumTypeNodes]>>,

    /// Governs how type sizes are returned from cache APIs for gas metering.
    /// If true, charging is suboptimal (charge on cache hit, multiple charges
    /// over a list of type sizes). If false, charging is only on cache miss,
    /// charges are aggregated when possible.
    pub(crate) charge_create_ty_on_cache_hit: bool,
}

impl FrameTypeCache {
    fn empty(charge_create_ty_on_cache_hit: bool) -> Self {
        Self {
            struct_defs: BTreeMap::new(),
            struct_def_fields: BTreeMap::new(),
            struct_fields: BTreeMap::new(),
            struct_variant_defs: BTreeMap::new(),
            struct_variant_def_fields: BTreeMap::new(),
            struct_variant_fields: BTreeMap::new(),
            signatures: BTreeMap::new(),
            per_instruction_cache: vec![],
            function_cache: BTreeMap::new(),
            generic_function_cache: BTreeMap::new(),
            instantiated_local_tys: None,
            local_ty_counts: None,
            charge_create_ty_on_cache_hit,
        }
    }

    // note(inline): do not inline, increases size a lot, might even decrease the performance
    pub(crate) fn get_field_type_and_struct_type(
        &mut self,
        idx: FieldInstantiationIndex,
        frame: &Frame,
    ) -> PartialVMResult<(&Type, &Type)> {
        Ok(match self.struct_fields.entry(idx) {
            Entry::Vacant(e) => {
                let cached = e.insert(CachedFieldAndStructType::for_struct(idx, frame)?);
                (&cached.field_ty, &cached.struct_ty)
            },
            Entry::Occupied(e) => {
                let cached = e.into_mut();
                (&cached.field_ty, &cached.struct_ty)
            },
        })
    }

    pub(crate) fn get_field_type_and_struct_type_counts(
        &mut self,
        idx: FieldInstantiationIndex,
        frame: &Frame,
    ) -> PartialVMResult<(Option<(NumTypeNodes, NumTypeNodes)>, Option<NumTypeNodes>)> {
        Ok(match self.struct_fields.entry(idx) {
            Entry::Vacant(e) => {
                let cached = e.insert(CachedFieldAndStructType::for_struct(idx, frame)?);
                if self.charge_create_ty_on_cache_hit {
                    (Some((cached.field_count, cached.struct_count)), None)
                } else {
                    (None, Some(cached.counts_sum))
                }
            },
            Entry::Occupied(e) => {
                let cached = e.get();
                let legacy_charge = self
                    .charge_create_ty_on_cache_hit
                    .then_some((cached.field_count, cached.struct_count));
                (legacy_charge, None)
            },
        })
    }

    // note(inline): do not inline, increases size a lot, might even decrease the performance
    pub(crate) fn get_variant_field_type_and_struct_type(
        &mut self,
        idx: VariantFieldInstantiationIndex,
        frame: &Frame,
    ) -> PartialVMResult<(&Type, &Type)> {
        Ok(match self.struct_variant_fields.entry(idx) {
            Entry::Vacant(e) => {
                let cached = e.insert(CachedFieldAndStructType::for_struct_variant(idx, frame)?);
                (&cached.field_ty, &cached.struct_ty)
            },
            Entry::Occupied(e) => {
                let cached = e.into_mut();
                (&cached.field_ty, &cached.struct_ty)
            },
        })
    }

    pub(crate) fn get_variant_field_type_and_struct_type_counts(
        &mut self,
        idx: VariantFieldInstantiationIndex,
        frame: &Frame,
    ) -> PartialVMResult<(Option<(NumTypeNodes, NumTypeNodes)>, Option<NumTypeNodes>)> {
        Ok(match self.struct_variant_fields.entry(idx) {
            Entry::Vacant(e) => {
                let cached = e.insert(CachedFieldAndStructType::for_struct_variant(idx, frame)?);
                if self.charge_create_ty_on_cache_hit {
                    (Some((cached.field_count, cached.struct_count)), None)
                } else {
                    (None, Some(cached.counts_sum))
                }
            },
            Entry::Occupied(e) => {
                let cached = e.get();
                let legacy_charge = self
                    .charge_create_ty_on_cache_hit
                    .then_some((cached.field_count, cached.struct_count));
                (legacy_charge, None)
            },
        })
    }

    // note(inline): do not inline, increases size a lot, might even decrease the performance
    pub(crate) fn get_struct_type_and_count(
        &mut self,
        idx: StructDefInstantiationIndex,
        frame: &Frame,
    ) -> PartialVMResult<(&Type, Option<NumTypeNodes>)> {
        let charge_on_hit = self.charge_create_ty_on_cache_hit;
        Ok(match self.struct_defs.entry(idx) {
            Entry::Vacant(e) => {
                let cached = e.insert(CachedStructType::for_struct(idx, frame)?);
                (&cached.ty, Some(cached.count))
            },
            Entry::Occupied(e) => {
                let cached = e.into_mut();
                (&cached.ty, charge_on_hit.then_some(cached.count))
            },
        })
    }

    pub(crate) fn get_struct_type(
        &mut self,
        idx: StructDefInstantiationIndex,
        frame: &Frame,
    ) -> PartialVMResult<&Type> {
        Ok(self.get_struct_type_and_count(idx, frame)?.0)
    }

    // note(inline): do not inline, increases size a lot, might even decrease the performance
    pub(crate) fn get_struct_variant_type_and_count(
        &mut self,
        idx: StructVariantInstantiationIndex,
        frame: &Frame,
    ) -> PartialVMResult<(&Type, Option<NumTypeNodes>)> {
        let charge_on_hit = self.charge_create_ty_on_cache_hit;
        Ok(match self.struct_variant_defs.entry(idx) {
            Entry::Vacant(e) => {
                let cached = e.insert(CachedStructType::for_struct_variant(idx, frame)?);
                (&cached.ty, Some(cached.count))
            },
            Entry::Occupied(e) => {
                let cached = e.into_mut();
                (&cached.ty, charge_on_hit.then_some(cached.count))
            },
        })
    }

    pub(crate) fn get_struct_variant_type(
        &mut self,
        idx: StructVariantInstantiationIndex,
        frame: &Frame,
    ) -> PartialVMResult<&Type> {
        Ok(self.get_struct_variant_type_and_count(idx, frame)?.0)
    }

    // note(inline): do not inline, increases size a lot, might even decrease the performance
    pub(crate) fn get_struct_fields_types(
        &mut self,
        idx: StructDefInstantiationIndex,
        frame: &Frame,
    ) -> PartialVMResult<&[Type]> {
        Ok(match self.struct_def_fields.entry(idx) {
            Entry::Vacant(e) => e
                .insert(CachedFieldTypes::for_struct_fields(idx, frame)?)
                .types
                .as_slice(),
            Entry::Occupied(e) => e.into_mut().types.as_slice(),
        })
    }

    pub(crate) fn get_struct_fields_type_counts_and_sum(
        &mut self,
        idx: StructDefInstantiationIndex,
        frame: &Frame,
    ) -> PartialVMResult<(&[NumTypeNodes], Option<NumTypeNodes>)> {
        Ok(match self.struct_def_fields.entry(idx) {
            Entry::Vacant(e) => {
                let cached = e.insert(CachedFieldTypes::for_struct_fields(idx, frame)?);
                if self.charge_create_ty_on_cache_hit {
                    (cached.counts.as_slice(), None)
                } else {
                    (&[], Some(cached.counts_sum))
                }
            },
            Entry::Occupied(e) => {
                if self.charge_create_ty_on_cache_hit {
                    (e.into_mut().counts.as_slice(), None)
                } else {
                    (&[], None)
                }
            },
        })
    }

    // note(inline): do not inline, increases size a lot, might even decrease the performance
    pub(crate) fn get_struct_variant_fields_types(
        &mut self,
        idx: StructVariantInstantiationIndex,
        frame: &Frame,
    ) -> PartialVMResult<&[Type]> {
        Ok(match self.struct_variant_def_fields.entry(idx) {
            Entry::Vacant(e) => e
                .insert(CachedFieldTypes::for_struct_variant_fields(idx, frame)?)
                .types
                .as_slice(),
            Entry::Occupied(e) => e.into_mut().types.as_slice(),
        })
    }

    pub(crate) fn get_struct_variant_fields_type_counts_and_sum(
        &mut self,
        idx: StructVariantInstantiationIndex,
        frame: &Frame,
    ) -> PartialVMResult<(&[NumTypeNodes], Option<NumTypeNodes>)> {
        Ok(match self.struct_variant_def_fields.entry(idx) {
            Entry::Vacant(e) => {
                let cached = e.insert(CachedFieldTypes::for_struct_variant_fields(idx, frame)?);
                if self.charge_create_ty_on_cache_hit {
                    (cached.counts.as_slice(), None)
                } else {
                    (&[], Some(cached.counts_sum))
                }
            },
            Entry::Occupied(e) => {
                if self.charge_create_ty_on_cache_hit {
                    (e.into_mut().counts.as_slice(), None)
                } else {
                    (&[], None)
                }
            },
        })
    }

    // note(inline): do not inline, increases size a lot, might even decrease the performance
    /// Returns a tuple of fully-instantiated runtime type, number of nodes in
    /// it (for gas metering), a boolean to indicate if this was a cache miss
    /// (for gas metering), and its depth for the signature at the given index.
    pub(crate) fn get_signature_index_type_and_count_and_depth(
        &mut self,
        idx: SignatureIndex,
        frame: &Frame,
    ) -> PartialVMResult<(&Type, NumTypeNodes, bool, usize)> {
        Ok(match self.signatures.entry(idx) {
            Entry::Vacant(e) => {
                let cached = e.insert(CachedSignatureType::new(idx, frame)?);
                (&cached.ty, cached.count, true, cached.depth)
            },
            Entry::Occupied(e) => {
                let cached = e.into_mut();
                (&cached.ty, cached.count, false, cached.depth)
            },
        })
    }

    pub(crate) fn get_signature_index_type(
        &mut self,
        idx: SignatureIndex,
        frame: &Frame,
    ) -> PartialVMResult<&Type> {
        Ok(self
            .get_signature_index_type_and_count_and_depth(idx, frame)?
            .0)
    }

    pub(crate) fn get_signature_index_type_count(
        &mut self,
        idx: SignatureIndex,
        frame: &Frame,
    ) -> PartialVMResult<Option<NumTypeNodes>> {
        let (_, count, is_miss, _) =
            self.get_signature_index_type_and_count_and_depth(idx, frame)?;
        Ok((self.charge_create_ty_on_cache_hit || is_miss).then_some(count))
    }

    pub(crate) fn make_rc(charge_create_ty_on_cache_hit: bool) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(FrameTypeCache::empty(
            charge_create_ty_on_cache_hit,
        )))
    }

    pub(crate) fn make_rc_for_function(
        function: &LoadedFunction,
        charge_create_ty_on_cache_hit: bool,
    ) -> Rc<RefCell<Self>> {
        let frame_cache = Rc::new(RefCell::new(FrameTypeCache::empty(
            charge_create_ty_on_cache_hit,
        )));

        frame_cache
            .borrow_mut()
            .per_instruction_cache
            .resize(function.code_size(), PerInstructionCache::Nothing);
        frame_cache
    }
}
