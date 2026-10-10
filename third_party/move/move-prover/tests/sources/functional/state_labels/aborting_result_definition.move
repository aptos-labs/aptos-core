// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// A result projection cannot assume an aborting callee's false postcondition.
module 0x42::aborting_result_definition {
    struct R has key { value: u64 }

    fun fail(): u64 { abort 1 }
    spec fail {
        pragma opaque;
        aborts_if true;
        ensures false;
    }

    fun caller(addr: address): u64 { let _ = addr; fail() }
    spec caller {
        // Negative: the function always aborts.
        aborts_if false;
        ensures {
            let value = ..S |~ result_of<fail>();
            (S |~ exists<R>(addr)) && result == value
        };
    }

    fun missing(addr: address): u64 acquires R {
        let R { value } = move_from<R>(addr);
        value
    }
    spec missing {
        // Negative: the intermediate removal cannot assume that R existed
        // on the path where move_from aborted because it was absent.
        aborts_if false;
        ensures (..S |~ remove<R>(addr)) && !(S |~ exists<R>(addr));
    }
}
