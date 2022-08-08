// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use bigdecimal::{FromPrimitive, ToPrimitive};

pub fn u64_to_bigdecimal(val: u64) -> bigdecimal::BigDecimal {
    bigdecimal::BigDecimal::from_u64(val).unwrap()
}

pub fn bigdecimal_to_u64(val: &bigdecimal::BigDecimal) -> u64 {
    val.to_u64().unwrap()
}
