module 0x42::address_constants {
    struct R has key {
        value: u64,
    }

    // Regression: inference must substitute address literals loaded into
    // stackless-bytecode temporaries before emitting source-level conditions.
    fun value_at_fixed_address(): u64 acquires R {
        R[@0x42].value
    }
    spec value_at_fixed_address(): u64 {
        pragma opaque = true;
        ensures [inferred] result == R[@0x42].value;
        aborts_if [inferred] !exists<R>(@0x42);
    }

}
/*
Verification: Succeeded.
*/
