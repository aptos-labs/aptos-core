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
        use 0x1::signer;
        pragma opaque;
        modifies Hook[signer::address_of(s)];
        ensures [inferred] result == (S1.. |~ result_of<old(Hook[signer::address_of(s)]).f>(x));
        ensures [inferred] {
            let a = signer::address_of(s);
            let b = old(Hook[signer::address_of(s)]);
            S1.. |~ publish<Hook>(a, b)
        };
        ensures [inferred] {
            let a = signer::address_of(s);
            ..S1 |~ remove<Hook>(a)
        };
        aborts_if [inferred] S1 |~ (exists<Hook>(signer::address_of(s)));
        aborts_if [inferred] !exists<Hook>(signer::address_of(s));
        aborts_if [inferred] S1 |~ (aborts_of<Hook[signer::address_of(s)].f>(x));
    }

    fun client(s: &signer, x: u64): u64 acquires Hook {
        run(s, x)
    }
    spec client(s: &signer, x: u64): u64 {
        use 0x1::signer;
        pragma opaque = true;
        modifies Hook[signer::address_of(s)];
        ensures [inferred] result == result_of<run>(s, x);
        aborts_if [inferred] aborts_of<run>(s, x);
    }

}
/*
Verification: Succeeded.
*/
