-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- A specification and a proof name a loop alike: the invariants of `loop k`
-- are keyed `invariant j of loop k`, numbered among the function's invariants,
-- and an overflow check is an `op` site.
spec nested where
  requires rows <= 1000
  requires columns <= 1000
  panics_if false
  ensures result <= rows * 1000
  loop 1 where
    invariant row <= rows
    invariant total <= row * 1000
  loop 2 where
    invariant column <= columns
    invariant total <= row * 1000 + column

verify nested by lean =>
  verex_obligations
  verex_on
  | invariant 2 of loop 1 step when loop 1 continues, loop 2 exits =>
    trace "nested: the outer invariant after an iteration"; verex_solve
  | invariant 3 of loop 2 entry when loop 1 continues =>
    trace "nested: the inner invariant at entry"; verex_solve
  | invariant 4 of loop 2 step when loop 2 continues =>
    trace "nested: the inner invariant after an iteration"; verex_solve
  | panics_if 1 when aborts at op 1 => trace "nested: `total += 1` aborts"; verex_solve
  | panics_if 1 when aborts at op 3 => trace "nested: `row += 1` aborts"; verex_solve
  | _ => verex_solve

spec skip_rows where
  requires rows <= 1000
  requires columns <= 1000
  panics_if false
  loop 1 where
    invariant row <= rows
    invariant count <= row * 1002
  loop 2 where
    invariant column <= columns
    invariant count <= (row - 1) * 1002 + column

verify skip_rows by lean =>
  verex_obligations
  verex_on
  | invariant 4 of loop 2 step when loop 2 continues =>
    trace "skip_rows: the inner invariant after an iteration"; verex_solve
  | invariant 1 of loop 1 step when loop 2 continues =>
    trace "skip_rows: `continue` of the outer loop"; verex_solve
  | _ => verex_solve

spec classify where
  requires limit <= 1000
  ensures result >= limit
  loop 1 where
    invariant index <= n
    invariant index <= limit + 1
  loop 2 where
    invariant total <= limit || total == kind

verify classify by lean =>
  verex_obligations
  verex_on
  | invariant 2 of loop 1 step when if 2 else =>
    trace "classify: the first loop's invariant after an iteration"; verex_solve
  | invariant 3 of loop 2 entry =>
    trace "classify: the second loop's invariant at entry"; verex_solve
  | _ => verex_solve

-- Facts at a loop are proof steps at its points.
spec facts where
  ensures result == n
  loop 1 where
    invariant i <= n

verify facts by lean =>
  verex_obligations
    at loop 1 step =>
      have stepping : i.val ≤ n.val := by assumption
      trace "facts: at the loop's step"
  verex_on
  | invariant 1 step => verex_prepare values; all_goals (have _ := stepping; omega)
  | _ => verex_solve
