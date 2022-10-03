// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use framework::natives::state_storage::StateStorageUsageResolver;
use move_core_types::resolver::MoveResolver;
use move_table_extension::TableResolver;
use std::fmt::Debug;

pub trait MoveResolverExt:
    MoveResolver<Err = Self::ExtError> + TableResolver + StateStorageUsageResolver
{
    type ExtError: Debug;
}

impl<E: Debug, T: MoveResolver<Err = E> + TableResolver + StateStorageUsageResolver + ?Sized>
    MoveResolverExt for T
{
    type ExtError = E;
}
