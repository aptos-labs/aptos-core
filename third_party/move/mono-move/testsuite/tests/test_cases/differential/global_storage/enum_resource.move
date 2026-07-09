// RUN: publish
module 0x42::enum_globals {
    enum R has key, drop {
        A { x: u64 },
        B { x: u64, y: u64 },
    }

    // move_to an enum resource, then read a variant field via borrow_global.
    public fun publish_and_read(s: signer, a: address, x: u64): u64 {
        move_to(&s, R::A { x });
        let r = borrow_global<R>(a);
        match (r) {
            A { x } => *x,
            B { x, y } => *x + *y,
        }
    }

    // move_to then move_from, summing the moved-out variant fields.
    public fun publish_and_take(s: signer, a: address, x: u64): u64 {
        move_to(&s, R::B { x, y: x + 1 });
        match (move_from<R>(a)) {
            A { x } => x,
            B { x, y } => x + y,
        }
    }

    // Force a collection while the enum resource lives in storage.
    public fun publish_gc_read(s: signer, a: address, x: u64): u64 {
        move_to(&s, R::B { x, y: x + 1 });
        0x0::test_utils::force_gc();
        let r = borrow_global<R>(a);
        let from_borrow = match (r) {
            A { x } => *x,
            B { x, y } => *x + *y,
        };
        0x0::test_utils::force_gc();
        let from_move = match (move_from<R>(a)) {
            A { x } => x,
            B { x, y } => x + y,
        };
        from_borrow * 1000 + from_move
    }
}

// RUN: execute 0x42::enum_globals::publish_and_read --args 0x42, 0x42, 100
// CHECK: results: 100

// RUN: execute 0x42::enum_globals::publish_and_take --args 0x7, 0x7, 55
// CHECK: results: 111

// x+(x+1) = 89+90 = 179 for both the borrow and the move-out.
// RUN: execute 0x42::enum_globals::publish_gc_read --args 0x9, 0x9, 89
// CHECK: results: 179179
