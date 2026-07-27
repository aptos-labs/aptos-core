// A fun param of a NON-carrying type does not advance: applying it in a loop
// leaves the value itself unchanged, so `f == old(f)` holds — the abstract
// variant stays a single constant, with no indexed family and no loop-head
// re-indexing. (Advancement is purely a property of mutation-carrying types.)
module 0x42::fun_param_identity_loop {

    fun apply_loop(f: |u64| u64 has copy + drop, n: u64): u64 {
        let i = 0;
        let acc = 0;
        while (i < n) {
            acc = f(acc);
            i = i + 1;
        } spec {
            invariant i <= n;
        };
        acc
    }
    spec apply_loop {
        pragma opaque;
        requires forall x: u64: !aborts_of<f>(x);
        aborts_if false;
        ensures f == old(f);
    }
}
