// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use bigdecimal::{FromPrimitive, Signed, ToPrimitive, Zero};

pub fn u64_to_bigdecimal(val: u64) -> bigdecimal::BigDecimal {
    bigdecimal::BigDecimal::from_u64(val).expect("Unable to convert u64 to big decimal")
}

pub fn bigdecimal_to_u64(val: &bigdecimal::BigDecimal) -> u64 {
    val.to_u64().expect("Unable to convert big decimal to u64")
}

pub fn ensure_not_negative(val: bigdecimal::BigDecimal) -> bigdecimal::BigDecimal {
    if val.is_negative() {
        return bigdecimal::BigDecimal::zero();
    }
    val
}
