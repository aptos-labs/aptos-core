// An opaque function running a function value nothing bounds after a state change, named by
// `result_of`/`aborts_of` in a caller's specification. Inference labels the predicates over the
// invoked value with the invocation's pre-state, the state after the removal; inside the
// function's abort evaluator (a Boogie function) a memory that label does not modify is the
// evaluator's own parameter or a bound state, never `old(..)`, which Boogie rejects in a
// function.
module 0x66::ia {
    use std::signer;

    struct Hook has key { f: |u64| u64 has copy + drop + store }
    struct R has key { v: u64 }

    fun run(s: &signer, x: u64): u64 acquires Hook {
        let h = move_from<Hook>(signer::address_of(s));
        let r = (h.f)(x);
        move_to(s, h);
        r
    }
    spec run {
        pragma opaque;
    }

    fun client(s: &signer, x: u64): u64 acquires Hook {
        run(s, x)
    }
}
