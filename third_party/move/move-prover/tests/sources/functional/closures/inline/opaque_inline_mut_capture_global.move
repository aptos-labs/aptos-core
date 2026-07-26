// Tests for lambdas capturing mutable references rooted in global storage.
// The captured mutation is carried by the closure value across the retained
// call and written back into the resource memory when the closure dies, where
// global invariants and modifies checking attach as for any memory update.
module 0x42::opaque_inline_mut_capture_global {
    struct R has key {
        value: u64,
    }

    struct G<T: store> has key {
        t: T,
    }

    spec module {
        invariant forall a: address where exists<R>(a): global<R>(a).value < 1000;
    }

    inline fun modify(f: |u64|) {
        f(1)
    }
    spec modify {
        pragma opaque;
        requires !aborts_of<f>(1);
        aborts_if false;
        ensures ensures_of<f>(1);
    }

    /// Test: capture of a mutable reference into global storage; the module
    /// invariant is checked when the mutation is written back.
    fun bump(a: address) {
        let r = &mut R[a].value;
        modify(|y| *r = *r + y spec {
            aborts_if r + y > MAX_U64;
            ensures r == old(r) + y;
        });
    }
    spec bump {
        requires global<R>(a).value < 999;
        aborts_if !exists<R>(a);
        ensures global<R>(a).value == old(global<R>(a).value) + 1;
    }

    /// Test: capture rooted in a generic resource instance.
    fun bump_generic(a: address) {
        let r = &mut G<u64>[a].t;
        modify(|y| *r = *r + y spec {
            aborts_if r + y > MAX_U64;
            ensures r == old(r) + y;
        });
    }
    spec bump_generic {
        requires global<G<u64>>(a).t < MAX_U64;
        aborts_if !exists<G<u64>>(a);
        ensures global<G<u64>>(a).t == old(global<G<u64>>(a).t) + 1;
    }
}
