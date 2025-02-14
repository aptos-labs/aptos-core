use std::collections::VecDeque;
use keccak_hash::H256;
use smallvec::{smallvec, SmallVec};
use aptos_gas_schedule::gas_params::natives::aptos_framework::{RLP_ENCODE_DECODE_BASE, RLP_ENCODE_DECODE_PER_BYTE};
use aptos_native_interface::{safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeError, SafeNativeResult};
use move_core_types::account_address::AccountAddress;
use move_core_types::gas_algebra::{NumBytes};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::loaded_data::runtime_types::Type;
use move_vm_types::values::Value;

pub const E_DECODE_FAILURE: u64 = 0x1;

//
// ----------------------------------------------------------------------------
// Encode/Decode for bool
// ----------------------------------------------------------------------------
//
pub fn native_rlp_encode_bool(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    let x: bool = safely_pop_arg!(args, bool);
    context.charge(RLP_ENCODE_DECODE_BASE + RLP_ENCODE_DECODE_PER_BYTE * NumBytes::new(1))?;
    let encoded_data = rlp::encode(&x);
    Ok(smallvec![Value::vector_u8(encoded_data.to_vec())])
}

pub fn native_rlp_decode_bool(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    let encoded_data: Vec<u8> = safely_pop_arg!(args, Vec<u8>);
    context.charge(RLP_ENCODE_DECODE_BASE + RLP_ENCODE_DECODE_PER_BYTE * NumBytes::new(encoded_data.len() as u64))?;

    // Attempt RLP decode
    match rlp::decode::<bool>(&encoded_data) {
        Ok(decoded_bool) => {
            Ok(smallvec![Value::bool(decoded_bool)])
        },
        Err(_e) => {
            Err(SafeNativeError::Abort {
                abort_code: E_DECODE_FAILURE,
            })
        }
    }
}

//
// ----------------------------------------------------------------------------
// Encode/Decode for u8
// ----------------------------------------------------------------------------
//
pub fn native_rlp_encode_u8(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    let x: u8 = safely_pop_arg!(args, u8);
    context.charge(RLP_ENCODE_DECODE_BASE + RLP_ENCODE_DECODE_PER_BYTE * NumBytes::new(1))?;
    let encoded_data = rlp::encode(&x);
    Ok(smallvec![Value::vector_u8(encoded_data.to_vec())])
}

pub fn native_rlp_decode_u8(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    let encoded_data: Vec<u8> = safely_pop_arg!(args, Vec<u8>);
    context.charge(RLP_ENCODE_DECODE_BASE + RLP_ENCODE_DECODE_PER_BYTE * NumBytes::new(encoded_data.len() as u64))?;

    // Attempt RLP decode
    match rlp::decode::<u8>(&encoded_data) {
        Ok(decoded_bytes) => {
            Ok(smallvec![Value::u8(decoded_bytes)])
        },
        Err(_e) => {
            Err(SafeNativeError::Abort {
                abort_code: E_DECODE_FAILURE,
            })
        }
    }
}

//
// ----------------------------------------------------------------------------
// Encode/Decode for u16
// ----------------------------------------------------------------------------
//
pub fn native_rlp_encode_u16(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    let x: u16 = safely_pop_arg!(args, u16);

    // Charge gas based on the size of a u16 (2 bytes)
    context.charge(
        RLP_ENCODE_DECODE_BASE +
            RLP_ENCODE_DECODE_PER_BYTE * NumBytes::new(2)
    )?;

    let encoded_data = rlp::encode(&x); // RLP-encode the integer
    Ok(smallvec![Value::vector_u8(encoded_data.to_vec())])
}

pub fn native_rlp_decode_u16(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    // RLP-encoded data as raw bytes
    let encoded_data: Vec<u8> = safely_pop_arg!(args, Vec<u8>);

    // Charge gas based on length of the encoded data
    context.charge(
        RLP_ENCODE_DECODE_BASE +
            RLP_ENCODE_DECODE_PER_BYTE * NumBytes::new(encoded_data.len() as u64)
    )?;

    match rlp::decode::<u16>(&encoded_data) {
        Ok(decoded_val) => {
            Ok(smallvec![Value::u16(decoded_val)])
        },
        Err(_) => {
            Err(SafeNativeError::Abort {
                abort_code: E_DECODE_FAILURE,
            })
        }
    }
}

//
// ----------------------------------------------------------------------------
// Encode/Decode for u32
// ----------------------------------------------------------------------------
//
pub fn native_rlp_encode_u32(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    let x: u32 = safely_pop_arg!(args, u32);

    // 4 bytes for u32
    context.charge(
        RLP_ENCODE_DECODE_BASE +
            RLP_ENCODE_DECODE_PER_BYTE * NumBytes::new(4)
    )?;

    let encoded_data = rlp::encode(&x);
    Ok(smallvec![Value::vector_u8(encoded_data.to_vec())])
}

pub fn native_rlp_decode_u32(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    let encoded_data: Vec<u8> = safely_pop_arg!(args, Vec<u8>);

    context.charge(
        RLP_ENCODE_DECODE_BASE +
            RLP_ENCODE_DECODE_PER_BYTE * NumBytes::new(encoded_data.len() as u64)
    )?;

    match rlp::decode::<u32>(&encoded_data) {
        Ok(decoded_val) => {
            Ok(smallvec![Value::u32(decoded_val)])
        },
        Err(_) => {
            Err(SafeNativeError::Abort {
                abort_code: E_DECODE_FAILURE,
            })
        }
    }
}

//
// ----------------------------------------------------------------------------
// Encode/Decode for u64
// ----------------------------------------------------------------------------
//
pub fn native_rlp_encode_u64(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    let x: u64 = safely_pop_arg!(args, u64);

    // 8 bytes for u64
    context.charge(
        RLP_ENCODE_DECODE_BASE +
            RLP_ENCODE_DECODE_PER_BYTE * NumBytes::new(8)
    )?;

    let encoded_data = rlp::encode(&x);
    Ok(smallvec![Value::vector_u8(encoded_data.to_vec())])
}

pub fn native_rlp_decode_u64(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    let encoded_data: Vec<u8> = safely_pop_arg!(args, Vec<u8>);

    context.charge(
        RLP_ENCODE_DECODE_BASE +
            RLP_ENCODE_DECODE_PER_BYTE * NumBytes::new(encoded_data.len() as u64)
    )?;

    match rlp::decode::<u64>(&encoded_data) {
        Ok(decoded_val) => {
            Ok(smallvec![Value::u64(decoded_val)])
        },
        Err(_) => {
            Err(SafeNativeError::Abort {
                abort_code: E_DECODE_FAILURE,
            })
        }
    }
}

//
// ----------------------------------------------------------------------------
// Encode/Decode for u128
// ----------------------------------------------------------------------------
//
pub fn native_rlp_encode_u128(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    let x: u128 = safely_pop_arg!(args, u128);

    // 16 bytes for u128
    context.charge(
        RLP_ENCODE_DECODE_BASE +
            RLP_ENCODE_DECODE_PER_BYTE * NumBytes::new(16)
    )?;

    let encoded_data = rlp::encode(&x);
    Ok(smallvec![Value::vector_u8(encoded_data.to_vec())])
}

pub fn native_rlp_decode_u128(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    let encoded_data: Vec<u8> = safely_pop_arg!(args, Vec<u8>);

    context.charge(
        RLP_ENCODE_DECODE_BASE +
            RLP_ENCODE_DECODE_PER_BYTE * NumBytes::new(encoded_data.len() as u64)
    )?;

    match rlp::decode::<u128>(&encoded_data) {
        Ok(decoded_val) => {
            Ok(smallvec![Value::u128(decoded_val)])
        },
        Err(_) => {
            Err(SafeNativeError::Abort {
                abort_code: E_DECODE_FAILURE,
            })
        }
    }
}

//
// ----------------------------------------------------------------------------
// Encode/Decode for byte vector
// ----------------------------------------------------------------------------
//
pub fn native_rlp_encode_bytes(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    let bytes: Vec<u8> = safely_pop_arg!(args, Vec<u8>);

    // Gas depends on input length
    context.charge(
        RLP_ENCODE_DECODE_BASE +
            RLP_ENCODE_DECODE_PER_BYTE * NumBytes::new(bytes.len() as u64)
    )?;

    let encoded_data = rlp::encode(&bytes);
    Ok(smallvec![Value::vector_u8(encoded_data.to_vec())])
}

pub fn native_rlp_decode_bytes(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    let encoded_data: Vec<u8> = safely_pop_arg!(args, Vec<u8>);

    context.charge(
        RLP_ENCODE_DECODE_BASE +
            RLP_ENCODE_DECODE_PER_BYTE * NumBytes::new(encoded_data.len() as u64)
    )?;

    match rlp::decode::<Vec<u8>>(&encoded_data) {
        Ok(decoded_bytes) => {
            Ok(smallvec![Value::vector_u8(decoded_bytes)])
        },
        Err(_) => {
            Err(SafeNativeError::Abort {
                abort_code: E_DECODE_FAILURE,
            })
        }
    }
}

//
// ----------------------------------------------------------------------------
// Encode/Decode for address
// ----------------------------------------------------------------------------
//
pub fn native_rlp_encode_address(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    let addr: AccountAddress = safely_pop_arg!(args, AccountAddress);

    // Convert address to bytes
    let addr_bytes = addr.into_bytes();
    context.charge(
        RLP_ENCODE_DECODE_BASE +
            RLP_ENCODE_DECODE_PER_BYTE * NumBytes::new(addr_bytes.len() as u64)
    )?;

    let addr = H256::from(addr_bytes);
    let encoded_data = rlp::encode(&addr);
    Ok(smallvec![Value::vector_u8(encoded_data.to_vec())])
}

pub fn native_rlp_decode_address(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    let encoded_data: Vec<u8> = safely_pop_arg!(args, Vec<u8>);

    context.charge(
        RLP_ENCODE_DECODE_BASE +
            RLP_ENCODE_DECODE_PER_BYTE * NumBytes::new(encoded_data.len() as u64)
    )?;

    // First decode as a raw byte vector
    match rlp::decode::<Vec<u8>>(&encoded_data) {
        Ok(addr_bytes) => {
            if addr_bytes.len() > AccountAddress::LENGTH {
                return Err(SafeNativeError::Abort {
                    abort_code: E_DECODE_FAILURE,
                });
            }
            // Convert back to an AccountAddress
            match AccountAddress::from_bytes(addr_bytes) {
                Ok(addr) => Ok(smallvec![Value::address(addr)]),
                Err(_) => Err(SafeNativeError::Abort {
                    abort_code: E_DECODE_FAILURE,
                }),
            }
        },
        Err(_) => {
            Err(SafeNativeError::Abort {
                abort_code: E_DECODE_FAILURE,
            })
        }
    }
}

pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let mut natives = vec![];

    natives.extend([

        // bool
        ("native_rlp_encode_bool", native_rlp_encode_bool as RawSafeNative),
        ("native_rlp_decode_bool", native_rlp_decode_bool as RawSafeNative),

        // u8
        ("native_rlp_encode_u8", native_rlp_encode_u8 as RawSafeNative),
        ("native_rlp_decode_u8", native_rlp_decode_u8 as RawSafeNative),

        // u16
        ("native_rlp_encode_u16", native_rlp_encode_u16 as RawSafeNative),
        ("native_rlp_decode_u16", native_rlp_decode_u16 as RawSafeNative),

        // u32
        ("native_rlp_encode_u32", native_rlp_encode_u32 as RawSafeNative),
        ("native_rlp_decode_u32", native_rlp_decode_u32 as RawSafeNative),

        // u64
        ("native_rlp_encode_u64", native_rlp_encode_u64 as RawSafeNative),
        ("native_rlp_decode_u64", native_rlp_decode_u64 as RawSafeNative),

        // u128
        ("native_rlp_encode_u128", native_rlp_encode_u128 as RawSafeNative),
        ("native_rlp_decode_u128", native_rlp_decode_u128 as RawSafeNative),

        // bytes
        ("native_rlp_encode_bytes", native_rlp_encode_bytes as RawSafeNative),
        ("native_rlp_decode_bytes", native_rlp_decode_bytes as RawSafeNative),

        // address
        ("native_rlp_encode_address", native_rlp_encode_address as RawSafeNative),
        ("native_rlp_decode_address", native_rlp_decode_address as RawSafeNative),
    ]);

    builder.make_named_natives(natives)
}
