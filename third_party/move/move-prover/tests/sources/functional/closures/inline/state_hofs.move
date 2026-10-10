// State-modifying lambdas in loop-based inline higher-order functions.
// Behavioral predicates over such a lambda are resolved in loop invariants
// by fixing the lambda spec's pre-state to function entry (the invariant's
// `old(..)` scope) and its post-state to the current state; the derived
// whole-memory effect operations are projected to per-element point facts
// (`exists<R>(a) && global<R>(a) == v` resp. `!exists<R>(a)`). The dropped
// frame is recovered by `unchanged_of<f>(x)`: the memory the lambda may
// write at arguments `x` — its derived modifies footprint — is unchanged
// relative to entry (`true` for pure lambdas). Canonical pattern:
//
//     invariant i <= n;
//     invariant forall j in 0..i: ensures_of<f>(v[j]);   // point facts
//     invariant forall j in 0..i: !aborts_of<f>(v[j]);   // reads at entry
//     invariant forall j in i..n: unchanged_of<f>(v[j]); // suffix frame
//     invariant forall x: address:
//         (forall j in 0..i: x != v[j]) ==> unchanged_of<f>(x); // outer frame
//
// The substituted conditions are asserted only, never assumed, so the
// resolution is sound for any lambda; disjointness of the per-element
// footprints (the callers' distinctness requires) is what makes the
// invariants provable.
module 0x42::state_hofs {
    use std::vector;

    struct R has key { v: u64 }

    inline fun for_each_addr(v: &vector<address>, f: |address|) {
        let i = 0;
        let n = vector::length(v);
        while (i < n) {
            f(*vector::borrow(v, i));
            i = i + 1;
        } spec {
            invariant i <= n;
            invariant forall j in 0..i: ensures_of<f>(v[j]);
            invariant forall j in 0..i: !aborts_of<f>(v[j]);
            invariant forall j in i..n: unchanged_of<f>(v[j]);
            invariant forall x: address: (forall j in 0..i: x != v[j]) ==> unchanged_of<f>(x);
        };
    }

    // ===== resource update: exact abort condition, per-element ensures, =====
    // ===== and preservation of every untouched address                  =====

    fun bump_all(v: &vector<address>) {
        for_each_addr(v, |a| {
            let r = &mut R[a];
            r.v = r.v + 1;
        });
    }
    spec bump_all {
        requires forall i in 0..len(v), j in 0..len(v): i != j ==> v[i] != v[j];
        requires forall i in 0..len(v): exists<R>(v[i]) && R[v[i]].v < MAX_U64;
        aborts_if false;
        ensures forall i in 0..len(v): R[v[i]].v == old(R[v[i]].v) + 1;
        ensures forall b: address: (forall j in 0..len(v): v[j] != b) ==>
            (exists<R>(b) == old(exists<R>(b)) && (old(exists<R>(b)) ==> R[b].v == old(R[b].v)));
    }

    // ===== move_from: the projected `remove` effect is `!exists` =====

    fun remove_all(v: &vector<address>) {
        for_each_addr(v, |a| {
            let R { v: _ } = move_from<R>(a);
        });
    }
    spec remove_all {
        requires forall i in 0..len(v), j in 0..len(v): i != j ==> v[i] != v[j];
        requires forall i in 0..len(v): exists<R>(v[i]);
        aborts_if false;
        ensures forall i in 0..len(v): !exists<R>(v[i]);
        ensures forall b: address: (forall j in 0..len(v): v[j] != b) ==>
            (exists<R>(b) == old(exists<R>(b)) && (old(exists<R>(b)) ==> R[b].v == old(R[b].v)));
    }

    // ===== non-vacuity canary: a wrong per-element expectation fails =====

    fun bump_all_wrong(v: &vector<address>) {
        for_each_addr(v, |a| {
            let r = &mut R[a];
            r.v = r.v + 1;
        });
    }
    spec bump_all_wrong {
        requires forall i in 0..len(v), j in 0..len(v): i != j ==> v[i] != v[j];
        requires forall i in 0..len(v): exists<R>(v[i]) && R[v[i]].v < MAX_U64;
        aborts_if false;
        ensures forall i in 0..len(v): R[v[i]].v == old(R[v[i]].v) + 2; // error: off by one
    }

    // ===== lambda calling an opaque state-mutating function: the callee's =====
    // ===== summary passes through the predicates                          =====

    fun bump(a: address) {
        let r = &mut R[a];
        r.v = r.v + 1;
    }
    spec bump {
        pragma opaque;
        modifies global<R>(a);
        aborts_if !exists<R>(a) || R[a].v == MAX_U64;
        ensures exists<R>(a);
        ensures R[a].v == old(R[a].v) + 1;
    }

    // For `|a| bump(a)`, `ensures_of<f>`/`aborts_of<f>` substitute to the
    // callee summaries `ensures_of<bump>`/`aborts_of<bump>`, keeping the
    // entry/current resolution: the two-state `ensures_of<bump>(v[j])` reads
    // its pre-state at entry, while the single-state `aborts_of<bump>(v[j])`
    // is evaluated at the current state (hence the strengthened `< MAX_U64
    // - 1` bound below, which keeps elements below `MAX_U64` even after
    // their bump). `unchanged_of` is not derivable from a callee summary
    // (see bp_inline_errors.move), so this variant of the HOF writes the
    // frame invariants by hand: user `old(..)` over global state in a loop
    // invariant resolves to function entry.
    inline fun for_each_addr_framed(v: &vector<address>, f: |address|) {
        let i = 0;
        let n = vector::length(v);
        while (i < n) {
            f(*vector::borrow(v, i));
            i = i + 1;
        } spec {
            invariant i <= n;
            invariant forall j in 0..i: ensures_of<f>(v[j]);
            invariant forall j in 0..i: !aborts_of<f>(v[j]);
            invariant forall j in i..n:
                exists<R>(v[j]) && R[v[j]].v == old(R[v[j]].v);
            invariant forall x: address: (forall j in 0..i: x != v[j]) ==>
                (exists<R>(x) == old(exists<R>(x)) &&
                    (old(exists<R>(x)) ==> R[x].v == old(R[x].v)));
        };
    }

    fun bump_all_via_call(v: &vector<address>) {
        for_each_addr_framed(v, |a| bump(a));
    }
    spec bump_all_via_call {
        requires forall i in 0..len(v), j in 0..len(v): i != j ==> v[i] != v[j];
        requires forall i in 0..len(v): exists<R>(v[i]) && R[v[i]].v < MAX_U64 - 1;
        aborts_if false;
        ensures forall i in 0..len(v): R[v[i]].v == old(R[v[i]].v) + 1;
        ensures forall b: address: (forall j in 0..len(v): v[j] != b) ==>
            (exists<R>(b) == old(exists<R>(b)) && (old(exists<R>(b)) ==> R[b].v == old(R[b].v)));
    }
}
