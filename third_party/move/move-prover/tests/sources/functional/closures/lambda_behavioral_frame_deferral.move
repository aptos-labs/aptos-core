// The shape `big_ordered_map::iter_modify` has: opaque, unverified, no frame for the
// callback, and a specification written entirely in behavioral predicates over the
// callback plus the mutable value parameter. Behavioral predicates must not pull global
// memory into the callee's specification memory, or the call site could never supply the
// frame.
module 0x42::lambda_behavioral_frame_deferral {
    struct Counter has key { n: u64 }
    struct Cell has key { v: u64 }

    public fun apply(c: &mut Cell, f: |&mut u64| bool): bool {
        f(&mut c.v)
    }
    spec apply {
        pragma opaque;
        pragma verify = false;
        pragma aborts_if_is_partial = true;
        requires requires_of<f>(c.v);
        aborts_if aborts_of<f>(c.v);
        ensures ensures_of<f>(old(c.v), result, c.v);
    }

    fun bump(a: address) acquires Counter {
        Counter[a].n = Counter[a].n + 1;
    }
    spec bump {
        pragma opaque;
        aborts_if !exists<Counter>(a);
        aborts_if global<Counter>(a).n + 1 > MAX_U64;
        modifies global<Counter>(a);
        ensures global<Counter>(a).n == old(global<Counter>(a).n) + 1;
    }

    public fun caller(c: &mut Cell, a: address): bool acquires Counter {
        let f = |v| {
            *v = *v + 1;
            bump(a);
            true
        };
        apply(c, f)
    }

    /// Must fail: the callback writes `Counter` through the opaque call.
    public fun claims_unchanged(c: &mut Cell, a: address): bool acquires Counter {
        let f = |v| {
            *v = *v + 1;
            bump(a);
            true
        };
        apply(c, f)
    }
    spec claims_unchanged {
        pragma aborts_if_is_partial = true;
        ensures global<Counter>(a).n == old(global<Counter>(a).n);
    }
}
