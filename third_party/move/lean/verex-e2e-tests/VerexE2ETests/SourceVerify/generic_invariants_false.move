// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A generic module invariant holds at the instantiations of the memory a
// function uses, as the Move Prover monomorphizes it: a write of `Box<T>`
// owes it at `T`, one of `Box<u64>` at `u64`, and a read assumes it.
module 0x42::generic_invariants_false {
    struct Box<T: store> has key { value: T, count: u64 }

    spec module {
        invariant<T> forall a: address where exists<Box<T>>(a): global<Box<T>>(a).count > 0;
    }

    public fun publish<T: store>(account: &signer, value: T) {
        move_to(account, Box { value, count: 1 });
    }

    public fun publish_zero<T: store>(account: &signer, value: T) {
        move_to(account, Box { value, count: 0 }); // error: the count is 0
    }

    public fun publish_u64(account: &signer) {
        move_to(account, Box<u64> { value: 7, count: 2 });
    }

    public fun reset_u64(a: address) acquires Box {
        borrow_global_mut<Box<u64>>(a).count = 0; // error: the count is 0
    }

    public fun count_of<T: store>(a: address): u64 acquires Box {
        borrow_global<Box<T>>(a).count
    }
    spec count_of {
        aborts_if !exists<Box<T>>(a);
        ensures result > 0;
    }
}
