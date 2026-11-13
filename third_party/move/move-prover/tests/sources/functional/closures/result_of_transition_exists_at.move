// Copyright © Aptos Foundation

// flag: --check-inconsistency
// flag: --unconditional-abort-as-inconsistency

// `object::spec_exists_at<T>` declares no memory but translates to a
// post-state `exists<T>`; an uninterpreted spec function never enters the
// unguarded pre-state-only relation of the carrier.
module 0x2::object {
    spec module {
        fun spec_exists_at<T: key>(object: address): bool;
    }
}

module 0x42::result_of_transition_exists_at {
    use 0x2::object;

    struct R has key { v: u64 }

    fun take(a: address): u64 acquires R {
        let R { v } = move_from<R>(a);
        v
    }
    spec take {
        pragma opaque;
        aborts_if !exists<R>(a);
        ensures !object::spec_exists_at<R>(a);
        modifies global<R>(a);
    }

    // Canary: must not verify (the carrier is unconstrained without a call).
    fun bad(a: address): u64 {
        0
    }
    spec bad {
        requires exists<R>(a);
        ensures result_of<take>(a) == 5;
    }

    // Canary: must not verify.
    fun caller_false(a: address): u64 acquires R {
        take(a)
    }
    spec caller_false {
        pragma aborts_if_is_partial;
        requires exists<R>(a);
        aborts_if result_of<take>(a) > MAX_U64;
        ensures false;
    }
}
