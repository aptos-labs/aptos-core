// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A mutable borrow of a vector element reads its index local again where the
// loan dies, while the reference denotes the element chosen at the borrow. A
// write to that index local while the loan is live would resolve the reference
// at another element, so such a function is refused; one that writes the index
// only after the loan has died — `bit_vector`'s loop — is carried and verifies.
// Exercised through the package path, so the refusal recorded by the borrow
// analysis survives namespace linking and relocation.

module 0x42::index_write_under_loan {
    use std::vector;

    /// Refused: the index local is written while the element borrow is live
    /// (call form of the borrow).
    public fun write_under_loan_call(v: &mut vector<u64>): u64 {
        let i = 0;
        let r = vector::borrow_mut(v, i);
        i = i + 1;
        *r = 7;
        i
    }
    spec write_under_loan_call {
        ensures false; // error: refused, the index local is written under a live loan
    }

    /// Refused: the same with index syntax.
    public fun write_under_loan_index(v: &mut vector<u64>): u64 {
        let i = 0;
        let r = &mut v[i];
        i = i + 1;
        *r = 7;
        i
    }
    spec write_under_loan_index {
        ensures false; // error: refused, the index local is written under a live loan
    }

    /// Refused: a destructuring assignment writes the index local while the
    /// element borrow is live.
    public fun write_under_loan_destructure(v: &mut vector<u64>): u64 {
        let i = 0;
        let j = 0;
        let r = vector::borrow_mut(v, i);
        (i, j) = (1, 1);
        *r = 7;
        i + j
    }
    spec write_under_loan_destructure {
        ensures false; // error: refused, the index local is written under a live loan
    }

    /// Not refused: moving the index out leaves the value the loan's death
    /// reads unchanged; the false postcondition is rejected as usual.
    public fun move_under_loan(v: &mut vector<u64>): u64 {
        let i = 0;
        let r = vector::borrow_mut(v, i);
        let k = move i;
        *r = 7;
        k
    }
    spec move_under_loan {
        ensures false; // error: the postcondition does not hold
    }

    /// Must verify: `bit_vector`'s loop — borrow the element, write through it,
    /// increment the index after the loan has died.
    public fun clear_all(v: &mut vector<bool>) {
        let n = vector::length(v);
        let i = 0;
        while ({
            spec {
                invariant n == len(v);
                invariant i <= n;
            };
            i < n
        }) {
            let elem = vector::borrow_mut(v, i);
            *elem = false;
            i = i + 1;
        };
    }

    /// Must verify: the index written after the loan has died, straight-line.
    public fun write_after_loan(v: &mut vector<u64>): u64 {
        let i = 0;
        let r = &mut v[i];
        *r = 7;
        i = i + 1;
        i
    }
    spec write_after_loan {
        requires len(v) > 0;
        ensures result == 1;
    }
}
