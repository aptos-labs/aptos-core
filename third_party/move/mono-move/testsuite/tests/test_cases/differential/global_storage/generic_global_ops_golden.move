// `move_from`, `borrow_global` and `borrow_global_mut` on `Box<u64>`, called from
// non-generic functions so the golden pins how each lowers. A `Box<u128>` with a different
// value sits at the same address, so an op on the wrong instantiation changes a result.

// RUN: publish --print(stackless,micro-ops)
module 0x42::gg {
    struct Box<T> has key { value: T }

    public fun put(account: signer, value: u64) { move_to(&account, Box<u64> { value }) }

    public fun put_wide(account: signer, value: u128) { move_to(&account, Box<u128> { value }) }

    public fun read(addr: address): u64 acquires Box { borrow_global<Box<u64>>(addr).value }

    public fun bump(addr: address, delta: u64) acquires Box {
        let boxed = borrow_global_mut<Box<u64>>(addr);
        boxed.value = boxed.value * 10 + delta;
    }

    public fun take(addr: address): u64 acquires Box {
        let Box { value } = move_from<Box<u64>>(addr);
        value
    }
}

// RUN: execute 0x42::gg::put_wide --args 0x42, 900 --commit
// CHECK: writes: create 0x42::gg::Box<u128> at 0x42 0x84030000000000000000000000000000

// Only `Box<u128>` exists yet.
// RUN: execute 0x42::gg::read --args 0x42
// CHECK-ERROR-PARITY

// RUN: execute 0x42::gg::put --args 0x42, 7 --commit
// CHECK: writes: create 0x42::gg::Box<u64> at 0x42 0x0700000000000000

// RUN: execute 0x42::gg::read --args 0x42
// CHECK: results: 7

// RUN: execute 0x42::gg::bump --args 0x42, 3 --commit
// CHECK: writes: modify 0x42::gg::Box<u64> at 0x42 0x4900000000000000

// RUN: execute 0x42::gg::read --args 0x42
// CHECK: results: 73

// RUN: execute 0x42::gg::take --args 0x42 --commit
// CHECK: results: 73 | writes: delete 0x42::gg::Box<u64> at 0x42

// RUN: execute 0x42::gg::take --args 0x42
// CHECK-ERROR-PARITY
