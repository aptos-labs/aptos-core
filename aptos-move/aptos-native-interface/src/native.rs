// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{context::SafeNativeContext, errors::SafeNativeResult};
use move_vm_types::{ty_interner::TypeId, values::Value};
use smallvec::SmallVec;
use std::collections::VecDeque;

/// Type alias representing a raw native function.
///
/// A raw native needs to be made into a closure that carries various configurations before
/// it can be used in the VM.
pub type RawSafeNative = fn(
    &mut SafeNativeContext,
    &[TypeId],
    VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>>;
