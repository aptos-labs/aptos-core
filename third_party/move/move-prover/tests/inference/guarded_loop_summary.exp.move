/*
Inference returns: exiting with bytecode transformation errors
Inference diagnostics:
error: WP retained a quantified loop summary despite the supplied invariant. Inference has not established a trusted complete contract. Strengthen the invariant to characterize the loop-carried values and mutated state relative to entry; a bounds-only invariant may not suffice.
   ┌─ tests/inference/guarded_loop_summary.move:18:11
   │
18 │           } spec {
   │ ╭───────────^
19 │ │             // [inferred] The sweep keeps the vector's shape, so `last` stays the
20 │ │             // final index and every access below is in range.
21 │ │             invariant [inferred] len(values) == len(old(values));
   · │
29 │ │             invariant [inferred] forall j: num: store <= j && j < i ==> values[j] >= p;
30 │ │         };
   │ ╰─────────^

warning: WP could not characterize the aborts of `guarded_loop_summary::partition` exactly, so its emitted `aborts_if` clauses are a lower bound and the specification carries `aborts_if_is_partial`. Complete the abort behavior and remove that pragma before relying on the contract. Reasons:
  = an emitted abort condition is flagged `vacuous` or `sathard`
   ┌─ tests/inference/guarded_loop_summary.move:6:5
   │
 6 │ ╭     fun partition(values: &mut vector<u64>, pivot: u64): u64 {
 7 │ │         let last = values.length() - 1;
 8 │ │         values.swap(pivot, last);
 9 │ │         let p = values[last];
   · │
32 │ │         store
33 │ │     }
   │ ╰─────^
*/
