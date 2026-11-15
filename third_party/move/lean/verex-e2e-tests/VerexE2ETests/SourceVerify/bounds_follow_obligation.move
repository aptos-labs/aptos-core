// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A leaf states the bounds of the operations its goal reads, and of those in the
// facts connected to the goal through shared arithmetic atoms. An operation in a
// fact the goal does not connect to has no bound until a decider brings that
// fact in.
module 0x42::bounds_follow_obligation {
    struct Pair has key {
        low: u64,
        high: u64,
    }
    spec Pair {
        invariant high >> 1 <= 9223372036854775807;
        invariant low & 255 <= high || high < low;
    }

    // The goal reads the operation through a chain of linking equations.
    fun masked_plus_one(x: u64): u64 {
        let y = x & 255;
        let z = y + 1;
        z
    }
    spec masked_plus_one {
        ensures result <= 256;
    }

    // The goal reads a field; the stored invariants' operations read others.
    fun low_of(addr: address): u64 {
        Pair[addr].low
    }
    spec low_of {
        aborts_if !exists<Pair>(addr);
        ensures result == global<Pair>(addr).low;
    }

    // A failed check of an operation is the path's decision: its quotient's bound
    // refutes the path, though the clause owed reads neither.
    fun ratio(a: u64, b: u64, flag: bool): u64 {
        assert!(flag, 1);
        a / b
    }
    spec ratio {
        aborts_if !flag;
        aborts_if b == 0;
        ensures result == a / b;
    }

    // An instance's premise reads an operation the goal does not.
    spec fun spec_half_positive(x: u64): bool;
    spec module {
        axiom forall x: u64 where (x >> 1) < 10: spec_half_positive(x);
    }

    fun smallish(x: u64): u64 {
        x
    }
    spec smallish {
        requires x < 4;
        ensures spec_half_positive(result);
    }
}
