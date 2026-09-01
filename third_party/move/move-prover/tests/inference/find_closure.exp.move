// Test spec inference for a generic `find` over a vector with a closure
// predicate, using behavioral predicates (`result_of`, `aborts_of`) in the
// agent-supplied loop invariants.
// flag: -T=20
// flag: --aptos
module 0x42::find_closure {
    use std::vector;

    public fun find<T>(v: &vector<T>, pred: |&T|bool has copy + drop): u64 {
        let i = 0;
        let n = vector::length(v);
        while (i < n) {
            if (pred(vector::borrow(v, i))) {
                return i
            };
            i = i + 1;
        } spec {
            invariant [inferred] i <= n;
            invariant [inferred] n == len(v);
            invariant [inferred] forall j: u64 where j < i: !result_of<pred>(v[j]);
            invariant [inferred] forall j: u64 where j < i: !aborts_of<pred>(v[j]);
        };
        n
    }
    spec find<T>(v: &vector<T>, pred: |&T|bool has copy + drop): u64 {
        pragma opaque = true, aborts_if_is_partial = true;
        ensures [inferred = sathard] (forall x: u64: x < len(v) ==> !result_of<pred>(v[x]) && !aborts_of<pred>(v[x])) ==> result == len(v);
        ensures [inferred = sathard] forall y: u64: (forall x: u64: x < y ==> !result_of<pred>(v[x]) && !aborts_of<pred>(v[x])) && (y < len(v) && result_of<pred>(v[y])) ==> result == y;
    }

}
/*
Inference diagnostics:
warning: WP could not characterize the aborts of `find_closure::find` exactly, so its emitted `aborts_if` clauses are a lower bound and the specification carries `aborts_if_is_partial`. Complete the abort behavior and remove that pragma before relying on the contract. Reasons:
  = an abort condition did not survive a memory-havocking loop
  = a callee's `aborts_of` behavior is not accounted for
   ┌─ tests/inference/find_closure.move:9:5
   │
 9 │ ╭     public fun find<T>(v: &vector<T>, pred: |&T|bool has copy + drop): u64 {
10 │ │         let i = 0;
11 │ │         let n = vector::length(v);
12 │ │         while (i < n) {
   · │
23 │ │         n
24 │ │     }
   │ ╰─────^

Verification: Succeeded.
*/
