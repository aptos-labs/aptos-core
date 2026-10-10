// RUN: publish
module 0x42::scalar {
    public fun add(x: u64, y: u64): u64 {
        x + y
    }

    public fun negate(x: i64): i64 {
        0 - x
    }
}
// RUN: execute 0x42::scalar::add --args 19, 23
// RUN: execute 0x42::scalar::negate --args 42
