// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_native_interface::{SafeNativeContext, SafeNativeError, SafeNativeResult};
use move_vm_types::{loaded_data::runtime_types::Type, values::Value};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

pub fn native_load_layouts(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(args.is_empty());

    // TODO(lazy-loading): charge gas?
    if context.get_feature_flags().is_lazy_loading_enabled() {
        Err(SafeNativeError::LoadLayouts {
            tys: ty_args,
            annotated: false,
        })
    } else {
        Ok(smallvec![])
    }
}

pub fn native_load_annotated_layouts(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(args.is_empty());

    // TODO(lazy-loading): charge gas?
    if context.get_feature_flags().is_lazy_loading_enabled() {
        Err(SafeNativeError::LoadLayouts {
            tys: ty_args,
            annotated: true,
        })
    } else {
        Ok(smallvec![])
    }
}
