// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Tests behavioral predicates on opaque higher-order functions where the
// closure's spec accesses global memory. This exercises the evaluator path
// (runtime dispatch on function values) when memory parameters are involved.

module 0x42::opaque_behavioral_global_memory {
    struct Resource has key {
        value: u64,
    }

    fun read_value(addr: address): u64 acquires Resource {
        Resource[addr].value
    }
    spec read_value {
        ensures result == global<Resource>(addr).value;
        aborts_if !exists<Resource>(addr);
    }

    fun apply_opaque(f: |address| u64, x: address): u64 {
        f(x)
    }
    spec apply_opaque {
        pragma opaque;
        ensures ensures_of<f>(x, result);
    }

    fun test(addr: address): u64 acquires Resource {
        apply_opaque(|a| read_value(a) spec {
            ensures result == global<Resource>(a).value;
            aborts_if !exists<Resource>(a);
        }, addr)
    }
    spec test {
        ensures result == global<Resource>(addr).value;
    }
}
