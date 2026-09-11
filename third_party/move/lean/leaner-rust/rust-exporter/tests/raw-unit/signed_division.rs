// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

pub fn signed_div_rem(dividend: i8, divisor: i8) -> (i8, i8) {
    (dividend / divisor, dividend % divisor)
}
