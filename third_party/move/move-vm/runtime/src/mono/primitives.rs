// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::mono::{context::ExecutionContext, memory::Memory};
use move_binary_format::errors::PartialVMResult;

pub fn add_u64(
    ctx: &dyn ExecutionContext,
    memory: &mut Memory,
    _size: usize,
) -> PartialVMResult<()> {
    let x2 = memory.pop_value::<u64>(ctx)?;
    let x1 = memory.pop_value::<u64>(ctx)?;
    // TODO: overflow
    memory.push_value(ctx, x1 + x2)
}

pub fn sub_u64(
    ctx: &dyn ExecutionContext,
    memory: &mut Memory,
    _size: usize,
) -> PartialVMResult<()> {
    let x2 = memory.pop_value::<u64>(ctx)?;
    let x1 = memory.pop_value::<u64>(ctx)?;
    // TODO: underflow
    memory.push_value(ctx, x1 - x2)
}

pub fn mul_u64(
    ctx: &dyn ExecutionContext,
    memory: &mut Memory,
    _size: usize,
) -> PartialVMResult<()> {
    let x2 = memory.pop_value::<u64>(ctx)?;
    let x1 = memory.pop_value::<u64>(ctx)?;
    // TODO: overflow
    memory.push_value(ctx, x1 * x2)
}

pub fn equals(ctx: &dyn ExecutionContext, memory: &mut Memory, size: usize) -> PartialVMResult<()> {
    // This is generic for arbitrary types, similar as compare would be, though need to revisit
    // embedded dynamically sized vectors.
    let x1 = memory.view(size + size, size);
    let x2 = memory.view(size, size);
    let is_eq = x1.view_as_slice() == x2.view_as_slice();
    memory.collapse(ctx, size + size, 0)?;
    memory.push_value(ctx, is_eq)
}

// ... and many more

// TODO: native interface via primitives
