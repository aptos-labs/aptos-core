// Behavioral predicates over spec-less lambdas resolved by the source-level
// WP analysis: modular summaries for called functions, imperative bodies,
// and global state effects anchored at the application site.
module 0x42::bp_inline_derive {
    use std::signer;

    // ===== Lambda calling a function with a spec: modular summary =====

    fun helper(y: u64): u64 {
        y + 1
    }
    spec helper {
        pragma opaque;
        aborts_if y == MAX_U64;
        ensures result == y + 1;
    }

    inline fun apply(f: |u64| u64, x: u64): u64 {
        let r = f(x);
        spec {
            assert !aborts_of<f>(x);
            assert ensures_of<f>(x, r);
        };
        r
    }

    fun call_helper(x: u64): u64 {
        apply(|y| helper(y), x)
    }
    spec call_helper {
        requires x < MAX_U64;
        ensures result == x + 1;
    }

    // ===== Lambda with global state effects: the derived two-state spec =====
    // ===== is anchored at the unique application site                   =====

    struct R has key { v: u64 }

    inline fun apply_addr(f: |address|, a: address) {
        f(a);
        spec {
            assert ensures_of<f>(a);
            assert !aborts_of<f>(a);
        };
    }

    fun publish_one(s: &signer) {
        let addr = signer::address_of(s);
        apply_addr(|_a| move_to(s, R { v: 1 }), addr);
    }
    spec publish_one {
        let addr = signer::address_of(s);
        aborts_if exists<R>(addr);
        ensures exists<R>(addr);
        ensures R[addr].v == 1;
    }

    // ===== result_of over an imperative body =====

    inline fun compute(f: |u64| u64, x: u64): u64 {
        let r = f(x);
        spec {
            assert r == result_of<f>(x);
        };
        r
    }

    fun compute_stepwise(x: u64): u64 {
        compute(|y| {
            let doubled = y * 2;
            if (doubled > 100) {
                doubled - 100
            } else {
                doubled
            }
        }, x)
    }
    spec compute_stepwise {
        requires x <= 1000;
        ensures result == (if (x * 2 > 100) x * 2 - 100 else x * 2);
    }
}
