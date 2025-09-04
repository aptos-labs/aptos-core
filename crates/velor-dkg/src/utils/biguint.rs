// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::SCALAR_NUM_BYTES;
use blstrs::Scalar;
use ff::Field;
use num_bigint::BigUint;

/// Returns the order of the scalar field in our implementation's choice of an elliptic curve group.
pub(crate) fn get_scalar_field_order_as_biguint() -> BigUint {
    let r = BigUint::from_bytes_be(
        hex::decode("73eda753299d7d483339d80809a1d80553bda402fffe5bfeffffffff00000001")
            .unwrap()
            .as_slice(),
    );

    // Here, we paranoically assert that r is correct, by checking 0 - 1 mod r (computed via Scalar) equals r-1 (computed from the constant above)
    let minus_one = Scalar::ZERO - Scalar::ONE;
    let max = &r - 1u8;
    assert_eq!(
        minus_one.to_bytes_le().as_slice(),
        max.to_bytes_le().as_slice()
    );

    r
}

/// Helper function useful when picking a random scalar and when hashing a message into a scalar.
pub fn biguint_to_scalar(big_uint: &BigUint) -> Scalar {
    // `blstrs`'s `Scalar::from_bytes_le` needs `SCALAR_NUM_BYTES` bytes. The current
    // implementation of `BigUint::to_bytes_le()` does not always return `SCALAR_NUM_BYTES` bytes
    // when the integer is smaller than 32 bytes. So we have to pad it.
    let mut bytes = big_uint.to_bytes_le();

    while bytes.len() < SCALAR_NUM_BYTES {
        bytes.push(0u8);
    }

    debug_assert_eq!(BigUint::from_bytes_le(&bytes.as_slice()), *big_uint);

    let slice = match <&[u8; SCALAR_NUM_BYTES]>::try_from(bytes.as_slice()) {
        Ok(slice) => slice,
        Err(_) => {
            panic!(
                "WARNING: Got {} bytes instead of {SCALAR_NUM_BYTES} (i.e., got {})",
                bytes.as_slice().len(),
                big_uint.to_string()
            );
        },
    };

    let opt = Scalar::from_bytes_le(slice);

    if opt.is_some().unwrap_u8() == 1u8 {
        opt.unwrap()
    } else {
        panic!("Deserialization of randomly-generated num_bigint::BigUint failed.");
    }
}
