// RUN: publish
module 0x46::batching {
    public fun add(x: u64, y: u64): u64 {
        x + y
    }
}
// RUN: execute 0x46::batching::add --args 1, 2
// RUN: execute 0x46::batching::add --args 1, 2
// RUN: execute 0x46::batching::add --args 40, 2
