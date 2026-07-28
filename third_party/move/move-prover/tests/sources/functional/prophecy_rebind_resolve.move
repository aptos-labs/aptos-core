// separate_baseline: path
// Copyright © Aptos Foundation
// A `&mut` binding ends not only at its death but also when its temp is
// redefined (shadowing) or when it exits the live range while a copy of it is
// still alive. Each such end resolves the binding's prophecy, chaining the
// pending obligation to the successor's. The tests pin: rebinding a reference
// local between two lenders, and the compiler's move-out/store-back pattern
// around a receiver call whose argument reads the enclosing struct.
module 0x42::prophecy_rebind_resolve {
    use std::vector;

    struct S has drop { a: u64, b: u64 }

    // `r`'s first binding ends at the rebind: the write through the first
    // binding must be visible in `a` afterwards, and the second binding's
    // write in `b`.
    fun rebind_local() {
        let s = S { a: 1, b: 2 };
        let r = &mut s.a;
        *r = 10;
        r = &mut s.b;
        *r = 20;
        spec {
            assert s.a == 10;
            assert s.b == 20;
            assert s.a == s.b; // error: intended failure guards against vacuity
        };
    }

    struct Bucket has store, drop { v: vector<u64> }

    fun set(self: &mut Bucket, i: u64, x: u64) {
        *vector::borrow_mut(&mut self.v, i) = x;
    }
    spec set {
        pragma opaque;
        aborts_if i >= len(self.v);
        ensures self.v == update(old(self.v), i, x);
    }

    struct Holder has drop { buckets: vector<Bucket>, size: u64 }

    // The receiver call's second argument reads `h.size`, so the compiler moves
    // `b` out, evaluates the arguments, and stores `b` back before the call —
    // one temp, two lifetimes whose prophecies must chain for the write to
    // reach the vector element.
    fun write_through_moved_receiver(h: &mut Holder): u64 {
        let b = vector::borrow_mut(&mut h.buckets, 0);
        b.set(h.size % 1, 7);
        *vector::borrow(&vector::borrow(&h.buckets, 0).v, 0)
    }
    spec write_through_moved_receiver {
        requires len(h.buckets) > 0 && len(h.buckets[0].v) > 0;
        aborts_if false;
        ensures result == 7;
    }
}
