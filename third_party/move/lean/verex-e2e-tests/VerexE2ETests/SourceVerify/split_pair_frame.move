// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A match on a pair of a resource and an argument: the split reduces its
// constructor reading at the matched values, and the join carries the
// component equations to the closure invoke of `process`. `plain` is the
// pair match with no closure invoke.
module 0x43::split_pair_frame {
    use 0x1::signer::address_of;

    enum Input { Number(u64), Other }

    enum State has key, copy, drop {
        Empty,
        Value(u64),
        Continuation(|u64|u64 has copy+drop+store),
    }

    fun process(s: &signer, input: Input) acquires State {
        let addr = address_of(s);
        match ((move_from<State>(addr), input)) {
            (Empty, Number(x)) => move_to(s, State::Value(x)),
            (Continuation(f), Number(x)) => move_to(s, State::Value(f(x))),
            (_, _) => abort 1
        }
    }
    spec process(s: &signer, input: Input) {
        use 0x1::signer;
        pragma opaque = true, aborts_if_is_partial = true;
        modifies State[signer::address_of(s)];
        ensures [inferred] { let a = signer::address_of(s); ..S1 |~ remove<State>(a) };
        ensures [inferred] (old(State[signer::address_of(s)]) is Empty) && (input is Number) ==> {
            let a = signer::address_of(s);
            S1.. |~ publish<State>(a, State::Value(input.0))
        };
    }

    fun plain(s: &signer, input: Input) acquires State {
        let addr = address_of(s);
        match ((move_from<State>(addr), input)) {
            (Value(y), Number(x)) => move_to(s, State::Value(x + y)),
            (_, _) => abort 1
        }
    }
}
