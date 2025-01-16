// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::{language_storage::StructTag, value::MoveTypeLayout, vm_status::StatusCode};
use move_vm_types::loaded_data::{
    runtime_types::{DepthFormula, Type},
    struct_name_indexing::{StructNameIndex, StructNameIndexMap},
};
use parking_lot::RwLock;

/// Layout information of a single struct instantiation.
#[derive(Clone)]
struct StructLayoutInfo {
    /// Layout of this struct instantiation.
    struct_layout: MoveTypeLayout,
    /// Number of nodes in the type layout.
    node_count: u64,
    /// True if this struct contains delayed fields, e.g., aggregators.
    has_identifier_mappings: bool,
}

impl StructLayoutInfo {
    fn unpack(self) -> (MoveTypeLayout, u64, bool) {
        (
            self.struct_layout,
            self.node_count,
            self.has_identifier_mappings,
        )
    }
}

/// Struct instantiation information included in [StructInfo].
#[derive(Clone)]
struct StructInstantiationInfo {
    /// Struct tag of this struct instantiation, and its pseudo-gas cost.
    struct_tag: Option<(StructTag, u64)>,
    /// Runtime struct layout information.
    struct_layout_info: Option<StructLayoutInfo>,
    /// Annotated struct layout information: layout with the node count.
    annotated_struct_layout_info: Option<(MoveTypeLayout, u64)>,
}

impl StructInstantiationInfo {
    fn none() -> Self {
        Self {
            struct_tag: None,
            struct_layout_info: None,
            annotated_struct_layout_info: None,
        }
    }
}

/// Cached information for any struct type. Caches information about its instantiations as well if
/// the struct is generic.
#[derive(Clone)]
struct StructInfo {
    /// Depth formula of a possibly generic struct, together with a thread id that cached it. If
    /// the formula is not yet cached, [None] is stored.
    depth_formula: Option<(DepthFormula, std::thread::ThreadId)>,
    /// Cached information for different struct instantiations.
    instantiation_info: hashbrown::HashMap<Vec<Type>, StructInstantiationInfo>,
}

impl StructInfo {
    /// Returns an empty struct information.
    pub(crate) fn none() -> Self {
        Self {
            depth_formula: None,
            instantiation_info: hashbrown::HashMap::new(),
        }
    }
}

/// A thread-safe struct information cache that can be used by the VM to store information about
/// structs, such as their depth formulae, tags, layouts.
pub(crate) struct StructInfoCache(RwLock<hashbrown::HashMap<StructNameIndex, StructInfo>>);

impl StructInfoCache {
    /// Returns an empty struct information cache.
    pub(crate) fn empty() -> Self {
        Self(RwLock::new(hashbrown::HashMap::new()))
    }

    /// Flushes the cached struct information.
    pub(crate) fn flush(&self) {
        self.0.write().clear()
    }

    /// Returns the depth formula associated with a struct, or [None] if it has not been cached.
    pub(crate) fn get_depth_formula(&self, idx: &StructNameIndex) -> Option<DepthFormula> {
        Some(self.0.read().get(idx)?.depth_formula.as_ref()?.0.clone())
    }

    /// Caches the depth formula, returning an error if the same thread has cached it before (i.e.,
    /// a recursive type has been found).
    pub(crate) fn store_depth_formula(
        &self,
        idx: StructNameIndex,
        struct_name_index_map: &StructNameIndexMap,
        depth_formula: &DepthFormula,
    ) -> PartialVMResult<()> {
        let mut cache = self.0.write();
        let struct_info = cache.entry(idx).or_insert_with(StructInfo::none);

        // Cache the formula if it has not been cached, and otherwise return the thread id of the
        // previously cached one. Release the write lock.
        let id = std::thread::current().id();
        let prev_id = match &struct_info.depth_formula {
            None => {
                struct_info.depth_formula = Some((depth_formula.clone(), id));
                return Ok(());
            },
            Some((_, prev_id)) => *prev_id,
        };
        drop(cache);

        if id == prev_id {
            // Same thread has put this entry previously, which means there is a recursion.
            let struct_name = struct_name_index_map.idx_to_struct_name_ref(idx)?;
            return Err(
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR).with_message(
                    format!(
                        "Depth formula for struct '{}' is already cached by the same thread",
                        struct_name.as_ref(),
                    ),
                ),
            );
        }

        Ok(())
    }

    /// Returns cached struct tag and its pseudo-gas cost if it exists, and [None] otherwise.
    pub(crate) fn get_struct_tag(
        &self,
        idx: &StructNameIndex,
        ty_args: &[Type],
    ) -> Option<(StructTag, u64)> {
        self.0
            .read()
            .get(idx)?
            .instantiation_info
            .get(ty_args)?
            .struct_tag
            .as_ref()
            .cloned()
    }

    /// Caches annotated struct tag and its pseudo-gas cost.
    pub(crate) fn store_struct_tag(
        &self,
        idx: StructNameIndex,
        ty_args: Vec<Type>,
        struct_tag: StructTag,
        cost: u64,
    ) {
        self.0
            .write()
            .entry(idx)
            .or_insert_with(StructInfo::none)
            .instantiation_info
            .entry(ty_args)
            .or_insert_with(StructInstantiationInfo::none)
            .struct_tag = Some((struct_tag, cost));
    }

    /// Returns struct layout information if it has been cached, and [None] otherwise.
    pub(crate) fn get_struct_layout_info(
        &self,
        idx: &StructNameIndex,
        ty_args: &[Type],
    ) -> Option<(MoveTypeLayout, u64, bool)> {
        self.0
            .read()
            .get(idx)?
            .instantiation_info
            .get(ty_args)?
            .struct_layout_info
            .as_ref()
            .map(|info| info.clone().unpack())
    }

    /// Caches struct layout information.
    pub(crate) fn store_struct_layout_info(
        &self,
        idx: StructNameIndex,
        ty_args: Vec<Type>,
        struct_layout: MoveTypeLayout,
        node_count: u64,
        has_identifier_mappings: bool,
    ) {
        let mut cache = self.0.write();
        let info = cache
            .entry(idx)
            .or_insert_with(StructInfo::none)
            .instantiation_info
            .entry(ty_args)
            .or_insert_with(StructInstantiationInfo::none);
        info.struct_layout_info = Some(StructLayoutInfo {
            struct_layout,
            node_count,
            has_identifier_mappings,
        });
    }

    /// Returns annotated struct layout information if it has been cached, and [None] otherwise.
    pub(crate) fn get_annotated_struct_layout_info(
        &self,
        idx: &StructNameIndex,
        ty_args: &[Type],
    ) -> Option<(MoveTypeLayout, u64)> {
        self.0
            .read()
            .get(idx)?
            .instantiation_info
            .get(ty_args)?
            .annotated_struct_layout_info
            .as_ref()
            .cloned()
    }

    /// Caches annotated struct layout information.
    pub(crate) fn store_annotated_struct_layout_info(
        &self,
        idx: StructNameIndex,
        ty_args: Vec<Type>,
        struct_layout: MoveTypeLayout,
        node_count: u64,
    ) {
        let mut cache = self.0.write();
        let info = cache
            .entry(idx)
            .or_insert_with(StructInfo::none)
            .instantiation_info
            .entry(ty_args)
            .or_insert_with(StructInstantiationInfo::none);
        info.annotated_struct_layout_info = Some((struct_layout, node_count));
    }
}

impl Clone for StructInfoCache {
    fn clone(&self) -> Self {
        Self(RwLock::new(self.0.read().clone()))
    }
}

#[cfg(test)]
mod test {
    // TODO(loader_v2):
    //   This has never been tested before, so we definitely should be adding tests.
}
