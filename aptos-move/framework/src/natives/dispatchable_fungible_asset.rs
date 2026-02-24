// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::function_info::extract_function_info;
use aptos_gas_schedule::gas_params::natives::aptos_framework::DISPATCHABLE_FUNGIBLE_ASSET_DISPATCH_BASE;
use aptos_native_interface::{
    RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeError, SafeNativeResult,
};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{loaded_data::runtime_types::Type, values::Value};
use smallvec::SmallVec;
use std::collections::VecDeque;

mod abort_codes {
    /// Dispatch target is not loaded
    pub const ENOT_LOADED: u64 = 4;
}

/***************************************************************************************************
 * native fun dispatchable_withdraw / dispatchable_deposit / dispatchable_derived_balance / dispatchable_derived_supply
 *
 *   Directs control flow based on the last argument. We use the same native function implementation
 *   for all dispatching native.
 *   gas cost: a flat fee because we charged the loading of those modules previously.
 *
 **************************************************************************************************/
pub(crate) fn native_dispatch(
    context: &mut SafeNativeContext,
    ty_args: &[Type],
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    let (module_name, func_name) = extract_function_info(&mut arguments)?;

    // Check if the module is already properly charged in this transaction.
    let check_visited = |a, n| {
        let special_addresses_considered_visited =
            context.get_feature_flags().is_account_abstraction_enabled()
                || context
                    .get_feature_flags()
                    .is_derivable_account_abstraction_enabled();
        if special_addresses_considered_visited {
            context
                .traversal_context()
                .check_is_special_or_visited(a, n)
        } else {
            context.traversal_context().legacy_check_visited(a, n)
        }
    };
    check_visited(module_name.address(), module_name.name()).map_err(|_| {
        SafeNativeError::abort_with_message(
            abort_codes::ENOT_LOADED,
            format!(
                "Module {}::{} is not loaded prior to native dispatch",
                module_name.address(),
                module_name.name()
            ),
        )
    })?;

    context.charge(DISPATCHABLE_FUNGIBLE_ASSET_DISPATCH_BASE)?;

    // Use Error to instruct the VM to perform a function call dispatch.
    Err(SafeNativeError::FunctionDispatch {
        module_name,
        func_name,
        ty_args: ty_args.to_vec(),
        args: arguments.into_iter().collect(),
    })
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let natives = [
        ("dispatchable_withdraw", native_dispatch as RawSafeNative),
        ("dispatchable_deposit", native_dispatch),
        ("dispatchable_derived_balance", native_dispatch),
        ("dispatchable_derived_supply", native_dispatch),
    ];

    builder.make_named_natives(natives)
}
