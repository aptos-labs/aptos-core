// Expansion-entry snapshots do not require `copy`.
module 0x42::vector_hofs_noncopy_receiver {
    use std::vector;

    struct NoCopy has store, drop { x: u64 }
    struct Holder has key { v: vector<NoCopy> }

    inline fun for_each_mut<T>(v: &mut vector<T>, f: |&mut T|) {
        let i = 0;
        let n = vector::length(v);
        while (i < n) {
            f(vector::borrow_mut(v, i));
            i = i + 1;
        } spec {
            invariant i <= n;
            invariant len(v) == len(old(v));
            invariant forall j in 0..i: ensures_of<f>(old(v)[j], v[j]);
            invariant forall j in i..n: v[j] == old(v)[j];
        };
    }

    fun clear_all(h: &mut Holder) {
        for_each_mut(&mut h.v, |e| {
            e.x = 0
        } spec {
            aborts_if false;
            ensures e.x == 0;
        });
    }
    spec clear_all {
        aborts_if false;
        ensures forall i in 0..len(h.v): h.v[i].x == 0;
    }
}
