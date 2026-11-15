// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A local named `proof` before a statement on the next line: `proof` starts a
// proof step only with the step on its line.
module 0x42::local_named_proof {
    enum Proof has drop { V1 { sigma: u64 } }

    fun next(x: u64): u64 { x % 2 }

    fun destructure(proof: Proof): u64 {
        let Proof::V1 { sigma } = proof;
        let reduced = next(sigma);
        reduced
    }
    spec destructure {
        aborts_if false;
        ensures result == proof.sigma % 2;
    }
}
