-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-! A common proof case is scheduled once before its source clauses
split. The same owners cover both cases; clause-local binders stay local. -/

set_option Elab.async false

open Lean Elab Command in
elab "#guard_shared_cases " count:num " owners " owners:num " in " command:command : command => do
  let environment ← getEnv
  let (output, ()) ← IO.FS.withIsolatedStreams (isolateStderr := false) <|
    elabCommand (← `(command|
      set_option verex.sharedContinuations true in
      set_option verex.solverProfile true in $command))
  setEnv environment
  unless (output.splitOn "\n").any (·.startsWith s!"  automatic owners: {owners.getNat},") do
    throwError "shared cases changed the number of automatic owners"
  let lines := (output.splitOn "\n").filter fun line =>
    line.startsWith "  stage common conditional " || line.startsWith "  stage common variant " ||
      line.startsWith "  stage common entry "
  if count.getNat == 0 then
    unless lines.isEmpty do throwError "an unshared conditional was lifted: {lines}"
  else
    unless lines.length == 1 && lines.all (fun line =>
        line.startsWith s!"  stage common conditional ×{count.getNat} " ||
        line.startsWith s!"  stage common variant ×{count.getNat} " ||
        line.startsWith s!"  stage common entry ×{count.getNat} ") do
      throwError "unexpected shared conditional count: {lines}"

open VerexIR.Proofs

-- Slot decoding is shared while its absent case remains a required proof.
#guard_shared_cases 1 owners 2 in
example (slot : Option Int) (present : slot.isSome = true) :
    Obligation "slot-cases" 0 1 (slot.elim False (fun n => n < n + 1)) ∧
    Obligation "slot-cases" 2 3 (slot.elim False (fun n => n < n + 2)) := by
  verex_solve_goals [] naming [("first", "slot-cases", 0, 1), ("second", "slot-cases", 2, 3)]

#guard_shared_cases 0 owners 2 in
example (slot : Option Int) (n : Int) (present : slot = some n) :
    Obligation "known-slot" 0 1 (slot.elim False (fun n => n < n + 1)) ∧
    Obligation "known-slot" 2 3 (slot.elim False (fun n => n < n + 2)) := by
  verex_solve_goals [] naming [("first", "known-slot", 0, 1), ("second", "known-slot", 2, 3)]

-- Distinct match expressions over the same enum value share its tag case.
#guard_shared_cases 1 owners 2 in
example (value : Sum Int Int) :
    Obligation "sum-cases" 0 1
      ((match value with | .inl n => n | .inr n => n + 1) <
        (match value with | .inl n => n + 1 | .inr n => n + 2)) ∧
    Obligation "sum-cases" 2 3
      ((match value with | .inl n => n + 1 | .inr n => n + 2) <
        (match value with | .inl n => n + 2 | .inr n => n + 3)) := by
  verex_solve_goals [] naming [("first", "sum-cases", 0, 1), ("second", "sum-cases", 2, 3)]

#guard_shared_cases 1 owners 2 in
example (b : Bool) :
    Obligation "shared-cases" 0 1 (1 ≤ (if b then (1 : Int) else 2)) ∧
    Obligation "shared-cases" 2 3 ((if b then (1 : Int) else 2) ≤ 2) := by
  verex_solve_goals [] naming [("lower", "shared-cases", 0, 1), ("upper", "shared-cases", 2, 3)]

-- Only one common split precedes clause decisions. The remaining Boolean
-- stays in each leaf, rather than forming all combinations up front.
#guard_shared_cases 1 owners 2 in
example (b c : Bool) :
    Obligation "two-cases" 0 1 (4 ≤ (if b then (1 : Int) else 2) + (if c then 3 else 4)) ∧
    Obligation "two-cases" 2 3 ((if b then (1 : Int) else 2) + (if c then 3 else 4) ≤ 6) := by
  verex_solve_goals [] naming [("lower", "two-cases", 0, 1), ("upper", "two-cases", 2, 3)]

-- An existing assumption decides the conditional without another split.
#guard_shared_cases 0 owners 2 in
example (b : Bool) (known : b = true) :
    Obligation "known-case" 0 1 (1 ≤ (if b then (1 : Int) else 2)) ∧
    Obligation "known-case" 2 3 ((if b then (1 : Int) else 2) ≤ 2) := by
  verex_solve_goals [] naming [("lower", "known-case", 0, 1), ("upper", "known-case", 2, 3)]

-- A direct proof of the group takes precedence over scheduling its cases.
#guard_shared_cases 0 owners 2 in
example (b : Bool)
    (known : Obligation "proved-group" 0 1 (1 ≤ (if b then (1 : Int) else 2)) ∧
      Obligation "proved-group" 2 3 ((if b then (1 : Int) else 2) ≤ 2)) :
    Obligation "proved-group" 0 1 (1 ≤ (if b then (1 : Int) else 2)) ∧
    Obligation "proved-group" 2 3 ((if b then (1 : Int) else 2) ≤ 2) := by
  verex_solve_goals [] naming [("lower", "proved-group", 0, 1), ("upper", "proved-group", 2, 3)]

-- Independent quantified indices are not the same discriminator, even if
-- their bound-variable expressions have the same shape.
#guard_shared_cases 0 owners 2 in
example :
    Obligation "bound-cases" 0 1 (∀ i : Int, (if i ≤ 0 then i else 0) ≤ 0) ∧
    Obligation "bound-cases" 2 3 (∀ j : Int, (if j ≤ 0 then j else 0) ≤ 0) := by
  verex_solve_goals [] naming [("left", "bound-cases", 0, 1), ("right", "bound-cases", 2, 3)]

-- Logical subformulas inside one source occurrence retain one leaf owner.
#guard_shared_cases 0 owners 1 in
example (b : Bool) :
    Obligation "one-case" 0 1
      (1 ≤ (if b then (1 : Int) else 2) ∧ (if b then (1 : Int) else 2) ≤ 2) := by
  verex_solve_goals [] naming [("range", "one-case", 0, 1)]

-- An observed native tag resolves the other constructor's guarded facts
-- before preparing the clause. Its logical conjuncts retain one owner.
section ObservedVariants
open VerexIR.Proofs.Denote
variable {unit : VerexIR.Validation.ValidatedUnit} [Skolems unit]
set_option quotPrecheck false
local notation "rows" => (NRows.cons .nil (.cons (.cons (.int 64 false) .nil) .nil))

#guard_shared_cases 0 owners 1 in
example (value : variantCarrier ["End", "Position"] rows) (n : Int)
    (tag : ("End" == variantName ["End", "Position"] rows value) = false)
    (observed : ("Position" == variantName ["End", "Position"] rows value) = true → n = 3) :
    Obligation "observed-variant" 0 1 (n < 4 ∧ 2 < n) := by
  verex_solve_goals [] naming [("bound", "observed-variant", 0, 1)]

end ObservedVariants

-- The completed suffix's clauses share one opening of their incoming
-- relation, with two owners covering both predecessor alternatives.
verex module 0x42::shared_entry_cases where
  pragma verify = false
  fun bounded(b : Bool) -> u64 := do
    let mut x : u64 := 0
    if b then x := 1 else x := 2
    x
  spec bounded where
    aborts_if false
    ensures 1 <= result
    ensures result <= 2

  fun wrong(b : Bool) -> u64 := do
    let mut x : u64 := 0
    if b then x := 1 else x := 2
    x
  spec wrong where
    aborts_if false
    ensures 1 <= result
    ensures result == 1

#guard_shared_cases 1 owners 2 in
verify 0x42::shared_entry_cases::bounded

/--
error: the specification clause `ensures result == 1` is not established
---
error: verex verification failed: the automatic verification of `wrong` failed; provide a proof in the module:
  verify wrong by lean =>
    verex_obligations
    verex_on
    | ensures_2 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
#guard_shared_cases 1 owners 2 in
verify 0x42::shared_entry_cases::wrong

-- An impossible ordering guard does not need the successful call's
-- quantified postconditions. They remain available if that attempt fails.
#guard_shared_cases 0 owners 1 in
example (map : VerexIR.SpecMap Int Int) (k : Int)
    (nonempty : 0 < VerexIR.SpecMap.size map)
    (outOfOrder : k ≤ VerexIR.SpecMap.keyAt map (VerexIR.SpecMap.size map - 1))
    (successful : ¬(0 < VerexIR.SpecMap.size map ∧
      VerexIR.SpecMap.intOrder.compare (VerexIR.SpecMap.keyAt map (VerexIR.SpecMap.size map - 1)) k ≠ .lt ∨
      VerexIR.SpecMap.size map < VerexIR.SpecMap.size map ∧
        VerexIR.SpecMap.intOrder.compare k (VerexIR.SpecMap.keyAt map (VerexIR.SpecMap.size map)) ≠ .lt))
    (_postcondition : ∀ i : Int, 0 ≤ i → i < VerexIR.SpecMap.size map →
      VerexIR.SpecMap.keyAt map i = VerexIR.SpecMap.keyAt map i) : False := by
  verex_solve_goals []

verex module 0x42::shared_case_errors where
  pragma verify = false
  fun wrong(b : Bool, n : u64) -> u64 := n
  spec wrong where
    requires n < 10
    aborts_if false
    ensures (if b then result + 1 else result + 2) >= n + 1
    ensures (if b then result + 1 else result + 2) == n + 1

-- Both cases are required. The second clause fails only on the false case,
-- while its valid sibling retains its own verdict and source identity.
/--
error: the specification clause `ensures (if b then result + 1 else result + 2) == n + 1` is not established
---
error: verex verification failed: the automatic verification of `wrong` failed; provide a proof in the module:
  verify wrong by lean =>
    verex_obligations
    verex_on
    | ensures_2 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
#guard_shared_cases 1 owners 2 in
verify 0x42::shared_case_errors::wrong
