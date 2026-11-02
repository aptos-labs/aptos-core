-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-!
An authored proof addresses an obligation by its key
(`doc/plan/proof-language.md`, "Obligation addressing"): a callee's
precondition by the callee's clause and the call, numbered among the calls
of the callee in source order, so that one call's precondition is routed
and the other's is not; without the call, every call's. A selector takes
an obligation's paths by their source decisions: the branch an `if` takes,
where a path aborts (an `assert!`, an operation), whether it leaves a loop,
and the arm a `match` takes.
-/

namespace VerexSL.Tests.Check.Specifications.Addressing

verex module 0x42::addressing where
  fun positive(x : u64) -> u64 := x
  spec positive where
    pragma opaque
    requires x > 0
    aborts_if false
    ensures result == x

  fun twice(x : u64) -> u64 := do
    let a := positive(x + 1)
    positive(a)
  spec twice where
    requires x < 100
    aborts_if false
    ensures result == x + 1

  fun twice_again(x : u64) -> u64 := do
    let a := positive(x + 1)
    positive(a)
  spec twice_again where
    requires x < 100
    aborts_if false
    ensures result == x + 1

  -- Only the second call's precondition takes the first alternative.
  verify twice by lean =>
    verex_obligations
    verex_on
    | requires 1 of positive at call 2 => trace "routed: the second call's precondition"; verex_solve
    | _ => verex_solve

  -- Both calls' preconditions take it.
  verify twice_again by lean =>
    verex_obligations
    verex_on
    | requires 1 of positive => trace "routed: a call's precondition"; verex_solve
    | _ => verex_solve

verex module 0x42::selection where
  fun pick(x : u64, flag : Bool) -> u64 := do
    if flag then
      assert!(x < 100, 7)
      x + 1
    else
      x * 2
  spec pick where
    aborts_if flag && x >= 100
    aborts_if !flag && x * 2 > 18446744073709551615
    ensures flag ==> result == x + 1
    ensures !flag ==> result == x * 2

  fun count_to(n : u64) -> u64 := do
    let mut i : u64 := 0
    while i < n do
      i := i + 1
    where
      invariant i <= n
    i
  spec count_to where
    aborts_if false
    ensures result == n

  -- Every abort path is one obligation, keyed by the first abort clause.
  verify pick by lean =>
    verex_obligations
    verex_on
    | aborts_if 1 when aborts at abort 1 => trace "routed: the assert! aborts"; verex_solve
    | aborts_if 1 when if 1 else, aborts at op 2 =>
      trace "routed: the multiplication aborts"; verex_solve
    | ensures 2 when if 1 else => trace "routed: the else branch"; verex_solve
    | _ => verex_solve

  verify count_to by lean =>
    verex_obligations
    verex_on
    | ensures 1 when loop 1 exits => trace "routed: the loop's exit"; verex_solve
    | invariant 1 step when loop 1 continues => trace "routed: an iteration"; verex_solve
    | _ => verex_solve

verex module 0x42::arms where
  enum Action has Copy, Drop, Store where
    | Idle
    | Transfer (amount : u64)
    | Split (left : u64, right : u64)

  fun total(action : Action) -> u64 :=
    match action with
      | Action::Idle {} => 0
      | Action::Transfer { amount := amount } => amount
      | Action::Split { left := left, right := right } => left + right
  spec total where
    pragma aborts_if_is_partial
    ensures result == match action with
      | Action::Idle {} => 0
      | Action::Transfer { amount := amount } => amount
      | Action::Split { left := left, right := right } => left + right

  verify total by lean =>
    verex_obligations
    verex_on
    | ensures 1 when match 1 Split => trace "routed: the Split arm"; verex_solve
    | _ => verex_solve

end VerexSL.Tests.Check.Specifications.Addressing
