// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// flag: --language-version=2.4

// Negative test for the `[weight = N]` annotation: a negative literal must
// be rejected by the parser because weight is `u32`.
//
// (Other syntactic errors — `[weight = abc]`, `[weight = 1.5]`, missing `=`,
// missing `]`, etc. — also fail to parse but the parser stops at the first
// error, so each malformed form needs its own test if exercised.)

module 0x42::proof_weight_neg {

    spec fun id_num(n: num): num [weight = -1] {
        if (n == 0) { 0 } else { id_num(n - 1) + 1 }
    }
}
