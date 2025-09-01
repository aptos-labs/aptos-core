// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Implementation of data cache for resource and resource groups used by Aptos VM.

use crate::move_vm_ext::AptosMoveResolver;
use aptos_gas_meter::AptosGasMeter;
use aptos_types::state_store::{state_key::StateKey, state_value::StateValueMetadata};
use aptos_vm_types::{
    abstract_write_op::AbstractResourceWriteOp, storage::change_set_configs::ChangeSetSizeTracker,
};
use move_binary_format::errors::{PartialVMResult, VMResult};
use move_core_types::{account_address::AccountAddress, gas_algebra::NumBytes};
use move_vm_runtime::Loader;
use move_vm_types::{
    data_cache::{MoveVmDataCache, NativeMoveVmDataCache},
    delayed_values::delayed_field_id::DelayedFieldID,
    gas::{DependencyGasMeter, GasMeter},
    loaded_data::runtime_types::Type,
    module_traversal::TraversalContext,
    values::{Reference, Value, VectorRef},
};
use std::collections::{BTreeMap, HashSet};

/// Cache for accessed resources and groups.
pub(crate) struct TransactionDataCache {}

impl TransactionDataCache {
    pub(crate) fn empty() -> Self {
        Self {}
    }

    pub(crate) fn save(&mut self) {
        unimplemented!()
    }

    pub(crate) fn undo(&mut self) {
        unimplemented!()
    }

    pub(crate) fn materialize(
        &mut self,
        _data_view: &impl AptosMoveResolver,
        _loader: &impl Loader,
        _new_slot_metadata: &Option<StateValueMetadata>,
        _delayed_field_ids: &HashSet<DelayedFieldID>,
    ) -> PartialVMResult<()> {
        unimplemented!()
    }

    pub(crate) fn charge_write_ops(
        &mut self,
        _change_set_size_tracker: &mut ChangeSetSizeTracker,
        _gas_meter: &mut impl AptosGasMeter,
    ) -> VMResult<()> {
        unimplemented!()
    }

    pub(crate) fn take_writes(&mut self) -> VMResult<BTreeMap<StateKey, AbstractResourceWriteOp>> {
        unimplemented!()
    }
}

/// Adapter to implement [MoveVmDataCache] to pass to the VM to resolve resources or resource
/// group members.
pub(crate) struct TransactionDataCacheAdapter<'a, DataView, CodeLoader> {
    /// Data cache containing all loaded resources.
    #[allow(dead_code)]
    data_cache: &'a mut TransactionDataCache,
    /// Global storage for data.
    #[allow(dead_code)]
    data_view: &'a DataView,
    /// Code loader (needed to extract metadata to check if resource is a group member.
    #[allow(dead_code)]
    loader: &'a CodeLoader,
}

impl<'a, DataView, CodeLoader> TransactionDataCacheAdapter<'a, DataView, CodeLoader>
where
    DataView: AptosMoveResolver,
    CodeLoader: Loader,
{
    /// Returns the new adapter for the data cache.
    pub fn new(
        data_cache: &'a mut TransactionDataCache,
        data_view: &'a DataView,
        loader: &'a CodeLoader,
    ) -> Self {
        Self {
            data_cache,
            data_view,
            loader,
        }
    }
}

impl<'a, DataView, CodeLoader> NativeMoveVmDataCache
    for TransactionDataCacheAdapter<'a, DataView, CodeLoader>
where
    DataView: AptosMoveResolver,
    CodeLoader: Loader,
{
    fn native_exists(
        &mut self,
        _gas_meter: &mut dyn DependencyGasMeter,
        _traversal_context: &mut TraversalContext,
        _addr: AccountAddress,
        _ty: &Type,
    ) -> PartialVMResult<(bool, Option<NumBytes>)> {
        todo!()
    }

    fn copy_on_write(&mut self, _reference: &Reference) -> PartialVMResult<()> {
        todo!()
    }
}

impl<'a, DataView, CodeLoader> MoveVmDataCache
    for TransactionDataCacheAdapter<'a, DataView, CodeLoader>
where
    DataView: AptosMoveResolver,
    CodeLoader: Loader,
{
    fn move_to(
        &mut self,
        _gas_meter: &mut impl GasMeter,
        _traversal_context: &mut TraversalContext,
        _is_generic: bool,
        _addr: AccountAddress,
        _ty: &Type,
        _value: Value,
    ) -> PartialVMResult<()> {
        todo!()
    }

    fn move_from(
        &mut self,
        _gas_meter: &mut impl GasMeter,
        _traversal_context: &mut TraversalContext,
        _is_generic: bool,
        _addr: AccountAddress,
        _ty: &Type,
    ) -> PartialVMResult<Value> {
        todo!()
    }

    fn exists(
        &mut self,
        _gas_meter: &mut impl GasMeter,
        _traversal_context: &mut TraversalContext,
        _is_generic: bool,
        _addr: AccountAddress,
        _ty: &Type,
    ) -> PartialVMResult<bool> {
        todo!()
    }

    fn borrow_global(
        &mut self,
        _gas_meter: &mut impl GasMeter,
        _traversal_context: &mut TraversalContext,
        _is_generic: bool,
        _is_mut: bool,
        _addr: AccountAddress,
        _ty: &Type,
    ) -> PartialVMResult<Value> {
        todo!()
    }

    fn vector_copy_on_write(&mut self, _reference: &VectorRef) -> PartialVMResult<()> {
        todo!()
    }
}
