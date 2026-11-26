// How a step's net effect on a resource is reported: create then delete,
// delete then recreate, create then modify, modify then delete, and several
// resources in one step.

// RUN: publish
module 0x42::edges {
    struct R has key, drop { v: u64, data: vector<u8> }
    struct Q has key, drop { n: u8 }

    public fun create_then_delete(account: signer): bool {
        let addr = std::signer::address_of(&account);
        move_to(&account, R { v: 1, data: b"a" });
        let R { v: _, data: _ } = move_from<R>(addr);
        exists<R>(addr)
    }

    public fun create_then_modify(account: signer) {
        let addr = std::signer::address_of(&account);
        move_to(&account, R { v: 1, data: b"a" });
        let resource = borrow_global_mut<R>(addr);
        resource.v = 2;
        std::vector::push_back(&mut resource.data, 0x62);
    }

    public fun recreate(account: signer, value: u64) {
        let addr = std::signer::address_of(&account);
        let R { v: _, data } = move_from<R>(addr);
        move_to(&account, R { v: value, data });
    }

    public fun modify_then_delete(addr: address): u64 {
        borrow_global_mut<R>(addr).v = 99;
        let R { v: value, data: _ } = move_from<R>(addr);
        value
    }

    public fun several(account: signer) {
        let addr = std::signer::address_of(&account);
        move_to(&account, Q { n: 1 });
        if (exists<R>(addr)) { move_from<R>(addr); };
        move_to(&account, R { v: 5, data: vector[] });
        borrow_global_mut<Q>(addr).n = 2;
    }

    public fun delete_both(addr: address) {
        move_from<Q>(addr);
        move_from<R>(addr);
    }
}

// RUN: execute 0x42::edges::create_then_delete --args 0x42 --commit
// CHECK: results: false | writes: none

// RUN: execute 0x42::edges::create_then_modify --args 0x42 --commit
// CHECK: writes: create 0x42::edges::R at 0x42 0x0200000000000000026162

// RUN: execute 0x42::edges::recreate --args 0x42, 3 --commit
// CHECK: writes: modify 0x42::edges::R at 0x42 0x0300000000000000026162

// Deleted and recreated with the bytes it had: V1 reports a modify, MonoMove no write.
// RUN: execute 0x42::edges::recreate --args 0x42, 3 --commit
// CHECK-V1: writes: modify 0x42::edges::R at 0x42 0x0300000000000000026162
// CHECK-V2: writes: none

// RUN: execute 0x42::edges::modify_then_delete --args 0x42 --commit
// CHECK: results: 99 | writes: delete 0x42::edges::R at 0x42

// RUN: execute 0x42::edges::several --args 0x42 --commit
// CHECK: writes: create 0x42::edges::Q at 0x42 0x02; create 0x42::edges::R at 0x42 0x050000000000000000

// RUN: execute 0x42::edges::several --args 0x43 --commit
// CHECK: writes: create 0x42::edges::Q at 0x43 0x02; create 0x42::edges::R at 0x43 0x050000000000000000

// RUN: execute 0x42::edges::create_then_modify --args 0x44 --commit
// CHECK: writes: create 0x42::edges::R at 0x44 0x0200000000000000026162

// R exists here: deleted and recreated with new bytes, reported as a modify.
// RUN: execute 0x42::edges::several --args 0x44 --commit
// CHECK: writes: create 0x42::edges::Q at 0x44 0x02; modify 0x42::edges::R at 0x44 0x050000000000000000

// RUN: execute 0x42::edges::delete_both --args 0x42 --commit
// CHECK: writes: delete 0x42::edges::Q at 0x42; delete 0x42::edges::R at 0x42
