-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

open VerexIR

-- A map built from literal vectors over the empty map computes: its keys in
-- order, a later equal key replacing an earlier one.
example : SpecMap.keyAt (SpecMap.updateAll (.ordered SpecMap.intOrder) ([] : SpecMap Int Int)
    [2, 1, 3] [5, 4, 6]) 0 = 1 := by
  verex_normalize

example : SpecMap.rank (SpecMap.updateAll (.ordered SpecMap.intOrder) ([] : SpecMap Int Int)
    [2, 1, 3] [5, 4, 6]) 3 = 2 := by
  verex_normalize

example : SpecMap.valueAt (SpecMap.updateAll (.ordered SpecMap.intOrder) ([] : SpecMap Int Int)
    [2, 1, 3] [5, 4, 6]) 2 = 5 := by
  verex_normalize

example : SpecMap.keyAt (SpecMap.updateAll .sequence ([] : SpecMap Int Int) [2, 1, 2] [5, 4, 6]) 1
      = 1 ∧ SpecMap.size (SpecMap.updateAll .sequence ([] : SpecMap Int Int) [2, 1, 2] [5, 4, 6]) = 2 := by
  verex_normalize

-- The default of a read outside the map is the one place a local name is
-- admitted: the owner's typed view spells an integer's by the opaque plain
-- fallback of the unit's carriers. A position inside the map and a key with
-- an entry read an entry's, so the default is not consumed and the
-- observation evaluates, at any default and the same value at two.
example (d : Int) : @SpecMap.keyAt Int Int ⟨d⟩ [(1, 4), (2, 5), (3, 6)] 0 = 1 := by
  verex_normalize

example (d : Int) : @SpecMap.keyAt Int Int ⟨d⟩ [(1, 4), (2, 5), (3, 6)] 2 = 3 := by
  verex_normalize

example (e : Int) : @SpecMap.keyAt Int Int ⟨e + 1⟩ [(1, 4), (2, 5), (3, 6)] 2 = 3 := by
  verex_normalize

example (d : Int) : @SpecMap.valueAt Int Int _ ⟨d⟩ [(1, 4), (2, 5), (3, 6)] 2 = 5 := by
  verex_normalize

-- Outside the map the value is the default, which is no value of this
-- fixture's: the observation stands, and nothing guesses a zero for it.
/-- error: `simp` made no progress -/
#guard_msgs in
example (d : Int) : @SpecMap.keyAt Int Int ⟨d⟩ [(1, 4), (2, 5), (3, 6)] (-1) = 0 := by
  verex_normalize

/-- error: `simp` made no progress -/
#guard_msgs in
example (d : Int) : @SpecMap.keyAt Int Int ⟨d⟩ [(1, 4), (2, 5), (3, 6)] 3 = 0 := by
  verex_normalize

/-- error: `simp` made no progress -/
#guard_msgs in
example (d : Int) : @SpecMap.keyAt Int Int ⟨d⟩ [] 0 = 0 := by
  verex_normalize

-- A key without an entry does not stay unevaluated: an earlier route reads
-- the absent value as the default itself, which is the owner's unspecified
-- value and not this fixture's zero, so the claim is refused.
/--
error: unsolved goals
d : Int
⊢ d = 0
-/
#guard_msgs in
example (d : Int) : @SpecMap.valueAt Int Int _ ⟨d⟩ [(1, 4), (2, 5), (3, 6)] 9 = 0 := by
  verex_normalize

-- With a symbolic key the map keeps its `updateAll` form and the lemmas
-- reading it.
example (k : Int) : SpecMap.hasKey (SpecMap.updateAll (.ordered SpecMap.intOrder)
    ([] : SpecMap Int Int) [k, 1] [5, 4]) 1 = true := by
  verex_normalize

-- Named cases run their authored steps; omitted cases use the cheap decider.
example (x : Int) : x * x = x * x ∧ x + 0 = x := by
  constructor
  verex_on
  | left => rfl

/-- error: unsolved goals
p : Prop
⊢ p
-/
#guard_msgs in
example (p : Prop) : p := by
  verex_on
  | absent => trivial

-- Choosing an outer variant does not enumerate a variant-valued field.
example (value : Unit ⊕ ((Unit ⊕ Unit) × Unit)) : True := by
  verex_cases value
  run_tac
    unless (← Lean.Elab.Tactic.getGoals).length == 2 do
      Lean.throwError "expected just the two outer variants"
  all_goals trivial
