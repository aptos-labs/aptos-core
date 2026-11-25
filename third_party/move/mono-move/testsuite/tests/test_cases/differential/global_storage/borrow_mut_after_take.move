// Borrowing a resource after move_from took it in the same step: borrow_global_mut when
// the resource was published in that step, and borrow_global_mut and borrow_global when
// it was committed by an earlier step. Each fails with MISSING_DATA.

// RUN: publish
module 0x42::after_take {
    struct R has key, drop { v: u64 }

    public fun publish(account: signer, value: u64) {
        move_to(&account, R { v: value });
    }

    public fun put_take_borrow_mut(account: signer, addr: address): u64 acquires R {
        move_to(&account, R { v: 1 });
        let R { v: _ } = move_from<R>(addr);
        borrow_global_mut<R>(addr).v = 2;
        0
    }

    public fun take_borrow_mut(addr: address): u64 acquires R {
        let R { v: value } = move_from<R>(addr);
        borrow_global_mut<R>(addr).v = value + 1;
        value
    }

    public fun take_borrow(addr: address): u64 acquires R {
        let R { v: _ } = move_from<R>(addr);
        borrow_global<R>(addr).v
    }
}

// RUN: execute 0x42::after_take::put_take_borrow_mut --args 0x42, 0x42
// CHECK-V1-SUBSTR: MISSING_DATA
// CHECK-V2-SUBSTR: BorrowGlobalMut: resource does not exist
// CHECK-ERROR-PARITY

// RUN: execute 0x42::after_take::publish --args 0x43, 5 --commit
// CHECK: writes: create 0x42::after_take::R at 0x43 0x0500000000000000

// RUN: execute 0x42::after_take::take_borrow_mut --args 0x43
// CHECK-V1-SUBSTR: MISSING_DATA
// CHECK-V2-SUBSTR: BorrowGlobalMut: resource does not exist
// CHECK-ERROR-PARITY

// RUN: execute 0x42::after_take::take_borrow --args 0x43
// CHECK-V1-SUBSTR: MISSING_DATA
// CHECK-V2-SUBSTR: BorrowGlobal: resource does not exist
// CHECK-ERROR-PARITY
