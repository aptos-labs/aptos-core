// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    gas::{DependencyGasMeter, GasMeter},
    loaded_data::runtime_types::Type,
    module_traversal::TraversalContext,
    values::{Reference, Value, VectorRef},
};
use move_binary_format::errors::PartialVMResult;
use move_core_types::{account_address::AccountAddress, gas_algebra::NumBytes};

pub trait NativeMoveVmDataCache {
    fn native_exists(
        &mut self,
        gas_meter: &mut dyn DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        addr: AccountAddress,
        ty: &Type,
    ) -> PartialVMResult<(bool, Option<NumBytes>)>;

    fn copy_on_write(&mut self, reference: &Reference) -> PartialVMResult<()>;
}

pub trait MoveVmDataCache: NativeMoveVmDataCache {
    fn move_to(
        &mut self,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        is_generic: bool,
        addr: AccountAddress,
        ty: &Type,
        value: Value,
    ) -> PartialVMResult<()>;

    fn move_from(
        &mut self,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        is_generic: bool,
        addr: AccountAddress,
        ty: &Type,
    ) -> PartialVMResult<Value>;

    fn exists(
        &mut self,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        is_generic: bool,
        addr: AccountAddress,
        ty: &Type,
    ) -> PartialVMResult<bool>;

    fn borrow_global(
        &mut self,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        is_generic: bool,
        is_mut: bool,
        addr: AccountAddress,
        ty: &Type,
    ) -> PartialVMResult<Value>;

    fn vector_copy_on_write(&mut self, reference: &VectorRef) -> PartialVMResult<()>;
}
