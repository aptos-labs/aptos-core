// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{frame::Frame, interpreter_caches::InstructionCacheBudget, LoadedFunction};
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
    /// Same as above, but for variant field instantiations
    variant_field_instantiation:
        BTreeMap<VariantFieldInstantiationIndex, ((Type, NumTypeNodes), (Type, NumTypeNodes))>,
    /// Maps signature index to a tuple of (Type, NumTypeNodes, depth) for single signature tokens.
    /// This cache stores instantiated types for signatures with a single type parameter.
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
    ///
    /// INVARIANT: `per_instruction_cache` is a pure memoization of [Self::function_cache] for
    /// `Call` sites and [Self::generic_function_cache] for `CallGeneric` sites. Those two maps,
    /// not this cache, decide whether a call site is resolved and gas is charged.
    ///
    /// A slot is filled only after that call site's entry in the matching map exists, and neither
    /// the slot nor that map entry is ever removed. Both are owned by the enclosing
    /// [FrameTypeCache], which lives for one interpreter invocation.
    ///
    /// A hit in `per_instruction_cache` therefore guarantees the matching map lookup would have
    /// hit too, and neither path charges gas; a miss must fall through to that map rather than
    /// resolving the call itself. This cache can consequently be shorter than the bytecode, or
    /// empty, with no effect on gas or execution results.
    per_instruction_cache: Vec<PerInstructionCache>,

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
    /// Returns a tuple of (Type, NumTypeNodes, depth) for the signature at the given index.
    /// The Type is the instantiated type, NumTypeNodes is the node count for gas metering,
    /// and depth is the maximum depth of the type tree.
    pub(crate) fn get_signature_index_type(
        &mut self,
        idx: SignatureIndex,
        frame: &Frame,
    ) -> PartialVMResult<(&Type, NumTypeNodes, usize)> {
        let (ty, ty_count, depth) = get_or_insert!(&mut self.single_sig_token_type, idx, {
            let ty = frame.instantiate_single_type(idx)?;
            let (ty_count, depth) = ty.num_nodes_with_max_depth();
            let ty_count = NumTypeNodes::new(ty_count as u64);
            (ty, ty_count, depth)
        });
        Ok((ty, *ty_count, *depth))
    }

    /// Returns the memoized entry for the instruction at `pc`, if any.
    ///
    /// A `None` result means "not memoized" and must be handled by falling through to
    /// [Self::function_cache] / [Self::generic_function_cache]. See the invariant on
    /// [Self::per_instruction_cache].
    pub(crate) fn instruction_cache_at(&self, pc: u16) -> Option<&PerInstructionCache> {
        self.per_instruction_cache.get(pc as usize)
    }

    /// Best-effort memoization of the resolved call target at `pc`.
    ///
    /// Dropping the write is always safe: it only costs a map lookup next time, never a different
    /// result or a different gas charge. See the invariant on [Self::per_instruction_cache].
    pub(crate) fn memoize_instruction(&mut self, pc: u16, entry: PerInstructionCache) {
        if let Some(slot) = self.per_instruction_cache.get_mut(pc as usize) {
            *slot = entry;
        }
    }

    pub(crate) fn make_rc() -> Rc<RefCell<Self>> {
        Rc::new(RefCell::<Self>::new(Default::default()))
    }

    /// Creates a frame cache for a function with `code_size` instructions, giving it a
    /// per-instruction cache only if `budget` can cover it. When the budget is exhausted the
    /// cache still works, just without the per-instruction fast path.
    pub(crate) fn make_rc_for_code_size(
        code_size: usize,
        budget: &mut InstructionCacheBudget,
    ) -> Rc<RefCell<Self>> {
        let mut frame_cache = Self::default();
        if budget.try_reserve(code_size) {
            frame_cache
                .per_instruction_cache
                .resize(code_size, PerInstructionCache::Nothing);
        }
        Rc::new(RefCell::new(frame_cache))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interpreter_caches::InstructionCacheBudgetOverride;

    fn budget_with(max_entries: usize) -> InstructionCacheBudget {
        let _override = InstructionCacheBudgetOverride::new(max_entries);
        InstructionCacheBudget::new()
    }

    fn allocated_entries(frame_cache: &Rc<RefCell<FrameTypeCache>>) -> usize {
        frame_cache.borrow().per_instruction_cache.len()
    }

    #[test]
    fn allocated_instruction_cache_is_addressable_within_bounds() {
        let mut budget = budget_with(1_000);
        let frame_cache = FrameTypeCache::make_rc_for_code_size(500, &mut budget);
        assert_eq!(allocated_entries(&frame_cache), 500);

        let mut frame_cache = frame_cache.borrow_mut();
        assert!(matches!(
            frame_cache.instruction_cache_at(0),
            Some(PerInstructionCache::Nothing)
        ));
        assert!(matches!(
            frame_cache.instruction_cache_at(499),
            Some(PerInstructionCache::Nothing)
        ));
        assert!(frame_cache.instruction_cache_at(500).is_none());

        // Out of range writes are dropped rather than panicking.
        frame_cache.memoize_instruction(500, PerInstructionCache::Nothing);
    }

    /// Absence of a per-instruction cache must read as a plain miss. Before the fix the access
    /// sites indexed the vector directly, so an unallocated cache was not representable: reads
    /// would have panicked instead of falling through to the function caches.
    #[test]
    fn absent_instruction_cache_reads_as_a_miss() {
        let mut budget = budget_with(0);
        let frame_cache = FrameTypeCache::make_rc_for_code_size(500, &mut budget);
        assert_eq!(allocated_entries(&frame_cache), 0);

        let mut frame_cache = frame_cache.borrow_mut();
        assert!(frame_cache.instruction_cache_at(0).is_none());
        assert!(frame_cache.instruction_cache_at(499).is_none());

        // Writes are dropped rather than panicking, and the cache stays absent.
        frame_cache.memoize_instruction(0, PerInstructionCache::Nothing);
        assert!(frame_cache.instruction_cache_at(0).is_none());
    }

    /// The shape of APTOSNT-487: one long generic callee reached at many distinct type vectors,
    /// each of which gets its own frame cache sized by the callee's bytecode length.
    #[test]
    fn aggregate_allocation_is_bounded_across_instantiations() {
        const CODE_SIZE: usize = 10_000;
        const INSTANTIATIONS: usize = 1_000;
        const MAX_ENTRIES: usize = 100_000;

        // Before the fix the resize was unconditional, so the total allocated was the product of
        // two attacker-controlled quantities with no ceiling.
        let unbounded_entries = CODE_SIZE * INSTANTIATIONS;

        let mut budget = budget_with(MAX_ENTRIES);
        let frame_caches = (0..INSTANTIATIONS)
            .map(|_| FrameTypeCache::make_rc_for_code_size(CODE_SIZE, &mut budget))
            .collect::<Vec<_>>();

        let allocated = frame_caches.iter().map(allocated_entries).sum::<usize>();
        assert_eq!(allocated, MAX_ENTRIES);
        assert_eq!(unbounded_entries / allocated, 100);

        // Exactly the first `MAX_ENTRIES / CODE_SIZE` instantiations get a per-instruction cache;
        // the rest fall back to the function caches.
        let with_cache = frame_caches
            .iter()
            .filter(|frame_cache| allocated_entries(frame_cache) > 0)
            .count();
        assert_eq!(with_cache, MAX_ENTRIES / CODE_SIZE);
        assert_eq!(allocated_entries(frame_caches.last().unwrap()), 0);
    }

    /// A refused reservation must not consume budget, otherwise one oversized function would
    /// starve every later one.
    #[test]
    fn refused_allocation_leaves_budget_for_smaller_functions() {
        let mut budget = budget_with(1_000);

        let too_big = FrameTypeCache::make_rc_for_code_size(1_001, &mut budget);
        assert_eq!(allocated_entries(&too_big), 0);

        let fits = FrameTypeCache::make_rc_for_code_size(1_000, &mut budget);
        assert_eq!(allocated_entries(&fits), 1_000);
    }
}
