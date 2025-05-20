// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::errors::PartialVMResult;
use move_vm_runtime::native_functions::{NativeContext, NativeFunction};
use move_vm_types::{
    loaded_data::runtime_types::Type, natives::function::NativeResult, values::Value,
};
use smallvec::smallvec;
use std::{collections::VecDeque, sync::Arc};

pub fn native_load_layouts(
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(args.is_empty());

    Ok(
        if context
            .module_storage()
            .runtime_environment()
            .vm_config()
            .enable_lazy_loading
        {
            NativeResult::LoadLayouts {
                tys: ty_args,
                annotated: false,
            }
        } else {
            // TODO(lazy-loading): charge gas?
            NativeResult::ok(0.into(), smallvec![])
        },
    )
}

pub fn make_native_load_layouts() -> NativeFunction {
    Arc::new(
        move |context, ty_args, args| -> PartialVMResult<NativeResult> {
            native_load_layouts(context, ty_args, args)
        },
    )
}
