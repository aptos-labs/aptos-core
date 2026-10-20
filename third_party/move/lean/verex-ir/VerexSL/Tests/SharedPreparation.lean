-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

open VerexIR.Proofs

-- The block sees the conjunction once, before any clause becomes a leaf.
-- Its case split must preserve the source names on both resulting paths.
example (flag : Bool) (n : Int) :
    Obligation "shared" 0 1 (n < n + 1) ∧
    Obligation "shared" 2 3 (n < n + 2) ∧
    Obligation "shared" 6 7 (∀ m : Int, n < m → n < m + 1) := by
  verex_obligations [] naming
      [("first", "shared", 0, 1), ("second", "shared", 2, 3), ("third", "shared", 6, 7)] preparing
    run_tac do
      unless (← Lean.Elab.Tactic.getMainTarget).isAppOfArity ``And 2 do
        throwError "shared preparation received an already split clause"
    cases flag
    all_goals verex_prepare values
    -- Substitution/map lifting may annotate the new target.
    all_goals run_tac do
      let goal ← Lean.Elab.Tactic.getMainGoal
      Lean.Elab.Tactic.setGoals [← goal.replaceTargetDefEq (.mdata {} (← goal.getType))]
  verex_on
  | first => verex_paths; all_goals (simp only [Obligation_eq]; change n < n + 1; omega)
  | second => verex_paths; all_goals (simp only [Obligation_eq]; change n < n + 2; omega)
  | third => verex_paths; all_goals (simp only [Obligation_eq]; intro m h; omega)
  | _ => fail "shared preparation lost a clause name"

-- The broader preparation also derives map-position facts. It must retain
-- the goal markers while unwrapping the assumptions those facts read.
example (m : VerexIR.RuntimeValue) :
    Obligation "map" 0 1 (VerexIR.Maps.rank m (.integer 0) < VerexIR.Maps.rank m (.integer 0) + 1) ∧
    Obligation "map" 2 3 (VerexIR.Maps.rank m (.integer 0) < VerexIR.Maps.rank m (.integer 0) + 2) := by
  verex_obligations [] naming [("first", "map", 0, 1), ("second", "map", 2, 3)] preparing
    verex_prepare
  verex_on
  | first, second => verex_paths; all_goals (simp only [Obligation_eq]; omega)
  | _ => fail "map-position preparation lost a clause name"

-- Markers on assumptions must not hide contradictions during preparation.
example (h : Obligation "shared" 4 5 False) : False := by
  verex_obligations [] preparing
    verex_prepare values
  done

-- A preparation block can close some branches, but must retain the others.
-- An invalid second conjunct is not proved by closing the first one.
example : True := by
  fail_if_success
    have : (0 : Int) ≤ 0 ∧ (0 : Int) < 0 := by
      verex_obligations [] preparing
        constructor
        · exact Int.le_refl 0
        · skip
      all_goals omega
  trivial

-- The verifier must inject the generated call/loop metadata while retaining
-- the authored block. The modifier remains usable as an ordinary identifier.
verex module 0x42::shared_preparation where
  fun source(x : u64) -> u64 := x
  spec source where
    pragma opaque
    aborts_if false
    ensures result == x

  fun copy_twice(x : u64) -> u64 := source(source(x))
  spec copy_twice where
    aborts_if false
    ensures result == x
    ensures result <= x
    ensures result >= x

  fun count(limit : u64) -> u64 := do
    let mut i := 0
    while i < limit do
      i := source(i) + 1
    spec do
      invariant i <= limit
    source(i)
  spec count where
    aborts_if false
    ensures result == limit
    ensures result <= limit

verify 0x42::shared_preparation::source
verify 0x42::shared_preparation::copy_twice by
  verex_obligations preparing
    verex_prepare values
    all_goals have preparing : True := trivial
  all_goals verex_decide

-- Call and loop obligations share the same preparation interface.
verify 0x42::shared_preparation::count by
  verex_obligations preparing
    verex_prepare values
  all_goals verex_decide
