// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// `pragma intrinsic` on a function the verifier has no meaning for leaves
/// the function its body, verified as any other, as the Move Prover reads
/// the pragma.
module 0x42::intrinsic_errors {
    fun one(): u64 {
        1
    }
    spec one {
        pragma intrinsic;
        ensures result == 2; // error: the body returns 1
    }

    /// Opaque as well: callers rely on its contract by the author's choice,
    /// and the body is not verified against it.
    fun two(): u64 {
        3
    }
    spec two {
        pragma intrinsic;
        pragma opaque;
        ensures result == 2;
    }
}
