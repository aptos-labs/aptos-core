// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn signed_div_rem(dividend: i8, divisor: i8) -> (i8, i8) {
    (dividend / divisor, dividend % divisor)
}
