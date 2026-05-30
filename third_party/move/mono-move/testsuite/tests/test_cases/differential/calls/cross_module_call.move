// RUN: publish
module 0x1::foo {
    public fun add_one(x: u64): u64 { x + 1 }
}

module 0x1::bar {
    public fun main(x: u64): u64 { 0x1::foo::add_one(x) }
}

// RUN: execute 0x1::bar::main --args 41
// CHECK: results: 42
