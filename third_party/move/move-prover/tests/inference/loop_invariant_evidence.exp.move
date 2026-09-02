// Human-facing baseline for optional missing-loop-invariant evidence.
// flag: --inference
// flag: --loop-invariant-evidence=3
// flag: -T=20
module 0x42::loop_invariant_evidence {
    fun count_down_together(x: u64, y: u64): (u64, u64) {
        while (x > 0 && y > 0) {
            x = x - 1;
            y = y - 1;
        };
        (x, y)
    }
    spec count_down_together(x: u64, y: u64): (u64, u64) {
        pragma opaque = true;
    }

}
/*
Inference diagnostics:
warning: WP inferred `vacuous` conditions after this loop without an invariant. The loop havoc left part of the inferred condition unconstrained. Add a loop invariant before relying on the inferred specification.
  ┌─ tests/inference/loop_invariant_evidence.move:7:16
  │
7 │         while (x > 0 && y > 0) {
  │                ^
  │
  = loop-invariant evidence (bounded to 3 completed back-edge traversal(s); diagnostic only)
  = source-visible loop-carried state: x, y
  = bounded WP status: exact within the displayed bound
  = bounded loop-head facts (for paths reaching each head):
      head[0]: head[0].x == x
               head[0].y == y
      head[1]: x > 0 && y > 0 ==> head[1].x == x - 1
               x > 0 && y > 0 ==> head[1].y == y - 1
      head[2]: x > 1 && y > 1 ==> head[2].x == x - 2
               x > 1 && y > 1 ==> head[2].y == y - 2
      head[3]: x > 2 && y > 2 ==> head[3].x == x - 3
               x > 2 && y > 2 ==> head[3].y == y - 3
  = seek a predicate which includes the entry facts and is preserved by one back-edge; bounded observations are not an invariant or a proof

Verification:
warning: WP inferred `vacuous` conditions after this loop without an invariant. The loop havoc left part of the inferred condition unconstrained. Add a loop invariant before relying on the inferred specification.
  ┌─ loop_invariant_evidence.enriched.move:7:16
  │
7 │         while (x > 0 && y > 0) {
  │                ^
  │
  = loop-invariant evidence (bounded to 3 completed back-edge traversal(s); diagnostic only)
  = source-visible loop-carried state: x, y
  = bounded WP status: exact within the displayed bound
  = bounded loop-head facts (for paths reaching each head):
      head[0]: head[0].x == x
               head[0].y == y
      head[1]: x > 0 && y > 0 ==> head[1].x == x - 1
               x > 0 && y > 0 ==> head[1].y == y - 1
      head[2]: x > 1 && y > 1 ==> head[2].x == x - 2
               x > 1 && y > 1 ==> head[2].y == y - 2
      head[3]: x > 2 && y > 2 ==> head[3].x == x - 3
               x > 2 && y > 2 ==> head[3].y == y - 3
  = seek a predicate which includes the entry facts and is preserved by one back-edge; bounded observations are not an invariant or a proof
*/
