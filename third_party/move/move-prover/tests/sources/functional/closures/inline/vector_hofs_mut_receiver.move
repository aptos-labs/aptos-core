// Tests the dual-form `old(..)` invariants of the `_mut` vector HOFs over
// receivers which are not entry-defined values of the enclosing function: a
// field projection `&mut s.v` and a `borrow_global_mut(..)` result. The
// invariants' `old(v)` anchors at the *expansion entry* — the receiver's
// value is recorded in a snapshot binding when the call is expanded — so
// the base case holds for receivers constructed at the call site. See
// `vector_hofs_for_each.move` for the plain parameter receivers.
module 0x42::vector_hofs_mut_receiver {
    use std::vector;

    inline fun for_each_mut<T>(v: &mut vector<T>, f: |&mut T|) {
        let i = 0;
        let n = vector::length(v);
        while (i < n) {
            f(vector::borrow_mut(v, i));
            i = i + 1;
        } spec {
            invariant i <= n;
            invariant len(v) == len(old(v));
            invariant n == len(v);
            invariant forall j in 0..i: ensures_of<f>(old(v)[j], v[j]);
            invariant forall j in 0..i: !aborts_of<f>(old(v)[j]);
            invariant forall j in i..n: v[j] == old(v)[j];
        };
    }

    inline fun enumerate_mut<T>(v: &mut vector<T>, f: |u64, &mut T|) {
        let i = 0;
        let n = vector::length(v);
        while (i < n) {
            f(i, vector::borrow_mut(v, i));
            i = i + 1;
        } spec {
            invariant i <= n;
            invariant len(v) == len(old(v));
            invariant n == len(v);
            invariant forall j in 0..i: ensures_of<f>(j, old(v)[j], v[j]);
            invariant forall j in 0..i: !aborts_of<f>(j, old(v)[j]);
            invariant forall j in i..n: v[j] == old(v)[j];
        };
    }

    struct S has copy, drop { v: vector<u64> }
    struct R has key { v: vector<u64> }

    /// The receiver is a field projection of a parameter.
    fun bump_field(s: &mut S) {
        for_each_mut(&mut s.v, |e| {
            *e = *e + 1
        } spec {
            aborts_if e == MAX_U64;
            ensures e == old(e) + 1;
        });
    }
    spec bump_field {
        requires forall i in 0..len(s.v): s.v[i] < MAX_U64;
        aborts_if false;
        ensures forall i in 0..len(s.v): s.v[i] == old(s.v)[i] + 1;
    }

    /// The receiver lives in global memory.
    fun bump_resource(a: address) acquires R {
        let r = borrow_global_mut<R>(a);
        for_each_mut(&mut r.v, |e| {
            *e = *e + 1
        } spec {
            aborts_if e == MAX_U64;
            ensures e == old(e) + 1;
        });
    }
    spec bump_resource {
        requires forall i in 0..len(global<R>(a).v): global<R>(a).v[i] < MAX_U64;
        aborts_if !exists<R>(a);
        ensures forall i in 0..len(global<R>(a).v):
            global<R>(a).v[i] == old(global<R>(a).v)[i] + 1;
    }

    /// `enumerate_mut` over a field projection.
    fun add_index(s: &mut S) {
        enumerate_mut(&mut s.v, |i, e| {
            *e = *e + i
        } spec {
            aborts_if e + i > MAX_U64;
            ensures e == old(e) + i;
        });
    }
    spec add_index {
        requires forall i in 0..len(s.v): s.v[i] + i <= MAX_U64;
        aborts_if false;
        ensures forall i in 0..len(s.v): s.v[i] == old(s.v)[i] + i;
    }

    /// Canary: a wrong caller postcondition must still be reported.
    fun bump_field_wrong(s: &mut S) {
        for_each_mut(&mut s.v, |e| {
            *e = *e + 1
        } spec {
            aborts_if e == MAX_U64;
            ensures e == old(e) + 1;
        });
    }
    spec bump_field_wrong {
        requires forall i in 0..len(s.v): s.v[i] < MAX_U64;
        aborts_if false;
        ensures forall i in 0..len(s.v): s.v[i] == old(s.v)[i] + 2; // error: off by one
    }
}
