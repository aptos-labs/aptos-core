// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    config::VMConfig,
    module_traversal::TraversalContext,
    storage::{
        layout_cache::DefiningModules, loader::traits::StructDefinitionLoader,
        ty_tag_converter::TypeTagConverter,
    },
    LayoutCacheEntry, LayoutCacheKey, RuntimeEnvironment,
};
use fxhash::FxHashMap;
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::{
    account_address::AccountAddress,
    ident_str,
    identifier::Identifier,
    int256,
    language_storage::{ModuleId, LEGACY_OPTION_VEC},
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
use std::{
    hash::{Hash, Hasher},
    sync::Arc,
};
use triomphe::Arc as TriompheArc;

/// Owned key for [LocalSinglePassStructLayoutCache] representing a fully-instantiated struct.
#[derive(Clone, Eq, PartialEq)]
struct StructLayoutKey {
    idx: StructNameIndex,
    ty_args: Vec<Type>,
}

/// Borrowed key for [LocalSinglePassStructLayoutCache] lookups, avoiding `ty_args.to_vec()`
/// allocation.
#[derive(Eq, PartialEq)]
struct StructLayoutKeyRef<'a> {
    idx: StructNameIndex,
    ty_args: &'a [Type],
}

impl hashbrown::Equivalent<StructLayoutKey> for StructLayoutKeyRef<'_> {
    fn equivalent(&self, other: &StructLayoutKey) -> bool {
        self.idx == other.idx && self.ty_args == other.ty_args.as_slice()
    }
}

// Ensure hash is the same as for StructLayoutKeyRef.
impl Hash for StructLayoutKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.idx.hash(state);
        self.ty_args.hash(state);
    }
}

// Ensure hash is the same as for StructLayoutKey.
impl Hash for StructLayoutKeyRef<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.idx.hash(state);
        self.ty_args.hash(state);
    }
}

type StructLayoutMap = hashbrown::HashMap<StructLayoutKey, (Arc<MoveStructLayout>, bool)>;

/// Cache for struct layouts constructed during a single layout construction pass.
/// On cache hit, the `Arc<MoveStructLayout>` is shared (cheap clone), and the
/// node count is not re-incremented, avoiding spurious `TOO_MANY_TYPE_NODES` errors.
/// Invariant: this cache must be scoped to one traversal only (one top-level
/// layout construction). Reusing it across passes could hide depth/cycle checks
/// and keep cyclic layouts alive; discarding it after the pass avoids leaks.
///
/// Uses separate maps for runtime vs annotated layouts because enum variant
/// fields are always processed with ANNOTATED=false, so a single map would
/// return a non-annotated layout when an annotated one is expected.
struct LocalSinglePassStructLayoutCache {
    runtime: StructLayoutMap,
    annotated: StructLayoutMap,
}

impl LocalSinglePassStructLayoutCache {
    fn new() -> Self {
        Self {
            runtime: StructLayoutMap::new(),
            annotated: StructLayoutMap::new(),
        }
    }

    fn select<const ANNOTATED: bool>(&mut self) -> &mut StructLayoutMap {
        if ANNOTATED {
            &mut self.annotated
        } else {
            &mut self.runtime
        }
    }
}

/// Per-allocation overhead: allocator metadata and size-class rounding, plus the reference counts
/// of an `Arc` (not counted by `size_of` of the value behind it). Sizes of the values themselves
/// are derived from `size_of` below, so they follow changes to the layout types automatically.
const ALLOCATION_OVERHEAD_IN_BYTES: usize = 16;

// Invariant: the overhead covers at least the strong and weak counts of an `Arc` allocation.
const _: () = assert!(ALLOCATION_OVERHEAD_IN_BYTES >= 2 * std::mem::size_of::<usize>());

/// Size of a layout node, counted as a struct node (the largest kind).
const PER_NODE_SIZE_ESTIMATE_IN_BYTES: usize = std::mem::size_of::<MoveTypeLayout>()
    + std::mem::size_of::<MoveStructLayout>()
    + ALLOCATION_OVERHEAD_IN_BYTES;

/// Size of an enum variant: its entry in the variants vector and its fields buffer.
const PER_VARIANT_SIZE_ESTIMATE_IN_BYTES: usize =
    std::mem::size_of::<Vec<MoveTypeLayout>>() + ALLOCATION_OVERHEAD_IN_BYTES;

/// Size of a defining module without its name: one copy in the set, one (with hash) in the vector.
const PER_MODULE_FIXED_SIZE_ESTIMATE_IN_BYTES: usize = std::mem::size_of::<ModuleId>()
    + std::mem::size_of::<(ModuleId, [u8; 32])>()
    + 2 * ALLOCATION_OVERHEAD_IN_BYTES;

/// Size of a defining module, including its name (stored twice).
fn defining_module_size_estimate_in_bytes(module_id: &ModuleId) -> usize {
    PER_MODULE_FIXED_SIZE_ESTIMATE_IN_BYTES.saturating_add(2 * module_id.name().len())
}

/// Size of a cached layout regardless of its contents: key, entry and modules.
const BASE_SIZE_ESTIMATE_IN_BYTES: usize = std::mem::size_of::<LayoutCacheKey>()
    + std::mem::size_of::<LayoutCacheEntry>()
    + std::mem::size_of::<DefiningModules>()
    + 2 * ALLOCATION_OVERHEAD_IN_BYTES;

/// Layouts with estimated size larger than this are never cached in the long-living layout cache.
/// With bounds on the number of nodes in a layout this is not expected to happen in practice.
pub(crate) const MAX_CACHED_LAYOUT_SIZE_ESTIMATE_IN_BYTES: usize = 1024 * 1024;

/// Counters accumulated during the construction of a single layout.
struct LayoutConstructionCounter {
    /// Number of nodes in the layout, used to enforce limits on the layout size.
    num_nodes: u64,
    /// Estimated number of bytes the layout (and its defining modules) occupy in memory. Used to
    /// bound the memory of the layout caches.
    size_estimate_in_bytes: usize,
}

impl LayoutConstructionCounter {
    fn new() -> Self {
        Self {
            num_nodes: 0,
            size_estimate_in_bytes: BASE_SIZE_ESTIMATE_IN_BYTES,
        }
    }

    fn add_node(&mut self) {
        self.num_nodes += 1;
        self.size_estimate_in_bytes = self
            .size_estimate_in_bytes
            .saturating_add(PER_NODE_SIZE_ESTIMATE_IN_BYTES);
    }

    fn add_variants(&mut self, num_variants: usize) {
        self.size_estimate_in_bytes = self
            .size_estimate_in_bytes
            .saturating_add(num_variants.saturating_mul(PER_VARIANT_SIZE_ESTIMATE_IN_BYTES));
    }

    fn add_defining_module(&mut self, module_id: &ModuleId) {
        self.size_estimate_in_bytes = self
            .size_estimate_in_bytes
            .saturating_add(defining_module_size_estimate_in_bytes(module_id));
    }
}

/// Stores type layout as well as a flag if it contains any delayed fields.
#[derive(Debug, Clone)]
pub struct LayoutWithDelayedFields {
    layout: TriompheArc<MoveTypeLayout>,
    contains_delayed_fields: bool,
}

impl LayoutWithDelayedFields {
    /// Returns the number of bytes the layout serializes to. Walks the layout.
    pub(crate) fn serialized_size(&self) -> u64 {
        self.layout.serialized_size()
    }

    /// If layout contains delayed fields, returns [None]. If there are no delayed fields, the
    /// layout is returned.
    pub fn into_layout_when_has_no_delayed_fields(self) -> Option<TriompheArc<MoveTypeLayout>> {
        (!self.contains_delayed_fields).then_some(self.layout)
    }

    /// If layout does not contain delayed fields, returns [None]. If there are delayed fields, the
    /// layout is returned.
    pub fn layout_when_contains_delayed_fields(&self) -> Option<&MoveTypeLayout> {
        self.contains_delayed_fields.then_some(&self.layout)
    }

    /// Unpacks and returns the layout and delayed fields flag for the caller to handle.
    pub fn unpack(self) -> (TriompheArc<MoveTypeLayout>, bool) {
        (self.layout, self.contains_delayed_fields)
    }
}

// Defined here because only this module can construct a [LayoutWithDelayedFields].
#[cfg(any(test, feature = "testing"))]
impl LayoutCacheEntry {
    /// Returns an entry with a dummy layout of the specified size estimate. Used for tests only.
    pub fn new_for_testing(size_estimate_in_bytes: usize) -> Self {
        let layout = LayoutWithDelayedFields {
            layout: TriompheArc::new(MoveTypeLayout::U8),
            contains_delayed_fields: false,
        };
        Self::new(layout, DefiningModules::new(), size_estimate_in_bytes)
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

    /// Returns the cache key for the given root type, or [None] if its layout is not cached.
    ///
    /// Only struct- and vector-headed types are cached, keyed by the interned id of the whole
    /// type. Keying on the whole type means an entry is only ever served for the exact type it was
    /// built from, so a hit stays equivalent to a miss in gas and in node/depth accounting.
    fn layout_cache_key(&self, ty: &Type) -> Option<LayoutCacheKey> {
        match ty {
            Type::Struct { .. } | Type::StructInstantiation { .. } | Type::Vector(_) => {},
            Type::Bool
            | Type::U8
            | Type::U16
            | Type::U32
            | Type::U64
            | Type::U128
            | Type::U256
            | Type::I8
            | Type::I16
            | Type::I32
            | Type::I64
            | Type::I128
            | Type::I256
            | Type::Address
            | Type::Signer
            | Type::Function { .. }
            | Type::Reference(_)
            | Type::MutableReference(_)
            | Type::TyParam(_) => return None,
        }

        // Types here are fully instantiated, so there is nothing to substitute. Interning panics
        // if that ever does not hold, as it already did when keys were built from `intern_ty_args`.
        let id = self
            .runtime_environment()
            .ty_pool()
            .instantiate_and_intern(ty, &[]);
        Some(LayoutCacheKey { id })
    }

    /// Returns the layout of a type, as well as a flag if it contains delayed fields or not.
    pub(crate) fn type_to_type_layout_with_delayed_fields(
        &self,
        gas_meter: &mut impl DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        ty: &Type,
    ) -> PartialVMResult<LayoutWithDelayedFields> {
        Ok(self
            .type_to_type_layout_with_delayed_fields_and_cached_size(
                gas_meter,
                traversal_context,
                ty,
            )?
            .0)
    }

    /// Like [Self::type_to_type_layout_with_delayed_fields], but also returns the number of bytes
    /// the layout serializes to. Cached layouts store their size, so a cache hit does not walk the
    /// layout.
    pub(crate) fn type_to_type_layout_with_delayed_fields_and_size(
        &self,
        gas_meter: &mut impl DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        ty: &Type,
    ) -> PartialVMResult<(LayoutWithDelayedFields, u64)> {
        let (layout, size) = self.type_to_type_layout_with_delayed_fields_and_cached_size(
            gas_meter,
            traversal_context,
            ty,
        )?;
        let size = size.unwrap_or_else(|| layout.serialized_size());
        Ok((layout, size))
    }

    /// Returns the layout of a type, and its serialized size if the layout is cached.
    fn type_to_type_layout_with_delayed_fields_and_cached_size(
        &self,
        gas_meter: &mut impl DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        ty: &Type,
    ) -> PartialVMResult<(LayoutWithDelayedFields, Option<u64>)> {
        if self.vm_config().enable_layout_caches {
            if let Some(key) = self.layout_cache_key(ty) {
                if let Some(result) = self.struct_definition_loader.load_layout_from_cache(
                    gas_meter,
                    traversal_context,
                    &key,
                ) {
                    return result.map(|(layout, size)| (layout, Some(size)));
                }

                // Otherwise a cache miss, compute the result and store it.
                let mut modules = DefiningModules::new();
                let (layout, size_estimate_in_bytes) = self
                    .type_to_type_layout_with_delayed_fields_impl::<false>(
                        gas_meter,
                        traversal_context,
                        &mut modules,
                        ty,
                    )?;
                // Do not cache layouts that are too large, to keep cache memory bounded.
                if size_estimate_in_bytes <= MAX_CACHED_LAYOUT_SIZE_ESTIMATE_IN_BYTES {
                    let cache_entry =
                        LayoutCacheEntry::new(layout.clone(), modules, size_estimate_in_bytes);
                    let size = cache_entry.serialized_size();
                    self.struct_definition_loader
                        .store_layout_to_cache(&key, cache_entry)?;
                    return Ok((layout, Some(size)));
                }
                return Ok((layout, None));
            }
        }

        let (layout, _) = self.type_to_type_layout_with_delayed_fields_impl::<false>(
            gas_meter,
            traversal_context,
            &mut DefiningModules::new(),
            ty,
        )?;
        Ok((layout, None))
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
        self.type_to_type_layout_with_delayed_fields_impl::<true>(
            gas_meter,
            traversal_context,
            &mut DefiningModules::new(),
            ty,
        )
        .map(|(layout, _)| layout)
    }

    /// Returns the VM config used in the system.
    fn vm_config(&self) -> &VMConfig {
        self.runtime_environment().vm_config()
    }

    /// Returns the runtime environment used in the system.
    pub(crate) fn runtime_environment(&self) -> &RuntimeEnvironment {
        self.struct_definition_loader.runtime_environment()
    }

    /// Returns the hash and size of the specified module, without charging gas.
    pub(crate) fn unmetered_get_module_hash_and_size(
        &self,
        module_id: &ModuleId,
    ) -> PartialVMResult<([u8; 32], usize)> {
        self.struct_definition_loader
            .unmetered_get_module_hash_and_size(module_id.address(), module_id.name())
            .map_err(|err| err.to_partial())
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
            struct_name.module(),
            struct_name.name(),
        ))
    }

    /// Since layout is a tree data structure, we limit its size and depth during construction.
    /// This function checks that the number of nodes in the layout and its depth are within limits
    /// enforced by the VM config. The count (and the size estimate) is incremented.
    fn check_depth_and_increment_count(
        &self,
        count: &mut LayoutConstructionCounter,
        depth: u64,
    ) -> PartialVMResult<()> {
        let max_count = self.vm_config().layout_max_size;
        let node_count = count.num_nodes;
        if node_count > max_count || node_count == max_count && self.is_lazy_loading_enabled() {
            return Err(
                PartialVMError::new(StatusCode::TOO_MANY_TYPE_NODES).with_message(format!(
                    "Number of type nodes when constructing type layout exceeded the maximum of {}",
                    max_count
                )),
            );
        }
        count.add_node();

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
    /// Returns the layout and the estimated number of bytes it (and its defining modules) occupy
    /// in memory when cached.
    fn type_to_type_layout_with_delayed_fields_impl<const ANNOTATED: bool>(
        &self,
        gas_meter: &mut impl DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        modules: &mut DefiningModules,
        ty: &Type,
    ) -> PartialVMResult<(LayoutWithDelayedFields, usize)> {
        let _timer = VM_TIMER.timer_with_label("type_to_type_layout_with_delayed_fields");

        let mut count = LayoutConstructionCounter::new();
        // Keep the cache scoped to this single traversal; do not hoist it
        // beyond this call (see invariant above).
        let mut struct_layout_cache = LocalSinglePassStructLayoutCache::new();
        let (layout, contains_delayed_fields) = self.type_to_type_layout_impl::<ANNOTATED>(
            gas_meter,
            traversal_context,
            modules,
            ty,
            &mut count,
            1,
            &mut struct_layout_cache,
        )?;
        let layout = LayoutWithDelayedFields {
            layout: TriompheArc::new(layout),
            contains_delayed_fields,
        };
        Ok((layout, count.size_estimate_in_bytes))
    }

    /// Converts [Type] to [MoveTypeLayout]. Fails if there is not enough gas to account for module
    /// loading, the layout is too deep / too large, or some other miscellaneous failures, e.g. if
    /// struct definition is not found.
    fn type_to_type_layout_impl<const ANNOTATED: bool>(
        &self,
        gas_meter: &mut impl DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        modules: &mut DefiningModules,
        ty: &Type,
        count: &mut LayoutConstructionCounter,
        depth: u64,
        struct_layout_cache: &mut LocalSinglePassStructLayoutCache,
    ) -> PartialVMResult<(MoveTypeLayout, bool)> {
        self.check_depth_and_increment_count(count, depth)?;

        Ok(match ty {
            Type::Bool => (MoveTypeLayout::Bool, false),
            Type::U8 => (MoveTypeLayout::U8, false),
            Type::U16 => (MoveTypeLayout::U16, false),
            Type::U32 => (MoveTypeLayout::U32, false),
            Type::U64 => (MoveTypeLayout::U64, false),
            Type::U128 => (MoveTypeLayout::U128, false),
            Type::U256 => (MoveTypeLayout::U256, false),
            Type::I8 => (MoveTypeLayout::I8, false),
            Type::I16 => (MoveTypeLayout::I16, false),
            Type::I32 => (MoveTypeLayout::I32, false),
            Type::I64 => (MoveTypeLayout::I64, false),
            Type::I128 => (MoveTypeLayout::I128, false),
            Type::I256 => (MoveTypeLayout::I256, false),
            Type::Address => (MoveTypeLayout::Address, false),
            Type::Signer => (MoveTypeLayout::Signer, false),
            Type::Function { .. } => (MoveTypeLayout::Function, false),
            Type::Vector(ty) => self
                .type_to_type_layout_impl::<ANNOTATED>(
                    gas_meter,
                    traversal_context,
                    modules,
                    ty,
                    count,
                    depth + 1,
                    struct_layout_cache,
                )
                .map(|(elem_layout, contains_delayed_fields)| {
                    let vec_layout = MoveTypeLayout::Vector(Box::new(elem_layout));
                    (vec_layout, contains_delayed_fields)
                })?,
            Type::Struct { idx, .. } => self.struct_to_type_layout::<ANNOTATED>(
                gas_meter,
                traversal_context,
                modules,
                idx,
                &[],
                count,
                depth + 1,
                struct_layout_cache,
            )?,
            Type::StructInstantiation(node) => self.struct_to_type_layout::<ANNOTATED>(
                gas_meter,
                traversal_context,
                modules,
                &node.idx(),
                node.ty_args(),
                count,
                depth + 1,
                struct_layout_cache,
            )?,
            Type::Reference(_) | Type::MutableReference(_) | Type::TyParam(_) => {
                return Err(
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .with_message(format!("No type layout for {:?}", ty)),
                );
            },
        })
    }

    /// Converts a sequence of types into a sequence of layouts. Does not increment depth, so the
    /// caller has to handle it.
    fn types_to_type_layouts<const ANNOTATED: bool>(
        &self,
        gas_meter: &mut impl DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        modules: &mut DefiningModules,
        tys: &[Type],
        count: &mut LayoutConstructionCounter,
        depth: u64,
        struct_layout_cache: &mut LocalSinglePassStructLayoutCache,
    ) -> PartialVMResult<(Vec<MoveTypeLayout>, bool)> {
        let mut contains_delayed_fields = false;
        let layouts = tys
            .iter()
            .map(|ty| {
                let (layout, ty_contains_delayed_fields) = self
                    .type_to_type_layout_impl::<ANNOTATED>(
                        gas_meter,
                        traversal_context,
                        modules,
                        ty,
                        count,
                        depth,
                        struct_layout_cache,
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
    //   We do not add struct cyclic checks here because it can be rather expensive to check. In
    //   general, because we have depth / count checks and charges for modules this will eventually
    //   terminate in any case. In the future, layouts should be revisited anyway.
    //   Consider adding proper charges here for layout construction (before rollout).
    fn struct_to_type_layout<const ANNOTATED: bool>(
        &self,
        gas_meter: &mut impl DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        modules: &mut DefiningModules,
        idx: &StructNameIndex,
        ty_args: &[Type],
        count: &mut LayoutConstructionCounter,
        depth: u64,
        struct_layout_cache: &mut LocalSinglePassStructLayoutCache,
    ) -> PartialVMResult<(MoveTypeLayout, bool)> {
        let use_local_cache = self.vm_config().enable_struct_layout_local_cache;
        // Check the per-construction-pass cache. On hit, return the shared Arc without
        // re-constructing or re-counting nodes. Uses a borrowed key to avoid allocation.
        let insert_into_cache = if use_local_cache {
            if let Some((cached_layout, contains_delayed_fields)) = struct_layout_cache
                .select::<ANNOTATED>()
                .get(&StructLayoutKeyRef { idx: *idx, ty_args })
            {
                return Ok((
                    MoveTypeLayout::Struct(cached_layout.clone()),
                    *contains_delayed_fields,
                ));
            }
            true
        } else {
            false
        };
        let struct_definition = self.struct_definition_loader.load_struct_definition(
            gas_meter,
            traversal_context,
            idx,
        )?;
        let struct_identifier = self
            .struct_definition_loader
            .runtime_environment()
            .struct_name_index_map()
            .idx_to_struct_name_ref(*idx)?;

        // METERING SAFETY:
        //   Module has already been loaded because we got the struct definition.
        if !modules.contains(struct_identifier.module()) {
            let (hash, _) = self
                .struct_definition_loader
                .unmetered_get_module_hash_and_size(
                    struct_identifier.module().address(),
                    struct_identifier.module().name(),
                )
                .map_err(|err| err.to_partial())?;
            modules.insert(struct_identifier.module(), hash);
            count.add_defining_module(struct_identifier.module());
        }

        let result = match &struct_definition.layout {
            // For enums, construct layouts for all possible variants. No special handling for
            // delayed fields is needed because enums cannot be delayed fields!
            StructLayout::Variants(variants) => {
                let mut variant_contains_delayed_fields = false;
                let enum_option_enabled = self.runtime_environment().vm_config().enable_enum_option;
                // convert enum representation of option for backward compatibility
                if enum_option_enabled && ANNOTATED {
                    let ty_tag_converter =
                        TypeTagConverter::new(self.struct_definition_loader.runtime_environment());
                    let struct_tag =
                        ty_tag_converter.struct_name_idx_to_struct_tag(idx, ty_args)?;
                    if struct_tag.is_option() {
                        let field_name = ident_str!(LEGACY_OPTION_VEC).to_owned();
                        let ty_builder = &self.vm_config().ty_builder;
                        if variants.len() < 2 || variants[1].1.is_empty() {
                            return Err(PartialVMError::new(
                                StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                            ));
                        }
                        let field_type =
                            ty_builder.create_ty_with_subst(&variants[1].1[0].1, ty_args)?;
                        let (mut field_layout, delayed_fields) = self
                            .types_to_type_layouts::<ANNOTATED>(
                                gas_meter,
                                traversal_context,
                                modules,
                                &[field_type],
                                count,
                                depth + 1,
                                struct_layout_cache,
                            )?;
                        variant_contains_delayed_fields |= delayed_fields;
                        if field_layout.is_empty() {
                            return Err(PartialVMError::new(
                                StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                            ));
                        }
                        let field_layout = MoveFieldLayout::new(
                            field_name,
                            MoveTypeLayout::Vector(Box::new(field_layout.pop().unwrap())),
                        );
                        let struct_layout =
                            MoveStructLayout::with_types(struct_tag, vec![field_layout]);
                        return Ok((
                            MoveTypeLayout::new_struct(struct_layout),
                            variant_contains_delayed_fields,
                        ));
                    }
                }
                count.add_variants(variants.len());
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
                                modules,
                                &self.apply_subst_for_field_tys(&variant.1, ty_args)?,
                                count,
                                depth,
                                struct_layout_cache,
                            )?;
                        variant_contains_delayed_fields |= variant_fields_contain_delayed_fields;
                        Ok(variant_field_layouts)
                    })
                    .collect::<PartialVMResult<Vec<_>>>()?;

                // TODO(#13806):
                //   Have annotated layouts for variants. Currently, we just return the raw layout
                //   for them.
                let variant_layout =
                    MoveTypeLayout::new_struct(MoveStructLayout::RuntimeVariants(variant_layouts));
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
                        modules,
                        &self.apply_subst_for_field_tys(fields, ty_args)?,
                        count,
                        depth,
                        struct_layout_cache,
                    )?;

                match (kind, fields_contain_delayed_fields) {
                    (None, fields_contain_delayed_fields) => {
                        let struct_layout = MoveTypeLayout::new_struct(
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
                            struct_name.module().address,
                            struct_name.module().name,
                            struct_name.name(),
                        );
                        return Err(PartialVMError::new_invariant_violation(msg));
                    },
                    (Some(kind), false) => {
                        // Note 1:
                        //   For delayed fields, simply never output annotated layout. The
                        //   callers should not be able to handle it in any case.
                        //
                        // Note 2:
                        //   For derived strings, aggregators or snapshots, do not use the
                        //   cache to reduce bounds on count/depth by resolving by struct
                        //   identity (see below). All these layouts are small (few nodes),
                        //   the cost of reconstructing them on cache miss is negligible,
                        //   and it is not worth additional feature gating.
                        use IdentifierMappingKind::*;
                        let layout = match &kind {
                            DerivedString => {
                                let inner_layout = MoveTypeLayout::new_struct(
                                    MoveStructLayout::new(field_layouts),
                                );
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
                                    MoveTypeLayout::new_struct(MoveStructLayout::new(field_layouts))
                                },
                                None => {
                                    let struct_name = self.get_struct_name(idx)?;
                                    let msg = format!(
                                        "Struct {}::{}::{} must contain at least one field",
                                        struct_name.module().address,
                                        struct_name.module().name,
                                        struct_name.name(),
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

        // Cache the constructed struct layout for reuse within this construction pass.
        // The owned key (with ty_args.to_vec()) is only constructed here on cache miss.
        if insert_into_cache {
            if let MoveTypeLayout::Struct(arc_layout) = &result.0 {
                struct_layout_cache.select::<ANNOTATED>().insert(
                    StructLayoutKey {
                        idx: *idx,
                        ty_args: ty_args.to_vec(),
                    },
                    (arc_layout.clone(), result.1),
                );
            }
        }

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

/// Cache keyed by `Arc` pointer identity for [`constant_serialized_size`] deduplication.
/// Stores BCS-size for each unique `Arc<MoveStructLayout>` already visited.
///
/// # Safety of raw-pointer keys
/// The pointer is obtained via `Arc::as_ptr` and is **never dereferenced** through the
/// map key - it is used solely as a unique identity token. Because the caller holds an
/// `Arc` to the layout tree throughout the entire traversal, the allocation behind each
/// pointer remains valid and will not be moved or freed, so pointer identity is stable
/// for the duration of the call. `HashMap` requires `Send`-safe keys; raw pointers are
/// `!Send`, so the cache must not be sent across threads - and it is not: it is local
/// and consumed inside a single call to `constant_serialized_size`.
type LayoutSizeCache = FxHashMap<*const MoveStructLayout, Option<usize>>;

/// Computes the constant BCS serialized size of a type layout, if the type has one.
///
/// Returns `(visited_node_count, Ok(Some(size)))` when every value of this type serializes
/// to the same number of bytes, `(visited_node_count, Ok(None))` when the size depends on
/// the value (e.g. vectors, enum variants), or `(visited_node_count, Err(...))` on an
/// unexpected layout form.
///
/// Uses an `Arc` pointer-identity cache to deduplicate struct subtrees, mirroring the
/// behavior of [`LocalSinglePassStructLayoutCache`] during layout construction. This bounds
/// total work to O(DAG size).
pub fn constant_serialized_size(
    ty_layout: &MoveTypeLayout,
    use_local_struct_cache: bool,
) -> (u64, PartialVMResult<Option<usize>>) {
    let mut cache = LayoutSizeCache::default();
    constant_serialized_size_impl(ty_layout, &mut cache, use_local_struct_cache)
}

fn constant_serialized_size_impl(
    ty_layout: &MoveTypeLayout,
    cache: &mut LayoutSizeCache,
    use_local_struct_cache: bool,
) -> (u64, PartialVMResult<Option<usize>>) {
    let mut visited_count = 1u64;
    let bcs_size_result = match ty_layout {
        // Primitive types always have constant size.
        MoveTypeLayout::Bool => bcs::serialized_size(&false).map(Some),
        MoveTypeLayout::U8 => bcs::serialized_size(&0u8).map(Some),
        MoveTypeLayout::U16 => bcs::serialized_size(&0u16).map(Some),
        MoveTypeLayout::U32 => bcs::serialized_size(&0u32).map(Some),
        MoveTypeLayout::U64 => bcs::serialized_size(&0u64).map(Some),
        MoveTypeLayout::U128 => bcs::serialized_size(&0u128).map(Some),
        MoveTypeLayout::U256 => bcs::serialized_size(&int256::U256::ZERO).map(Some),
        MoveTypeLayout::I8 => bcs::serialized_size(&0i8).map(Some),
        MoveTypeLayout::I16 => bcs::serialized_size(&0i16).map(Some),
        MoveTypeLayout::I32 => bcs::serialized_size(&0i32).map(Some),
        MoveTypeLayout::I64 => bcs::serialized_size(&0i64).map(Some),
        MoveTypeLayout::I128 => bcs::serialized_size(&0i128).map(Some),
        MoveTypeLayout::I256 => bcs::serialized_size(&int256::I256::ZERO).map(Some),
        MoveTypeLayout::Address => bcs::serialized_size(&AccountAddress::ZERO).map(Some),

        // No constant size:
        //   - signer's size is VM implementation detail, and can change,
        //   - vectors have no constant size,
        //   - functions have no constant size.
        MoveTypeLayout::Signer | MoveTypeLayout::Vector(_) | MoveTypeLayout::Function => {
            return (visited_count, Ok(None))
        },

        MoveTypeLayout::Struct(arc) => {
            let ptr = Arc::as_ptr(arc);
            if use_local_struct_cache {
                if let Some(&cached) = cache.get(&ptr) {
                    // Repeated struct references during layout construction increment the node
                    // counter by 1, not by all descendants.
                    return (1, Ok(cached));
                }
            }

            // Structs have constant size, but not enums.
            let size = match arc.as_ref() {
                MoveStructLayout::RuntimeVariants(_) | MoveStructLayout::WithVariants { .. } => {
                    None
                },
                MoveStructLayout::Runtime(fields) => {
                    let mut total: Option<usize> = Some(0);
                    for field in fields {
                        let (cur_count, cur) =
                            constant_serialized_size_impl(field, cache, use_local_struct_cache);
                        visited_count = visited_count.saturating_add(cur_count);
                        match cur {
                            Err(e) => return (visited_count, Err(e)),
                            Ok(Some(v)) => total = total.and_then(|s| s.checked_add(v)),
                            Ok(None) => {
                                total = None;
                                break;
                            },
                        }
                    }
                    total
                },
                MoveStructLayout::WithFields(_) | MoveStructLayout::WithTypes { .. } => {
                    return (
                        visited_count,
                        Err(PartialVMError::new(StatusCode::VALUE_SERIALIZATION_ERROR)
                            .with_message(
                                "Only runtime types expected, but found WithFields/WithTypes"
                                    .to_string(),
                            )),
                    );
                },
            };

            if use_local_struct_cache {
                cache.insert(ptr, size);
            }
            return (visited_count, Ok(size));
        },
        MoveTypeLayout::Native(_, inner) => {
            let (cur_count, cur) =
                constant_serialized_size_impl(inner, cache, use_local_struct_cache);
            visited_count = visited_count.saturating_add(cur_count);
            match cur {
                Err(e) => return (visited_count, Err(e)),
                Ok(v) => Ok(v),
            }
        },
    };
    (
        visited_count,
        bcs_size_result.map_err(|e| {
            PartialVMError::new(StatusCode::VALUE_SERIALIZATION_ERROR).with_message(format!(
                "failed to compute serialized size of a value: {:?}",
                e
            ))
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        module_traversal::{TraversalContext, TraversalStorage},
        storage::loader::test_utils::{generic_struct_ty, struct_ty, MockStructDefinitionLoader},
    };
    use claims::{assert_err, assert_none, assert_ok, assert_some};
    use move_core_types::{
        ability::AbilitySet, account_address::AccountAddress, language_storage::StructTag,
    };
    use move_vm_types::gas::UnmeteredGasMeter;
    use std::str::FromStr;
    use test_case::test_case;

    /// Constructs layout (without any metering) for the given type.
    fn construct_layout_for_test(
        layout_converter: &LayoutConverter<MockStructDefinitionLoader>,
        ty: &Type,
        annotated: bool,
    ) -> PartialVMResult<LayoutWithDelayedFields> {
        let mut gas_meter = UnmeteredGasMeter;
        let traversal_storage = TraversalStorage::new();
        let mut traversal_context = TraversalContext::new(&traversal_storage);

        if annotated {
            layout_converter.type_to_annotated_type_layout_with_delayed_fields(
                &mut gas_meter,
                &mut traversal_context,
                ty,
            )
        } else {
            layout_converter.type_to_type_layout_with_delayed_fields(
                &mut gas_meter,
                &mut traversal_context,
                ty,
            )
        }
    }

    /// Wraps `ty` in `n` nested vectors.
    fn vec_ty(ty: Type, n: usize) -> Type {
        (0..n).fold(ty, |ty, _| Type::vector(ty))
    }

    /// Asserts that every key in `keys` is distinct from every other.
    fn assert_all_distinct(keys: &[LayoutCacheKey]) {
        for (i, key) in keys.iter().enumerate() {
            for other in &keys[i + 1..] {
                assert_ne!(key, other);
            }
        }
    }

    #[test]
    fn test_layout_cache_key_for_vector_wrapped_structs() {
        let loader = MockStructDefinitionLoader::default();
        let layout_converter = LayoutConverter::new(&loader);
        let idx = StructNameIndex::new(0);

        // Each nesting level keys to its own entry, else `vector<S>` gets the layout of `S`.
        let by_nesting = (0..4)
            .map(|n| assert_some!(layout_converter.layout_cache_key(&vec_ty(struct_ty(idx), n))))
            .collect::<Vec<_>>();
        assert_all_distinct(&by_nesting);

        // The same type always keys to the same entry.
        for (n, key) in by_nesting.iter().enumerate() {
            assert_eq!(
                assert_some!(layout_converter.layout_cache_key(&vec_ty(struct_ty(idx), n))),
                *key
            );
        }

        // Generic structs behind vectors stay keyed by their type arguments.
        let by_ty_args = [Type::U8, Type::Bool, Type::U64]
            .map(|ty_arg| vec_ty(generic_struct_ty(idx, vec![ty_arg]), 1))
            .map(|ty| assert_some!(layout_converter.layout_cache_key(&ty)));
        assert_all_distinct(&by_ty_args);

        // Different structs never collide, at any nesting level.
        let other_idx = StructNameIndex::new(1);
        for n in 0..4 {
            assert_ne!(
                assert_some!(layout_converter.layout_cache_key(&vec_ty(struct_ty(idx), n))),
                assert_some!(layout_converter.layout_cache_key(&vec_ty(struct_ty(other_idx), n)))
            );
        }
    }

    #[test]
    fn test_layout_cache_key_for_uncacheable_tys() {
        let loader = MockStructDefinitionLoader::default();
        let layout_converter = LayoutConverter::new(&loader);

        // Primitives and functions have constant-size layouts; references and type parameters have
        // no layout at all. Neither is worth an entry.
        let uncacheable = [
            Type::Bool,
            Type::U8,
            Type::U16,
            Type::U32,
            Type::U64,
            Type::U128,
            Type::U256,
            Type::I8,
            Type::I16,
            Type::I32,
            Type::I64,
            Type::I128,
            Type::I256,
            Type::Address,
            Type::Signer,
            Type::function(vec![Type::U8], vec![Type::U256], AbilitySet::EMPTY),
            Type::reference(Type::U8),
            Type::mut_reference(Type::U8),
            Type::TyParam(0),
        ];
        for ty in &uncacheable {
            assert_none!(layout_converter.layout_cache_key(ty));
        }

        // Selection is on the head only, so wrapping any of them in a vector does make it
        // cacheable. Those layouts are tiny, but carving out an exception would not pay for itself.
        let vector_wrapped = [Type::U8, Type::Bool, Type::Address]
            .map(|ty| assert_some!(layout_converter.layout_cache_key(&vec_ty(ty, 1))));
        assert_all_distinct(&vector_wrapped);
    }

    #[test_case(true)]
    #[test_case(false)]
    fn test_layout_with_delayed_fields(contains_delayed_fields: bool) {
        let layout = LayoutWithDelayedFields {
            // Dummy layout.
            layout: TriompheArc::new(MoveTypeLayout::U8),
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

    /// Returns the size estimate of the layout constructed for the given type.
    fn size_estimate_for_test(
        layout_converter: &LayoutConverter<MockStructDefinitionLoader>,
        ty: &Type,
    ) -> usize {
        let mut gas_meter = UnmeteredGasMeter;
        let traversal_storage = TraversalStorage::new();
        let mut traversal_context = TraversalContext::new(&traversal_storage);
        let result = layout_converter.type_to_type_layout_with_delayed_fields_impl::<false>(
            &mut gas_meter,
            &mut traversal_context,
            &mut DefiningModules::new(),
            ty,
        );
        let (_, size_estimate_in_bytes) = assert_ok!(result);
        size_estimate_in_bytes
    }

    fn module_size_estimate(name: &str) -> usize {
        let module_id = ModuleId::from_str(&format!("0x1::{}", name)).unwrap();
        defining_module_size_estimate_in_bytes(&module_id)
    }

    #[test]
    fn test_layout_size_estimate_accumulates_nodes() {
        let loader = MockStructDefinitionLoader::default();
        let layout_converter = LayoutConverter::new(&loader);

        let base = BASE_SIZE_ESTIMATE_IN_BYTES;
        let node = PER_NODE_SIZE_ESTIMATE_IN_BYTES;

        assert_eq!(
            size_estimate_for_test(&layout_converter, &Type::U8),
            base + node
        );
        for n in [1, 5, 20] {
            assert_eq!(
                size_estimate_for_test(&layout_converter, &vec_ty(Type::U64, n)),
                base + (n + 1) * node
            );
        }

        // Struct with 10 primitive fields: 1 node for the struct, 10 for the fields, plus the
        // defining module.
        let a = loader.get_struct_identifier("A");
        loader.add_struct("A", (0..10).map(|_| ("x", Type::U8)).collect::<Vec<_>>());
        assert_eq!(
            size_estimate_for_test(&layout_converter, &struct_ty(a)),
            base + 11 * node + module_size_estimate("foo")
        );

        // A struct with fields of the same struct type: the first field constructs the layout of
        // A, while other fields reuse it from the local cache and only account for a single node.
        // The defining module is accounted for only once.
        let b = loader.get_struct_identifier("B");
        loader.add_struct("B", (0..5).map(|_| ("a", struct_ty(a))).collect::<Vec<_>>());
        assert_eq!(
            size_estimate_for_test(&layout_converter, &struct_ty(b)),
            base + (1 + 11 + 4) * node + module_size_estimate("foo")
        );
    }

    #[test]
    fn test_layout_size_estimate_accumulates_variants() {
        let loader = MockStructDefinitionLoader::default();
        let layout_converter = LayoutConverter::new(&loader);

        let base = BASE_SIZE_ESTIMATE_IN_BYTES;
        let node = PER_NODE_SIZE_ESTIMATE_IN_BYTES;
        let variant = PER_VARIANT_SIZE_ESTIMATE_IN_BYTES;
        let module = module_size_estimate("foo");

        // Empty variants add no nodes, but are still accounted for.
        let num_variants = 50;
        let names = (0..num_variants)
            .map(|i| format!("V{}", i))
            .collect::<Vec<_>>();
        let e = loader.get_struct_identifier("E");
        loader.add_enum("E", names.iter().map(|name| (name.as_str(), vec![])));
        assert_eq!(
            size_estimate_for_test(&layout_converter, &struct_ty(e)),
            base + node + module + num_variants * variant
        );

        // Variant fields are accounted for as nodes on top of the variants.
        let f = loader.get_struct_identifier("F");
        loader.add_enum("F", vec![
            ("V0", vec![]),
            ("V1", vec![("x", Type::U8), ("y", Type::U64)]),
        ]);
        assert_eq!(
            size_estimate_for_test(&layout_converter, &struct_ty(f)),
            base + 3 * node + module + 2 * variant
        );

        // Fields of the same enum type reuse the layout from the local cache, so variants are
        // accounted for only once.
        let s = loader.get_struct_identifier("S");
        loader.add_struct("S", (0..5).map(|_| ("e", struct_ty(e))).collect::<Vec<_>>());
        assert_eq!(
            size_estimate_for_test(&layout_converter, &struct_ty(s)),
            base + (1 + 5) * node + module + num_variants * variant
        );
    }

    #[test]
    fn test_layout_size_estimate_accumulates_modules() {
        let loader = MockStructDefinitionLoader::default();
        let layout_converter = LayoutConverter::new(&loader);

        let base = BASE_SIZE_ESTIMATE_IN_BYTES;
        let node = PER_NODE_SIZE_ESTIMATE_IN_BYTES;

        // Same struct defined in modules with short and long names: only the module part of the
        // estimate differs.
        let long_name = "a".repeat(200);
        let short_module = "0x1::m";
        let long_module = format!("0x1::{}", long_name);

        let s_short = loader.get_struct_identifier_in_module(short_module, "S");
        loader.add_struct_in_module(short_module, "S", vec![("x", Type::U8)]);
        let s_long = loader.get_struct_identifier_in_module(&long_module, "S");
        loader.add_struct_in_module(&long_module, "S", vec![("x", Type::U8)]);

        let short_estimate = size_estimate_for_test(&layout_converter, &struct_ty(s_short));
        let long_estimate = size_estimate_for_test(&layout_converter, &struct_ty(s_long));
        assert_eq!(short_estimate, base + 2 * node + module_size_estimate("m"));
        assert_eq!(
            long_estimate,
            base + 2 * node + module_size_estimate(&long_name)
        );
        assert_eq!(long_estimate - short_estimate, 2 * (long_name.len() - 1));

        // Struct with fields from many different modules accounts for every defining module.
        let num_modules = 20;
        let module_names = (0..num_modules)
            .map(|i| format!("m{}_{}", i, long_name))
            .collect::<Vec<_>>();
        let fields = module_names
            .iter()
            .map(|name| {
                let module = format!("0x1::{}", name);
                let idx = loader.get_struct_identifier_in_module(&module, "L");
                loader.add_struct_in_module(&module, "L", vec![("x", Type::U8)]);
                ("l", struct_ty(idx))
            })
            .collect::<Vec<_>>();
        let big = loader.get_struct_identifier_in_module("0x1::big", "Big");
        loader.add_struct_in_module("0x1::big", "Big", fields);

        let expected_modules = module_size_estimate("big")
            + module_names
                .iter()
                .map(|name| module_size_estimate(name))
                .sum::<usize>();
        assert_eq!(
            size_estimate_for_test(&layout_converter, &struct_ty(big)),
            base + (1 + 2 * num_modules) * node + expected_modules
        );
    }

    #[test_case(true)]
    #[test_case(false)]
    fn test_simple_tys_to_layouts(annotated: bool) {
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
                Type::vector(Type::U8),
                MoveTypeLayout::Vector(Box::new(MoveTypeLayout::U8)),
            ),
            (
                Type::function(vec![Type::U8], vec![Type::U256], AbilitySet::EMPTY),
                MoveTypeLayout::Function,
            ),
        ];

        for (ty, expected_layout) in test_cases {
            let result = construct_layout_for_test(&layout_converter, &ty, annotated);
            let layout = assert_ok!(result);

            assert!(!layout.contains_delayed_fields);
            assert_eq!(layout.layout.as_ref(), &expected_layout);
        }
    }

    #[test_case(true, true)]
    #[test_case(true, false)]
    #[test_case(false, true)]
    #[test_case(false, false)]
    fn test_layout_too_large(enable_lazy_loading: bool, annotated: bool) {
        let vm_config = VMConfig {
            enable_lazy_loading,
            layout_max_size: 2,
            ..VMConfig::default_for_test()
        };
        let loader = MockStructDefinitionLoader::new_with_config(vm_config);
        let layout_converter = LayoutConverter::new(&loader);

        let vec_u8_ty = Type::vector(Type::U8);
        assert_ok!(construct_layout_for_test(
            &layout_converter,
            &vec_u8_ty,
            annotated
        ));

        let vec_vec_u8_ty = Type::vector(vec_u8_ty);
        let result = construct_layout_for_test(&layout_converter, &vec_vec_u8_ty, annotated);
        if enable_lazy_loading {
            let err = assert_err!(result);
            assert_eq!(err.major_status(), StatusCode::TOO_MANY_TYPE_NODES);
        } else {
            assert_ok!(result);
        }

        let vec_vec_vec_u8_ty = Type::vector(vec_vec_u8_ty);
        let err = assert_err!(construct_layout_for_test(
            &layout_converter,
            &vec_vec_vec_u8_ty,
            annotated
        ));
        assert_eq!(err.major_status(), StatusCode::TOO_MANY_TYPE_NODES);
    }

    #[test_case(true)]
    #[test_case(false)]
    fn test_layout_too_deep(annotated: bool) {
        let vm_config = VMConfig {
            layout_max_depth: 2,
            ..VMConfig::default_for_test()
        };
        let loader = MockStructDefinitionLoader::new_with_config(vm_config);
        let layout_converter = LayoutConverter::new(&loader);

        let vec_u8_ty = Type::vector(Type::U8);
        assert_ok!(construct_layout_for_test(
            &layout_converter,
            &vec_u8_ty,
            annotated
        ));

        let vec_vec_u8_ty = Type::vector(vec_u8_ty);
        let err = assert_err!(construct_layout_for_test(
            &layout_converter,
            &vec_vec_u8_ty,
            annotated
        ));
        assert_eq!(err.major_status(), StatusCode::VM_MAX_VALUE_DEPTH_REACHED);
    }

    #[test]
    fn test_struct_layouts() {
        let loader = MockStructDefinitionLoader::default();

        let a = loader.get_struct_identifier("A");
        let b = loader.get_struct_identifier("B");
        let c = loader.get_struct_identifier("C");
        loader.add_struct("A", vec![]);
        loader.add_struct("B", vec![("c", struct_ty(c))]);
        loader.add_struct("C", vec![]);

        let layout_converter = LayoutConverter::new(&loader);

        let runtime_layout = |fields| MoveTypeLayout::new_struct(MoveStructLayout::Runtime(fields));
        let annotated_layout = |name: &str, fields| {
            MoveTypeLayout::new_struct(MoveStructLayout::with_types(
                StructTag {
                    address: AccountAddress::ONE,
                    module: Identifier::from_str("foo").unwrap(),
                    name: Identifier::from_str(name).unwrap(),
                    type_args: vec![],
                },
                fields,
            ))
        };

        for (i, name) in [(a, "A"), (c, "C")] {
            let layout = assert_ok!(construct_layout_for_test(
                &layout_converter,
                &struct_ty(i),
                false
            ));
            let layout = assert_some!(layout.into_layout_when_has_no_delayed_fields());
            assert_eq!(layout.as_ref(), &runtime_layout(vec![]));

            let layout = assert_ok!(construct_layout_for_test(
                &layout_converter,
                &struct_ty(i),
                true
            ));
            let layout = assert_some!(layout.into_layout_when_has_no_delayed_fields());
            assert_eq!(layout.as_ref(), &annotated_layout(name, vec![]));
        }

        let layout = assert_ok!(construct_layout_for_test(
            &layout_converter,
            &struct_ty(b),
            false
        ));
        let layout = assert_some!(layout.into_layout_when_has_no_delayed_fields());
        assert_eq!(
            layout.as_ref(),
            &runtime_layout(vec![runtime_layout(vec![])])
        );

        let layout = assert_ok!(construct_layout_for_test(
            &layout_converter,
            &struct_ty(b),
            true
        ));
        let layout = assert_some!(layout.into_layout_when_has_no_delayed_fields());
        assert_eq!(
            layout.as_ref(),
            &annotated_layout("B", vec![MoveFieldLayout::new(
                Identifier::from_str("c").unwrap(),
                annotated_layout("C", vec![])
            )])
        );
    }

    #[test_case(true)]
    #[test_case(false)]
    fn test_runtime_cyclic_module_dependency(annotated: bool) {
        let loader = MockStructDefinitionLoader::default();

        // No cycles (structs and enums):
        //
        // struct A {}
        // enum B {
        //   V1 { x: bool, },
        //   V2 { a: A, },
        // }
        let a = loader.get_struct_identifier("A");
        let b = loader.get_struct_identifier("B");
        loader.add_struct("A", vec![]);
        loader.add_enum("B", vec![
            ("V1", vec![("x", Type::Bool)]),
            ("V2", vec![("a", struct_ty(a))]),
        ]);

        // Cycles between structs C and D, and between E and itself:
        //
        // struct C { d: D, }
        // struct D { c: C, }
        // struct E { e: E, }
        let c = loader.get_struct_identifier("C");
        let d = loader.get_struct_identifier("D");
        let e = loader.get_struct_identifier("E");
        loader.add_struct("C", vec![("d", struct_ty(d))]);
        loader.add_struct("D", vec![("c", struct_ty(c))]);
        loader.add_struct("E", vec![("e", struct_ty(e))]);

        // Cycles between enums F and G.
        //
        // enum F {
        //   V0 { g: G, },
        // }
        // enum G {
        //   V0 { fs: vector<F>, },
        // }
        let f = loader.get_struct_identifier("F");
        let g = loader.get_struct_identifier("G");
        loader.add_enum("F", vec![("V0", vec![("g", struct_ty(g))])]);
        loader.add_enum("G", vec![("V0", vec![("fs", Type::vector(struct_ty(f)))])]);

        let layout_converter = LayoutConverter::new(&loader);
        for idx in [a, b] {
            assert_ok!(construct_layout_for_test(
                &layout_converter,
                &struct_ty(idx),
                annotated
            ));
        }

        for idx in [c, d, e, f, g] {
            let err = assert_err!(construct_layout_for_test(
                &layout_converter,
                &struct_ty(idx),
                annotated
            ));
            assert_eq!(err.major_status(), StatusCode::VM_MAX_VALUE_DEPTH_REACHED);
        }
    }

    #[test_case(true)]
    #[test_case(false)]
    fn test_nested_generic_no_runtime_cyclic_module_dependency(annotated: bool) {
        let loader = MockStructDefinitionLoader::default();

        // struct A<T> { x: T }
        let a = loader.get_struct_identifier("A");
        loader.add_struct("A", vec![("x", Type::TyParam(0))]);

        // struct B<T> { x: T }
        let b = loader.get_struct_identifier("B");
        loader.add_struct("B", vec![("x", Type::TyParam(0))]);

        // struct C {
        //   x: B<u8>,
        // }
        let c = loader.get_struct_identifier("C");
        loader.add_struct("C", vec![("x", generic_struct_ty(b, vec![Type::U8]))]);

        // Type for A<A<A<u8>>> - should not have cycles!
        let a_u8 = generic_struct_ty(a, vec![Type::U8]);
        let a_a_u8 = generic_struct_ty(a, vec![a_u8]);
        let a_a_a_u8 = generic_struct_ty(a, vec![a_a_u8]);

        // Type for B<A<B<u8>>> - should not have cycles!
        let b_u8 = generic_struct_ty(b, vec![Type::U8]);
        let a_b_u8 = generic_struct_ty(a, vec![b_u8]);
        let b_a_b_u8 = generic_struct_ty(b, vec![a_b_u8]);

        // B<C> does not create cycles because instantiations are different: C contains B<u8>.
        let b_c = generic_struct_ty(b, vec![struct_ty(c)]);

        for ty in [a_a_a_u8, b_a_b_u8, b_c] {
            let layout_converter = LayoutConverter::new(&loader);
            assert_ok!(construct_layout_for_test(&layout_converter, &ty, annotated));
        }
    }

    #[test_case(true)]
    #[test_case(false)]
    fn test_runtime_cyclic_module_dependency_generic(annotated: bool) {
        let loader = MockStructDefinitionLoader::default();

        // Cycle between generic struct B and C:
        //
        // struct B<T> { c: C<T> }
        // struct C<T> { x: T, b: B<T> }
        let b = loader.get_struct_identifier("B");
        let c = loader.get_struct_identifier("C");
        loader.add_struct("B", vec![(
            "c",
            generic_struct_ty(c, vec![Type::TyParam(0)]),
        )]);
        loader.add_struct("C", vec![
            ("x", Type::TyParam(0)),
            ("b", generic_struct_ty(b, vec![Type::TyParam(0)])),
        ]);

        // Cycle between generic enum and generic struct:
        //
        // struct D<T> { x: T, e: E<u8>, }
        // enum E<T> {
        //   V0 { ds: vector<D<u8>>, },
        // }
        let d = loader.get_struct_identifier("D");
        let e = loader.get_struct_identifier("E");
        loader.add_struct("D", vec![
            ("x", Type::TyParam(0)),
            ("e", generic_struct_ty(e, vec![Type::U8])),
        ]);
        loader.add_enum("E", vec![("V0", vec![(
            "ds",
            Type::vector(generic_struct_ty(d, vec![Type::U8])),
        )])]);

        let layout_converter = LayoutConverter::new(&loader);

        for idx in [b, c, d, e] {
            let err = assert_err!(construct_layout_for_test(
                &layout_converter,
                &generic_struct_ty(idx, vec![Type::Bool]),
                annotated
            ));
            assert_eq!(err.major_status(), StatusCode::VM_MAX_VALUE_DEPTH_REACHED);
        }
    }
}
