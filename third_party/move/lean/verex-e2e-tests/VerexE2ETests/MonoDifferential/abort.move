// RUN: publish
module 0x43::aborting {
    public fun abort_if_large(x: u64): u64 {
        assert!(x <= 100, 42);
        x
    }
}
// RUN: execute 0x43::aborting::abort_if_large --args 5
// RUN: execute 0x43::aborting::abort_if_large --args 200
