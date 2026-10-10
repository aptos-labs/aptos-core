// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// One function is reached as a behavioral target under two different function
// types: as a partially applied closure stored in an enum variant
// (`|u64|u64`), and as the whole function value named directly in a
// specification (`|u64, u64|u64`). The behavioral spec functions of a target
// are named after the target alone, so the translation must declare them once
// for the whole file rather than once per function type.
module 0x42::behavioral_target_two_masks {

    enum State has key, copy, drop {
        Value(u64),
        Continuation(|u64|u64)
    }

    #[persistent]
    fun add(x: u64, y: u64): u64 {
        x + y
    }

    spec add {
        aborts_if x + y > MAX_U64;
        ensures result == x + y;
    }

    fun pending(x: u64): State {
        State::Continuation(|y| add(x, y))
    }

    spec module {
        /// Behavior of the closure stored in the enum variant.
        fun continuation_aborts(st: State, y: u64): bool {
            match (st) {
                Continuation(f) => aborts_of<f>(y),
                _ => false,
            }
        }

        /// Behavior of the same target named as a whole function value.
        fun add_aborts(x: u64, y: u64): bool {
            aborts_of<add>(x, y)
        }
    }

    spec pending {
        aborts_if false;
        ensures forall y: u64: continuation_aborts(result, y) == add_aborts(x, y);
    }
}
