// `object::exists_at` on a resource committed by an earlier step, at an address without
// one, after the resource is moved out in the same transaction, and after it is moved out
// and published again.

// RUN: publish
module 0x1::object {
    public native fun exists_at<T: key>(object: address): bool;
}
module 0x42::m {
    struct R has key { v: u64 }

    public fun publish(account: signer) {
        move_to(&account, R { v: 7 });
    }

    public fun committed(addr: address): bool {
        0x1::object::exists_at<R>(addr)
    }

    public fun after_move_from(addr: address): (bool, u64) acquires R {
        let R { v: value } = move_from<R>(addr);
        (0x1::object::exists_at<R>(addr), value)
    }

    public fun after_republish(account: signer, addr: address): (bool, bool) acquires R {
        let R { v: value } = move_from<R>(addr);
        let gone = 0x1::object::exists_at<R>(addr);
        move_to(&account, R { v: value + 1 });
        (gone, 0x1::object::exists_at<R>(addr))
    }
}

// RUN: execute 0x42::m::publish --args 0x42 --commit
// CHECK: writes: create 0x42::m::R at 0x42 0x0700000000000000

// RUN: execute 0x42::m::committed --args 0x42
// CHECK: results: true

// RUN: execute 0x42::m::committed --args 0x43
// CHECK: results: false

// RUN: execute 0x42::m::after_move_from --args 0x42
// CHECK: results: false, 7

// RUN: execute 0x42::m::after_republish --args 0x42, 0x42 --commit
// CHECK: results: false, true | writes: modify 0x42::m::R at 0x42 0x0800000000000000
