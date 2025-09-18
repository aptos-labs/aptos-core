// Copyright (c) 2024 Supra.

use std::collections::VecDeque;
use ark_std::iterable::Iterable;
use keccak_hash::H256;
use rlp::Rlp;
use smallvec::{smallvec, SmallVec};
use aptos_gas_schedule::gas_params::natives::aptos_framework::{RLP_ENCODE_DECODE_BASE, RLP_ENCODE_DECODE_PER_BYTE, UTIL_FROM_BYTES_BASE, UTIL_FROM_BYTES_PER_BYTE};
use aptos_native_interface::{safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeError, SafeNativeResult};
use move_core_types::account_address::AccountAddress;
use move_core_types::gas_algebra::{NumBytes};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::loaded_data::runtime_types::Type;
use move_vm_types::values::Value;

const E_DECODE_FAILURE: u64 = 0x1;
const E_INVALID_TYPE_ARG: u64 = 0x2;
const E_UNSUPPORTED_TYPE: u64 = 0x3;
const E_ENCODE_FAILURE: u64 = 0x4;

/// Macro: pop a single scalar `T` from `args`, charge gas for `num_bytes`,
/// RLP-encode the value, and return a `vector<u8>` Value.
macro_rules! rlp_encode_scalar {
    ($context:expr, $args:ident, $rust_ty:ty, $num_bytes:expr) => {{
        let x: $rust_ty = safely_pop_arg!($args, $rust_ty);
        $context.charge(
            RLP_ENCODE_DECODE_BASE +
            RLP_ENCODE_DECODE_PER_BYTE * NumBytes::new($num_bytes)
        )?;
        let encoded_data = rlp::encode(&x).to_vec();
        Ok(smallvec![Value::vector_u8(encoded_data)])
    }};
}

/// Macro: pop a `Vec<u8>` from `args`, charge gas, RLP-decode into `T`,
/// and return a `Value::T(...)`.
macro_rules! rlp_decode_scalar {
    ($context:expr, $args:ident, $rust_ty:ty, $value_ctor:expr) => {{
        let encoded_data: Vec<u8> = safely_pop_arg!($args, Vec<u8>);
        // Gas charge
        $context.charge(
            RLP_ENCODE_DECODE_BASE +
            RLP_ENCODE_DECODE_PER_BYTE * NumBytes::new(encoded_data.len() as u64)
        )?;
        // RLP decode
        match rlp::decode::<$rust_ty>(&encoded_data) {
            Ok(decoded_val) => Ok(smallvec![$value_ctor(decoded_val)]),
            Err(_) => Err(SafeNativeError::Abort {
                abort_code: E_DECODE_FAILURE,
            }),
        }
    }};
}

/// Macro to BCS-deserialize from `data` into a `Vec<T>` of the chosen type, then RLP-encode it.
macro_rules! rlp_encode_list {
    ($data:expr, $rust_ty:ty, $err_code:expr) => {{
        let vec_t: Vec<$rust_ty> = match bcs::from_bytes(&$data) {
            Ok(val) => val,
            Err(_) => return Err(SafeNativeError::Abort { abort_code: $err_code }),
        };
        let encoded = rlp::encode_list(&vec_t).to_vec();
        Ok(smallvec![Value::vector_u8(encoded)])
    }};
}

/// Macro to decode from RLP as `Vec<T>`
macro_rules! rlp_decode_list {
    ($rlp:expr, $rust_ty:ty, $build_value_fn:expr, $err_code:expr) => {{
        match $rlp.as_list::<$rust_ty>() {
            Ok(decoded) => Ok(smallvec![$build_value_fn(decoded)]),
            Err(_) => Err(SafeNativeError::Abort { abort_code: $err_code }),
        }
    }};
}

//
// ----------------------------------------------------------------------------
// Encode/Decode for type T
// Types supported: bool, u8, u16, u32, u64, u128, address, vector<u8>
// Attempting to encode any other type results in E_UNSUPPORTED_TYPE error
// ----------------------------------------------------------------------------
//
fn native_rlp_encode(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {

    if ty_args.len() != 1 {
        return Err(SafeNativeError::Abort { abort_code: E_INVALID_TYPE_ARG });
    }

    let inner_ty = &ty_args[0];

    match inner_ty {
        Type::Bool  => rlp_encode_scalar!(context, args, bool, 1),
        Type::U8    => rlp_encode_scalar!(context, args, u8,   1),
        Type::U16   => rlp_encode_scalar!(context, args, u16,  2),
        Type::U32   => rlp_encode_scalar!(context, args, u32,  4),
        Type::U64   => rlp_encode_scalar!(context, args, u64,  8),
        Type::U128  => rlp_encode_scalar!(context, args, u128, 16),

        // address is custom
        Type::Address => {
            let addr: AccountAddress = safely_pop_arg!(args, AccountAddress);
            let addr_bytes = addr.into_bytes();
            context.charge(
                RLP_ENCODE_DECODE_BASE +
                    RLP_ENCODE_DECODE_PER_BYTE * NumBytes::new(addr_bytes.len() as u64)
            )?;
            let addr = H256::from(addr_bytes);
            let encoded_data = rlp::encode(&addr).to_vec();
            Ok(smallvec![Value::vector_u8(encoded_data)])
        }

        // vector<u8> is also custom
        Type::Vector(inner_type) => {
            match &**inner_type {
                // only vector<u8> encoding is supported
                Type::U8 => {
                    let bytes: Vec<u8> = safely_pop_arg!(args, Vec<u8>);
                    context.charge(
                        RLP_ENCODE_DECODE_BASE +
                            RLP_ENCODE_DECODE_PER_BYTE * NumBytes::new(bytes.len() as u64)
                    )?;
                    let encoded_data = rlp::encode(&bytes).to_vec();
                    Ok(smallvec![Value::vector_u8(encoded_data)])
                }
                _ => Err(SafeNativeError::Abort { abort_code: E_UNSUPPORTED_TYPE }),
            }
        }

        _ => Err(SafeNativeError::Abort { abort_code: E_UNSUPPORTED_TYPE }),
    }
}

fn native_rlp_decode(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {

    if ty_args.len() != 1 {
        return Err(SafeNativeError::Abort { abort_code: E_INVALID_TYPE_ARG });
    }

    let inner_ty = &ty_args[0];

    match inner_ty {
        Type::Bool   => rlp_decode_scalar!(context, args, bool,  Value::bool),
        Type::U8     => rlp_decode_scalar!(context, args, u8,    Value::u8),
        Type::U16    => rlp_decode_scalar!(context, args, u16,   Value::u16),
        Type::U32    => rlp_decode_scalar!(context, args, u32,   Value::u32),
        Type::U64    => rlp_decode_scalar!(context, args, u64,   Value::u64),
        Type::U128   => rlp_decode_scalar!(context, args, u128,  Value::u128),
        Type::Address => {
            let encoded_data: Vec<u8> = safely_pop_arg!(args, Vec<u8>);
            context.charge(
                RLP_ENCODE_DECODE_BASE +
                    RLP_ENCODE_DECODE_PER_BYTE * NumBytes::new(encoded_data.len() as u64)
            )?;
            // address => decode as Vec<u8>, then convert
            match rlp::decode::<Vec<u8>>(&encoded_data) {
                Ok(addr_bytes) => {
                    match AccountAddress::from_bytes(addr_bytes) {
                        Ok(addr) => Ok(smallvec![Value::address(addr)]),
                        Err(_)   => Err(SafeNativeError::Abort { abort_code: E_DECODE_FAILURE }),
                    }
                },
                Err(_) => Err(SafeNativeError::Abort { abort_code: E_DECODE_FAILURE }),
            }
        }
        Type::Vector(inner_type) => {
            match &**inner_type {
                // only vector<u8> encoding is supported
                Type::U8 => {
                    let encoded_data: Vec<u8> = safely_pop_arg!(args, Vec<u8>);
                    context.charge(
                        RLP_ENCODE_DECODE_BASE +
                            RLP_ENCODE_DECODE_PER_BYTE * NumBytes::new(encoded_data.len() as u64)
                    )?;
                    match rlp::decode::<Vec<u8>>(&encoded_data) {
                        Ok(decoded_bytes) => Ok(smallvec![Value::vector_u8(decoded_bytes)]),
                        Err(_) => Err(SafeNativeError::Abort { abort_code: E_DECODE_FAILURE }),
                    }
                }
                _ => Err(SafeNativeError::Abort { abort_code: E_UNSUPPORTED_TYPE }),
            }
        }

        _ => Err(SafeNativeError::Abort { abort_code: E_UNSUPPORTED_TYPE }),
    }
}

//
// ----------------------------------------------------------------------------
// Encode/Decode for list
// Type of lists supported: bool, u8, u16, u32, u64, u128, address
// Attempting to encode any other type results in E_UNSUPPORTED_TYPE error
// ----------------------------------------------------------------------------
//
fn native_rlp_encode_list_scalar(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {

    if ty_args.len() != 1 {
        return Err(SafeNativeError::Abort { abort_code: E_INVALID_TYPE_ARG });
    }
    let inner_ty = &ty_args[0];

    let data: Vec<u8> = safely_pop_arg!(args, Vec<u8>);
    let total_data_bytes = data.len() as u64;
    context.charge(
        RLP_ENCODE_DECODE_BASE+
            RLP_ENCODE_DECODE_PER_BYTE * NumBytes::new(total_data_bytes))?;

    match inner_ty {
        Type::Bool => {
            rlp_encode_list!(data, bool, E_ENCODE_FAILURE)
        }
        Type::U8 => {
            rlp_encode_list!(data, u8, E_ENCODE_FAILURE)
        }
        Type::U16 => {
            rlp_encode_list!(data, u16, E_ENCODE_FAILURE)
        }
        Type::U32 => {
            rlp_encode_list!(data, u32, E_ENCODE_FAILURE)
        }
        Type::U64 => {
            rlp_encode_list!(data, u64, E_ENCODE_FAILURE)
        }
        Type::U128 => {
            rlp_encode_list!(data, u128, E_ENCODE_FAILURE)
        }
        Type::Address => {
            // Address is a bit more custom
            let data: Vec<AccountAddress> = bcs::from_bytes(&data)
                .map_err(|_| SafeNativeError::Abort { abort_code: E_ENCODE_FAILURE })?;

            let mut address_vec = vec![];
            for addr in data {
                let addr_bytes = addr.into_bytes();
                let addr = H256::from(addr_bytes);
                address_vec.push(addr);
            }
            let encoded_data = rlp::encode_list(&address_vec).to_vec();
            Ok(smallvec![Value::vector_u8(encoded_data)])
        }
        _ => {
            Err(SafeNativeError::Abort { abort_code: E_UNSUPPORTED_TYPE })
        }
    }
}

fn native_rlp_decode_list_scalar(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {

    if ty_args.len() != 1 {
        return Err(SafeNativeError::Abort { abort_code: E_INVALID_TYPE_ARG });
    }
    let inner_ty = &ty_args[0];

    let encoded_data: Vec<u8> = safely_pop_arg!(args, Vec<u8>);
    context.charge(
        RLP_ENCODE_DECODE_BASE+
            RLP_ENCODE_DECODE_PER_BYTE * NumBytes::new(encoded_data.len() as u64)
    )?;

    let rlp = Rlp::new(&encoded_data);

    match inner_ty {
        Type::Bool => {
            rlp_decode_list!(rlp, bool, Value::vector_bool, E_DECODE_FAILURE)
        },
        Type::U8 => {
            rlp_decode_list!(rlp, u8, Value::vector_u8, E_DECODE_FAILURE)
        },
        Type::U16 => {
            rlp_decode_list!(rlp, u16, Value::vector_u16, E_DECODE_FAILURE)
        },
        Type::U32 => {
            rlp_decode_list!(rlp, u32, Value::vector_u32, E_DECODE_FAILURE)
        },
        Type::U64 => {
            rlp_decode_list!(rlp, u64, Value::vector_u64, E_DECODE_FAILURE)
        },
        Type::U128 => {
            rlp_decode_list!(rlp, u128, Value::vector_u128, E_DECODE_FAILURE)
        },
        Type::Address => {
            // Custom logic for addresses
            match rlp.as_list::<Vec<u8>>() {
                Ok(decoded) => {
                    let mut address_vec = vec![];
                    for address_bytes in decoded {
                        if address_bytes.len() > AccountAddress::LENGTH {
                            return Err(SafeNativeError::Abort {
                                abort_code: E_DECODE_FAILURE,
                            });
                        }
                        match AccountAddress::from_bytes(address_bytes) {
                            Ok(addr) => address_vec.push(addr),
                            Err(_) => return Err(SafeNativeError::Abort {
                                abort_code: E_DECODE_FAILURE,
                            }),
                        }
                    }
                    Ok(smallvec![Value::vector_address(address_vec)])
                },
                Err(_) => Err(SafeNativeError::Abort { abort_code: E_DECODE_FAILURE }),
            }
        },
        _ => Err(SafeNativeError::Abort { abort_code: E_UNSUPPORTED_TYPE }),
    }
}

fn native_rlp_encode_list_byte_array(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {

    let data: Vec<u8> = safely_pop_arg!(args, Vec<u8>);
    let total_data_bytes = data.len() as u64;
    context.charge(
        RLP_ENCODE_DECODE_BASE+
            RLP_ENCODE_DECODE_PER_BYTE * NumBytes::new(total_data_bytes))?;
    let data: Vec<Vec<u8>> = bcs::from_bytes(&data)
        .map_err(|_| SafeNativeError::Abort { abort_code: E_ENCODE_FAILURE })?;
    let encoded_data = rlp::encode_list::<Vec<u8>, _>(&data).to_vec();
    Ok(smallvec![Value::vector_u8(encoded_data)])
}

fn native_rlp_decode_list_byte_array(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {

    let encoded_data: Vec<u8> = safely_pop_arg!(args, Vec<u8>);
    context.charge(
        RLP_ENCODE_DECODE_BASE+
            RLP_ENCODE_DECODE_PER_BYTE * NumBytes::new(encoded_data.len() as u64)
    )?;

    let rlp = Rlp::new(&encoded_data);

    match rlp.as_list::<Vec<u8>>() {
        Ok(decoded) => {
            //serialize and return data
            context.charge(
                UTIL_FROM_BYTES_BASE+
                    UTIL_FROM_BYTES_PER_BYTE * NumBytes::new(encoded_data.len() as u64)
            )?;
            let serialized_data = bcs::to_bytes(&decoded)
                .map_err(|_| SafeNativeError::Abort { abort_code: E_DECODE_FAILURE })?;
            Ok(smallvec![Value::vector_u8(serialized_data)])
        },
        Err(_) => Err(SafeNativeError::Abort { abort_code: E_DECODE_FAILURE }),
    }
}

pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let mut natives = vec![];

    natives.extend([
        ("native_rlp_encode", native_rlp_encode as RawSafeNative),
        ("native_rlp_decode", native_rlp_decode as RawSafeNative),

        ("native_rlp_encode_list_scalar", native_rlp_encode_list_scalar as RawSafeNative),
        ("native_rlp_decode_list_scalar", native_rlp_decode_list_scalar as RawSafeNative),

        ("native_rlp_encode_list_byte_array", native_rlp_encode_list_byte_array as RawSafeNative),
        ("native_rlp_decode_list_byte_array", native_rlp_decode_list_byte_array as RawSafeNative),
    ]);

    builder.make_named_natives(natives)
}
