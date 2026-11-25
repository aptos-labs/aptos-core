// Resource keys that differ only in the order of phantom type arguments:
// Tag<A, B> and Tag<B, A> have identical layouts and live side by side at one
// address. Writes are pinned on each mutating step.

// RUN: publish
module 0x42::porder {
    struct A {}
    struct B {}
    struct Tag<phantom X, phantom Y> has key, drop { v: u64 }

    public fun put_both(account: signer) {
        move_to(&account, Tag<A, B> { v: 11 });
        move_to(&account, Tag<B, A> { v: 22 });
    }

    public fun bump_second(addr: address) { borrow_global_mut<Tag<B, A>>(addr).v = 23 }

    public fun take_first(addr: address): (u64, u64, bool, bool) {
        let Tag { v: first } = move_from<Tag<A, B>>(addr);
        (first, borrow_global<Tag<B, A>>(addr).v, exists<Tag<A, B>>(addr), exists<Tag<B, A>>(addr))
    }
}

// RUN: execute 0x42::porder::put_both --args 0x42 --commit
// CHECK: writes: create 0x42::porder::Tag<0x42::porder::A, 0x42::porder::B> at 0x42 0x0b00000000000000; create 0x42::porder::Tag<0x42::porder::B, 0x42::porder::A> at 0x42 0x1600000000000000

// RUN: execute 0x42::porder::bump_second --args 0x42 --commit
// CHECK: writes: modify 0x42::porder::Tag<0x42::porder::B, 0x42::porder::A> at 0x42 0x1700000000000000

// RUN: execute 0x42::porder::take_first --args 0x42 --commit
// CHECK: results: 11, 23, false, true | writes: delete 0x42::porder::Tag<0x42::porder::A, 0x42::porder::B> at 0x42
