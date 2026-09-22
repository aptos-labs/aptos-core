// Bounded loop-invariant evidence unrolls *every* loop in the function, not
// only the one it reports on, so the bounded DAG carries on the order of
// `(depth + 1)^loops` paths and a full weakest precondition runs once per loop
// head over that DAG. At the default depth of 3 that is 4 paths for one loop
// and roughly a million for ten.
//
// `many_loops` below is past the budget. The evidence is a diagnostic -- it
// suggests an invariant, it does not establish one -- so exceeding the budget
// must produce a note saying why no evidence is offered, and inference itself
// must still emit its contract. Before the budget existed this function did not
// terminate.
//
// `one_loop` is the control: comfortably inside the budget, it still receives
// bounded head facts, so the budget cannot be satisfied by disabling evidence.
module 0x42::loop_evidence_budget {

    fun one_loop(n: u64): u64 {
        let total = 0;
        let i = 0;
        while (i < n) {
            total = total + i;
            i = i + 1;
        };
        total
    }
    spec one_loop(n: u64): u64 {
        pragma opaque = true, aborts_if_is_partial = true;
    }


    fun many_loops(n: u64): u64 {
        let total = 0;
        let i = 0;
        while (i < n) { total = total + i; i = i + 1; };
        let i = 0;
        while (i < n) { total = total + i; i = i + 1; };
        let i = 0;
        while (i < n) { total = total + i; i = i + 1; };
        let i = 0;
        while (i < n) { total = total + i; i = i + 1; };
        let i = 0;
        while (i < n) { total = total + i; i = i + 1; };
        let i = 0;
        while (i < n) { total = total + i; i = i + 1; };
        total
    }
    spec many_loops(n: u64): u64 {
        pragma opaque = true, aborts_if_is_partial = true;
    }

}
/*
Inference diagnostics:
warning: WP inferred `vacuous` conditions after this loop without an invariant. The loop havoc left part of the inferred condition unconstrained. Add a loop invariant before relying on the inferred specification.
   ┌─ tests/inference/loop_evidence_budget.move:20:16
   │
20 │         while (i < n) {
   │                ^
   │
   = loop-invariant evidence (bounded to 3 completed back-edge traversal(s); diagnostic only)
   = source-visible loop-carried state: total, i
   = bounded WP status: exact within the displayed bound
   = bounded loop-head facts (for paths reaching each head):
       head[0]: head[0].i == 0
                head[0].total == 0
       head[1]: 0 < n ==> head[1].i == 1
                0 < n ==> head[1].total == 0
       head[2]: 1 < n ==> head[2].i == 2
                1 < n ==> head[2].total == 1
       head[3]: 2 < n ==> head[3].i == 3
                2 < n ==> head[3].total == 3
   = seek a predicate which includes the entry facts and is preserved by one back-edge; bounded observations are not an invariant or a proof

warning: WP could not characterize the aborts of `loop_evidence_budget::one_loop` exactly, so its emitted `aborts_if` clauses are a lower bound and the specification carries `aborts_if_is_partial`. Complete the abort behavior and remove that pragma before relying on the contract. Reasons:
  = an emitted abort condition is flagged `vacuous` or `sathard`
   ┌─ tests/inference/loop_evidence_budget.move:17:5
   │
17 │ ╭     fun one_loop(n: u64): u64 {
18 │ │         let total = 0;
19 │ │         let i = 0;
20 │ │         while (i < n) {
   · │
24 │ │         total
25 │ │     }
   │ ╰─────^

warning: WP inferred `vacuous` conditions after this loop without an invariant. The loop havoc left part of the inferred condition unconstrained. Add a loop invariant before relying on the inferred specification.
   ┌─ tests/inference/loop_evidence_budget.move:30:16
   │
30 │         while (i < n) { total = total + i; i = i + 1; };
   │                ^
   │
   = loop-invariant evidence (bounded to 3 completed back-edge traversal(s); diagnostic only)
   = source-visible loop-carried state: total, i
   = bounded WP status: partial
   = bounded loop-head facts (for paths reaching each head):
       head[0]: head[0].i == 0
                head[0].total == 0
       head[1]: 0 < n ==> head[1].i == 1
                0 < n ==> head[1].total == 0
       head[2]: 1 < n ==> head[2].i == 2
                1 < n ==> head[2].total == 1
       head[3]: 2 < n ==> head[3].i == 3
                2 < n ==> head[3].total == 3
   = partial evidence: 5 other loop(s) in this function are summarized, so facts that depend on their results are unconstrained here
   = seek a predicate which includes the entry facts and is preserved by one back-edge; bounded observations are not an invariant or a proof

warning: WP inferred `vacuous` conditions after this loop without an invariant. The loop havoc left part of the inferred condition unconstrained. Add a loop invariant before relying on the inferred specification.
   ┌─ tests/inference/loop_evidence_budget.move:32:16
   │
32 │         while (i < n) { total = total + i; i = i + 1; };
   │                ^
   │
   = loop-invariant evidence (bounded to 3 completed back-edge traversal(s); diagnostic only)
   = source-visible loop-carried state: total, i
   = bounded WP status: partial
   = bounded loop-head facts (for paths reaching each head):
       head[0]: head[0].i == 0
       head[1]: head[1].i == 1
       head[2]: head[2].i == 2
       head[3]: head[3].i == 3
   = partial evidence: 4 fact(s) across the bounded heads depend on summarized loop state and were omitted
   = partial evidence: 5 other loop(s) in this function are summarized, so facts that depend on their results are unconstrained here
   = seek a predicate which includes the entry facts and is preserved by one back-edge; bounded observations are not an invariant or a proof

warning: WP inferred `vacuous` conditions after this loop without an invariant. The loop havoc left part of the inferred condition unconstrained. Add a loop invariant before relying on the inferred specification.
   ┌─ tests/inference/loop_evidence_budget.move:34:16
   │
34 │         while (i < n) { total = total + i; i = i + 1; };
   │                ^
   │
   = loop-invariant evidence (bounded to 3 completed back-edge traversal(s); diagnostic only)
   = source-visible loop-carried state: total, i
   = bounded WP status: partial
   = bounded loop-head facts (for paths reaching each head):
       head[0]: head[0].i == 0
       head[1]: head[1].i == 1
       head[2]: head[2].i == 2
       head[3]: head[3].i == 3
   = partial evidence: 4 fact(s) across the bounded heads depend on summarized loop state and were omitted
   = partial evidence: 5 other loop(s) in this function are summarized, so facts that depend on their results are unconstrained here
   = seek a predicate which includes the entry facts and is preserved by one back-edge; bounded observations are not an invariant or a proof

warning: WP inferred `vacuous` conditions after this loop without an invariant. The loop havoc left part of the inferred condition unconstrained. Add a loop invariant before relying on the inferred specification.
   ┌─ tests/inference/loop_evidence_budget.move:36:16
   │
36 │         while (i < n) { total = total + i; i = i + 1; };
   │                ^
   │
   = loop-invariant evidence (bounded to 3 completed back-edge traversal(s); diagnostic only)
   = source-visible loop-carried state: total, i
   = bounded WP status: partial
   = bounded loop-head facts (for paths reaching each head):
       head[0]: head[0].i == 0
       head[1]: head[1].i == 1
       head[2]: head[2].i == 2
       head[3]: head[3].i == 3
   = partial evidence: 4 fact(s) across the bounded heads depend on summarized loop state and were omitted
   = partial evidence: 5 other loop(s) in this function are summarized, so facts that depend on their results are unconstrained here
   = seek a predicate which includes the entry facts and is preserved by one back-edge; bounded observations are not an invariant or a proof

warning: WP inferred `vacuous` conditions after this loop without an invariant. The loop havoc left part of the inferred condition unconstrained. Add a loop invariant before relying on the inferred specification.
   ┌─ tests/inference/loop_evidence_budget.move:38:16
   │
38 │         while (i < n) { total = total + i; i = i + 1; };
   │                ^
   │
   = loop-invariant evidence (bounded to 3 completed back-edge traversal(s); diagnostic only)
   = source-visible loop-carried state: total, i
   = bounded WP status: partial
   = bounded loop-head facts (for paths reaching each head):
       head[0]: head[0].i == 0
       head[1]: head[1].i == 1
       head[2]: head[2].i == 2
       head[3]: head[3].i == 3
   = partial evidence: 4 fact(s) across the bounded heads depend on summarized loop state and were omitted
   = partial evidence: 5 other loop(s) in this function are summarized, so facts that depend on their results are unconstrained here
   = seek a predicate which includes the entry facts and is preserved by one back-edge; bounded observations are not an invariant or a proof

warning: WP inferred `vacuous` conditions after this loop without an invariant. The loop havoc left part of the inferred condition unconstrained. Add a loop invariant before relying on the inferred specification.
   ┌─ tests/inference/loop_evidence_budget.move:40:16
   │
40 │         while (i < n) { total = total + i; i = i + 1; };
   │                ^
   │
   = loop-invariant evidence (bounded to 3 completed back-edge traversal(s); diagnostic only)
   = source-visible loop-carried state: total, i
   = bounded WP status: partial
   = bounded loop-head facts (for paths reaching each head):
       head[0]: head[0].i == 0
       head[1]: head[1].i == 1
       head[2]: head[2].i == 2
       head[3]: head[3].i == 3
   = partial evidence: 4 fact(s) across the bounded heads depend on summarized loop state and were omitted
   = partial evidence: 5 other loop(s) in this function are summarized, so facts that depend on their results are unconstrained here
   = seek a predicate which includes the entry facts and is preserved by one back-edge; bounded observations are not an invariant or a proof

warning: WP could not characterize the aborts of `loop_evidence_budget::many_loops` exactly, so its emitted `aborts_if` clauses are a lower bound and the specification carries `aborts_if_is_partial`. Complete the abort behavior and remove that pragma before relying on the contract. Reasons:
  = an emitted abort condition is flagged `vacuous` or `sathard`
   ┌─ tests/inference/loop_evidence_budget.move:27:5
   │
27 │ ╭     fun many_loops(n: u64): u64 {
28 │ │         let total = 0;
29 │ │         let i = 0;
30 │ │         while (i < n) { total = total + i; i = i + 1; };
   · │
41 │ │         total
42 │ │     }
   │ ╰─────^

Verification: Succeeded.
*/
