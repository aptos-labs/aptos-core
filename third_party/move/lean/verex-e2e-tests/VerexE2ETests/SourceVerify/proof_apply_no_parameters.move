// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// A step of a function without parameters, whose only native values are read
/// from the store.
module 0x42::proof_apply_no_parameters {
    struct Config has key {
        rate: u64,
    }

    // The obligation's locals are callee results and field reads, at the
    // carriers of the module's family.
    fun halved(): u64 {
        let rate = Config[@0x1].rate;
        rate / 2 * 2 / 2
    }
    spec halved {
        pragma verify = true;
        requires exists<Config>(@0x1);
        aborts_if false;
        ensures result <= Config[@0x1].rate;
    }
}
