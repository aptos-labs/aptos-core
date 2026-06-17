// also_include_for: prophecy
module 0x42::prophecy_ginv {
    struct Counter has key, drop { val: u64 }

    // A module-level global invariant must be re-established after the resource is
    // mutated. Under the prophecy model the resource is written eagerly at the borrow,
    // so the invariant is asserted there.
    spec module {
        invariant forall a: address where exists<Counter>(a): global<Counter>(a).val <= 100;
    }

    fun set_ok(a: address) acquires Counter {
        let r = &mut Counter[a].val;
        *r = 50;
    }
    spec set_ok {
        aborts_if !exists<Counter>(a);
    }
}
