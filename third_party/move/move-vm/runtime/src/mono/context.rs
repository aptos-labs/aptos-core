// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::mono::code::MonoCode;
#[cfg(test)]
use mockall::*;
use move_binary_format::errors::PartialVMResult;

#[derive(Clone, Copy, PartialOrd, PartialEq, Ord, Eq, Debug)]
pub struct FunctionId {
    pub function_hash: u128,
}

#[derive(Clone, PartialOrd, PartialEq, Ord, Eq, Debug)]
pub struct StorageId {
    pub type_hash: u128,
}

#[derive(Clone)]
pub struct FunctionContext<'ctx> {
    pub id: FunctionId,
    pub code: &'ctx [MonoCode<'ctx>],
    pub params_size: usize,
    pub locals_size: usize,
    pub local_table: &'ctx [usize],
}

#[derive(Clone)]
pub struct MemoryBounds {
    pub initial_capacity: usize,
    pub max_size: usize,
}

#[cfg_attr(test, automock)]
pub trait ExecutionContext {
    fn fetch_data<'a>(&'a self, _id: &StorageId) -> PartialVMResult<&'a [u8]>;

    fn fetch_function<'a>(&self, _id: &FunctionId) -> PartialVMResult<FunctionContext<'a>>;

    fn stack_bounds(&self) -> MemoryBounds;

    fn heap_bounds(&self) -> MemoryBounds;
}
