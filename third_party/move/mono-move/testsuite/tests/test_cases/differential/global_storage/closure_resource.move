// A resource that holds a closure: one step commits it, and a later step reads it back and
// calls it. Also compares a closure with itself, directly and inside a struct, and runs a
// function whose closure comparison is never reached.

// RUN: publish
module 0x42::closure_resource {
    struct S has copy, drop { f: |u64|u64 has copy + drop, k: u64 }

    struct H has key { f: |u64|u64 has copy + drop + store }

    #[persistent]
    public fun add_k(offset: u64, value: u64): u64 { value + offset }

    public fun publish(account: signer, offset: u64) {
        move_to(&account, H { f: |value| add_k(offset, value) });
    }

    public fun call_stored(addr: address, arg: u64): u64 acquires H {
        let func = borrow_global<H>(addr).f;
        func(arg)
    }

    public fun self_eq(offset: u64): bool {
        let func: |u64|u64 has copy + drop = |value| add_k(offset, value);
        func == func
    }

    public fun struct_eq(offset: u64): bool {
        let wrapper = S { f: |value| add_k(offset, value), k: offset };
        wrapper == wrapper
    }

    // The equality is never executed.
    public fun unreached_eq(offset: u64): u64 {
        let func: |u64|u64 has copy + drop = |value| add_k(offset, value);
        if (offset == 0 && func == func) return 0;
        func(1)
    }
}

// RUN: execute 0x42::closure_resource::publish --args 0x42, 7 --commit
// CHECK: writes: create 0x42::closure_resource::H at 0x42 0x070100000000000000000000000000000000000000000000000000000000000000004210636c6f737572655f7265736f75726365056164645f6b000100000000000000020700000000000000

// RUN: execute 0x42::closure_resource::call_stored --args 0x42, 1
// CHECK: results: 8

// RUN: execute 0x42::closure_resource::self_eq --args 3
// CHECK: results: true

// RUN: execute 0x42::closure_resource::struct_eq --args 3
// CHECK: results: true

// RUN: execute 0x42::closure_resource::unreached_eq --args 3
// CHECK: results: 4
