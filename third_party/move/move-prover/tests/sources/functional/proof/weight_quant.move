// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// flag: --language-version=2.4

// Positive test: `[weight = N]` on general `forall` / `exists` expressions
// Each function below verifies cleanly under the new general-quant syntax.

module 0x42::proof_weight_quant {

    // 1. `forall` with `[weight = N]` in `ensures`.
    fun id(x: u64): u64 {
        x
    }
    spec id {
        ensures forall y: u64 [weight = 10]: y == y;
        ensures result == x;
    }

    // 2. `exists` with `[weight = N]` in `ensures`.
    fun zero(): u64 {
        0
    }
    spec zero {
        ensures exists y: u64 [weight = 15]: y == 0;
        ensures result == 0;
    }

    // 3. `forall` with both a trigger group and `[weight = N]`.
    spec fun p(x: num): bool { x >= 0 }
    fun trivial_pos(x: u64): u64 {
        x
    }
    spec trivial_pos {
        ensures forall y: num {p(y)} [weight = 20]:
            y >= 0 ==> p(y);
    }

    // 4. `forall` with `[weight = 0]` — explicitly the SMT default; accepted.
    fun id_again(x: u64): u64 {
        x
    }
    spec id_again {
        ensures forall y: u64 [weight = 0]: y == y;
        ensures result == x;
    }
}
