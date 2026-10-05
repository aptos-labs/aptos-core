// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

module 0x42::behavioral_derived_opaque {
    fun abstract_value(x: u64): u64 { x + 1 }
    spec abstract_value {
        pragma opaque;
        aborts_if [abstract] false;
        aborts_if [concrete] x == MAX_U64;
        ensures [abstract] result == x;
        ensures [concrete] result == x + 1;
    }

    fun wrapper(x: u64): u64 { abstract_value(x) }
    fun outer(x: u64): u64 { wrapper(x) }

    fun check() {}
    spec check {
        // Both the direct and transitive summaries must use the opaque
        // callee's caller-visible contract, not its concrete implementation.
        ensures result_of<wrapper>(1) == 1;
        ensures result_of<outer>(1) == 1;
    }
}
