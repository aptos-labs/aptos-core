// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    natives::helpers::{make_safe_native, SafeNativeContext, SafeNativeError, SafeNativeResult},
    safely_pop_arg,
};
use aptos_types::on_chain_config::{Features, TimedFeatures};
use move_binary_format::errors::PartialVMError;
use move_core_types::{
    gas_algebra::{InternalGas, InternalGasPerByte, NumBytes},
    vm_status::StatusCode,
};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{loaded_data::runtime_types::Type, values::Value};
use smallvec::{smallvec, SmallVec};
use std::{collections::VecDeque, sync::Arc};

// !!!! NOTE !!!!
// This file is intended for natives from the util module in the framework.
// DO NOT PUT HELPER FUNCTIONS HERE!

/// Abort code when from_bytes fails (0x01 == INVALID_ARGUMENT)
const EFROM_BYTES: u64 = 0x01_0001;

/***************************************************************************************************
 * native fun from_bytes
 *
 *   gas cost: base_cost + unit_cost * bytes_len
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct FromBytesGasParameters {
    pub base: InternalGas,
    pub per_byte: InternalGasPerByte,
}

fn native_from_bytes(
    gas_params: &FromBytesGasParameters,
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(ty_args.len(), 1);
    debug_assert_eq!(args.len(), 1);

    // TODO(Gas): charge for getting the layout
    let layout = context.type_to_type_layout(&ty_args[0])?.ok_or_else(|| {
        PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR).with_message(format!(
            "Failed to get layout of type {:?} -- this should not happen",
            ty_args[0]
        ))
    })?;

    let bytes = safely_pop_arg!(args, Vec<u8>);
    context.charge(gas_params.base + gas_params.per_byte * NumBytes::new(bytes.len() as u64))?;
    let val = match Value::simple_deserialize(&bytes, &layout) {
        Some(val) => val,
        None => {
            return Err(SafeNativeError::Abort {
                abort_code: EFROM_BYTES,
            })
        },
    };

    Ok(smallvec![val])
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct GasParameters {
    pub from_bytes: FromBytesGasParameters,
}

pub fn make_all(
    gas_params: GasParameters,
    timed_features: TimedFeatures,
    features: Arc<Features>,
) -> impl Iterator<Item = (String, NativeFunction)> {
    let natives = [(
        "from_bytes",
        make_safe_native(
            gas_params.from_bytes,
            timed_features,
            features,
            native_from_bytes,
        ),
    )];

    crate::natives::helpers::make_module_natives(natives)
}
