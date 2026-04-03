//# publish
module 0xc0ffee::m {
    // Exclusive range: a..b matches a <= x < b
    public fun test_exclusive(x: u64): u64 {
        match (x) {
            1..10 => 1,
            _ => 0,
        }
    }

    // Inclusive range: a..=b matches a <= x <= b
    public fun test_inclusive(x: u64): u64 {
        match (x) {
            1..=10 => 1,
            _ => 0,
        }
    }

    // Open-end range: a.. matches x >= a
    public fun test_open_end(x: u64): u64 {
        match (x) {
            5.. => 1,
            _ => 0,
        }
    }

    // Open-start exclusive: ..b matches x < b
    public fun test_open_start(x: u64): u64 {
        match (x) {
            ..10 => 1,
            _ => 0,
        }
    }

    // Open-start inclusive: ..=b matches x <= b
    public fun test_open_start_inclusive(x: u64): u64 {
        match (x) {
            ..=10 => 1,
            _ => 0,
        }
    }
}

//# run 0xc0ffee::m::test_exclusive --args 0u64

//# run 0xc0ffee::m::test_exclusive --args 1u64

//# run 0xc0ffee::m::test_exclusive --args 5u64

//# run 0xc0ffee::m::test_exclusive --args 9u64

//# run 0xc0ffee::m::test_exclusive --args 10u64

//# run 0xc0ffee::m::test_inclusive --args 0u64

//# run 0xc0ffee::m::test_inclusive --args 1u64

//# run 0xc0ffee::m::test_inclusive --args 5u64

//# run 0xc0ffee::m::test_inclusive --args 10u64

//# run 0xc0ffee::m::test_inclusive --args 11u64

//# run 0xc0ffee::m::test_open_end --args 4u64

//# run 0xc0ffee::m::test_open_end --args 5u64

//# run 0xc0ffee::m::test_open_end --args 100u64

//# run 0xc0ffee::m::test_open_start --args 0u64

//# run 0xc0ffee::m::test_open_start --args 9u64

//# run 0xc0ffee::m::test_open_start --args 10u64

//# run 0xc0ffee::m::test_open_start_inclusive --args 0u64

//# run 0xc0ffee::m::test_open_start_inclusive --args 10u64

//# run 0xc0ffee::m::test_open_start_inclusive --args 11u64
