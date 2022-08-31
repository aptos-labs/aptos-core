// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_types::network_address::NetworkAddress;
use move_deps::{
    move_binary_format::errors::PartialVMResult,
    move_core_types::gas_algebra::{InternalGas, InternalGasPerByte, NumBytes},
    move_vm_runtime::native_functions::{NativeContext, NativeFunction},
    move_vm_types::{
        loaded_data::runtime_types::Type, natives::function::NativeResult, pop_arg, values::Value,
    },
};
use smallvec::smallvec;
use std::{collections::VecDeque, sync::Arc};

#[derive(Debug, Clone)]
pub struct NetworkAddressGasParameters {
    pub base: InternalGas,
    pub per_byte: InternalGasPerByte,
}

fn native_validate_network_addresses(
    gas_params: &NetworkAddressGasParameters,
    _context: &mut NativeContext,
    mut _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(args.len() == 1);

    let bytes = pop_arg!(args, Vec<u8>);
    let cost = gas_params.base + gas_params.per_byte * NumBytes::new(bytes.len() as u64);
    let network_addresses = bcs::from_bytes::<Vec<NetworkAddress>>(&bytes).ok();
    let are_network_addresses_valid = match network_addresses {
        Some(network_addresses) => network_addresses
            .iter()
            .all(|network_address| network_address.is_aptosnet_addr()),
        None => false,
    };

    Ok(NativeResult::ok(
        cost,
        smallvec![Value::bool(are_network_addresses_valid)],
    ))
}

pub fn make_native_validate_network_addresses(
    gas_params: NetworkAddressGasParameters,
) -> NativeFunction {
    Arc::new(move |context, ty_args, args| {
        native_validate_network_addresses(&gas_params, context, ty_args, args)
    })
}

#[derive(Debug, Clone)]
pub struct GasParameters {
    pub validate_network_addresses: NetworkAddressGasParameters,
}

pub fn make_all(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    let natives = [(
        "validate_network_addresses",
        make_native_validate_network_addresses(gas_params.validate_network_addresses),
    )];

    crate::natives::helpers::make_module_natives(natives)
}
