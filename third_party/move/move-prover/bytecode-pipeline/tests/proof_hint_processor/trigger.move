// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Pipeline tests for the trigger proof hint.
module 0x42::TestTrigger {

    // ==========================================================================
    // Basic trigger: append trigger to a postcondition quantifier.

    spec fun is_valid(x: u64): bool;

    fun test_trigger(x: u64): u64 {
        x
    }
    spec test_trigger {
        ensures forall y: u64: is_valid(y) ==> is_valid(y);

        proof {
            trigger forall y: u64 with {is_valid(y)};
        }
    }

    // ==========================================================================
    // Trigger on global state invariant quantifier.

    struct R has key { x: u64 }

    spec fun r_value(a: address): u64;

    spec module {
        invariant [global] forall a: address: global<R>(a).x > 0;
    }

    fun test_trigger_global(addr: address) {
        let r = &mut R[addr];
        r.x = r.x + 1;
    }
    spec test_trigger_global {
        proof {
            trigger forall a: address with {r_value(a)};
        }
    }

    // ==========================================================================
    // Generic trigger with type-dependent types.

    struct Box<T> has drop { val: T }

    spec fun box_value<T>(b: Box<T>): T;

    fun test_generic_trigger<T: drop>(b: Box<T>): Box<T> {
        b
    }
    spec test_generic_trigger {
        ensures forall x: Box<T>: box_value(x) == box_value(x);

        proof {
            trigger forall x: Box<T> with {box_value(x)};
        }
    }
}
