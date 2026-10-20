-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

open VerexIR VerexIR.Maps

/-- Closed value ranks, as a unit's are. -/
private def ranks : ValueRanks := { variant := fun _ _ => 0, function := fun _ => 0 }

-- A map built from literal vectors over the empty map computes: its keys in
-- order, a later equal key replacing an earlier one.
example (layout : Layout) :
    keyAt (updateAll layout (.ordered ranks) (empty layout)
      (.vector #[.integer 2, .integer 1, .integer 3]) (.vector #[.integer 5, .integer 4, .integer 6]))
      0 = .integer 1 := by
  verex_normalize

example (layout : Layout) :
    Maps.rank (updateAll layout (.ordered ranks) (empty layout)
      (.vector #[.integer 2, .integer 1, .integer 3]) (.vector #[.integer 5, .integer 4, .integer 6]))
      (.integer 3) = 2 := by
  verex_normalize

example (layout : Layout) :
    valueAt (updateAll layout (.ordered ranks) (empty layout)
      (.vector #[.integer 2, .integer 1, .integer 3]) (.vector #[.integer 5, .integer 4, .integer 6]))
      (.integer 2) = .integer 5 := by
  verex_normalize

example (layout : Layout) :
    keyAt (updateAll layout .sequence (empty layout)
      (.vector #[.integer 2, .integer 1, .integer 2]) (.vector #[.integer 5, .integer 4, .integer 6])) 1
      = .integer 1 ∧ size (updateAll layout .sequence (empty layout)
      (.vector #[.integer 2, .integer 1, .integer 2]) (.vector #[.integer 5, .integer 4, .integer 6])) = 2 := by
  verex_normalize

-- With a symbolic key the map keeps its `updateAll` form and the laws
-- reading it.
example (layout : Layout) (k : Int) :
    hasKey (updateAll layout (.ordered ranks) (empty layout)
      (.vector #[.integer k, .integer 1]) (.vector #[.integer 5, .integer 4])) (.integer 1) = true := by
  verex_normalize

-- Abstract maps may carry a handle and validity before their entries.
-- Preparation names the whole map and retains its carrier's size bound.
example [VerexIR.Proofs.Denote.Carriers] (owner : StructHandle)
    (handle validity : RuntimeValue)
    (entries : (VerexIR.Proofs.Denote.NTy.table
      (.int 64 false) (.int 64 false)).carrier) (key : RuntimeValue)
    (present : hasKey (.nominal owner none
      #[handle, validity, VerexIR.Proofs.Denote.NTy.encode (.table (.int 64 false) (.int 64 false)) entries]) key = true) :
    hasKey (.nominal owner none
      #[handle, validity, VerexIR.Proofs.Denote.NTy.encode (.table (.int 64 false) (.int 64 false)) entries]) key = true := by
  verex_name_maps
  guard_hyp entries : RuntimeValue
  guard_hyp entries_keys : KeysRead RuntimeValue.asInt RuntimeValue.integer entries
  exact present

example [VerexIR.Proofs.Denote.Carriers] (owner : StructHandle)
    (handle validity : RuntimeValue)
    (entries : (VerexIR.Proofs.Denote.NTy.table
      (.int 64 false) (.int 64 false)).carrier) :
    size (.nominal owner none
      #[handle, validity, VerexIR.Proofs.Denote.NTy.encode (.table (.int 64 false) (.int 64 false)) entries]) < 2 ^ 64 := by
  verex_name_maps
  exact entries_size_bounds.2

-- A named map retains the source key type, including its numeric range.
-- This lets authored proofs instantiate quantified contracts at a map key.
open VerexIR.Proofs.Denote in
example [Carriers] (owner : StructHandle) (validity : RuntimeValue)
    (entries : (NTy.table (.int 8 false) (.int 64 false)).carrier) (i : Int)
    (low : 0 ≤ i)
    (high : i < size (.nominal owner none
      #[validity, NTy.encode (.table (.int 8 false) (.int 64 false)) entries])) :
    ∃ key : SpecInt (.bits 8) false,
      RuntimeValue.integer key.val = keyAt (.nominal owner none
        #[validity, NTy.encode (.table (.int 8 false) (.int 64 false)) entries]) i := by
  verex_name_maps
  exact encoded_keyAt entries_keys_encoded low high

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

-- Variant facts use the theorem's Skolems instance; a carrier is not the
-- validated unit argument of the theorem.
open VerexIR.Proofs.Denote in
example {unit : Validation.ValidatedUnit} [Skolems unit] (owner : StructHandle)
    (value : variantCarrier ["Absent", "Present"] (.cons .nil (.cons (.cons (.int 64 false) .nil) .nil)))
    (payload : Array RuntimeValue)
    (encoded : NTy.encode (.enum owner .nil ["Absent", "Present"]
      (.cons .nil (.cons (.cons (.int 64 false) .nil) .nil)) (by decide)) value =
      .nominal owner (some "Present") payload) :
    variantName ["Absent", "Present"] (.cons .nil (.cons (.cons (.int 64 false) .nil) .nil)) value = "Present" := by
  verex_bounds
  assumption
