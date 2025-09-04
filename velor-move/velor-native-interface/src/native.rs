// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{context::SafeNativeContext, errors::SafeNativeResult};
use move_vm_types::{loaded_data::runtime_types::Type, values::Value};
use smallvec::SmallVec;
use std::collections::VecDeque;

/// Type alias representing a raw native function.
///
/// A raw native needs to be made into a closure that carries various configurations before
/// it can be used in the VM.
pub type RawSafeNative = fn(
    &mut SafeNativeContext,
    Vec<Type>,
    VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>>;
