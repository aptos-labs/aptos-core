-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.IR.Execution
import MoveModel.IR.Mono.Correctness.Semantics

/-!
# Local execution-step transport

This layer lifts primitive semantic congruence to the local head-step
judgments used by structured execution induction. Function calls are absent
from these judgments and are handled by certified call closure separately.
-/

namespace MoveModel.IR

/-- A continuing operation instruction takes the same step after
tag-equivalent substitutions. -/
theorem InstrNext.call_instantiate_of_typeArgsTagEq {lhs rhs : List Ty}
    (hargs : TypeArgsTagEq lhs rhs) (op : Oper)
    {dsts srcs : List LocalIndex} {s s' : MoveState}
    (h : InstrNext
      (.call dsts (op.instantiate lhs) srcs) s s') :
    InstrNext (.call dsts (op.instantiate rhs) srcs) s s' := by
  cases op <;> cases h <;>
    try { constructor <;> assumption }
  all_goals first
    | apply InstrNext.op (hsrcs := by assumption) (hlen := by assumption)
      rw [← Oper.sem_instantiate_eq hargs]
      assumption
    | rename_i resource types dst target address value ha hpresent
      have hkey := resourceKey_instantiateTypes_eq hargs resource types
      rw [hkey] at hpresent ⊢
      exact .borrowGlobalInst ha hpresent

/-- An aborting operation instruction has the same outcome after
tag-equivalent substitutions. -/
theorem InstrStop.call_instantiate_of_typeArgsTagEq {lhs rhs : List Ty}
    (hargs : TypeArgsTagEq lhs rhs) (op : Oper)
    {dsts srcs : List LocalIndex} {s : MoveState} {outcome : FrameOutcome}
    (h : InstrStop
      (.call dsts (op.instantiate lhs) srcs) s outcome) :
    InstrStop (.call dsts (op.instantiate rhs) srcs) s outcome := by
  cases op <;> cases h <;>
    try { constructor <;> assumption }
  all_goals first
    | apply InstrStop.op (hsrcs := by assumption)
      rw [← Oper.sem_instantiate_eq hargs]
      assumption
    | simp only [Oper.instantiate]
      apply InstrStop.borrowGlobalInst (ha := by assumption)
      rw [← resourceKey_instantiateTypes_eq hargs _ _]
      assumption
    | simp only [Oper.instantiate]
      apply InstrStop.borrowGlobal <;> assumption
    | simp only [Oper.instantiate]
      apply InstrStop.borrowVecElem <;> assumption

/-- Any continuing non-call instruction transports across tag-equivalent
substitutions. Calls cannot inhabit `InstrNext`, so their case is discharged
by the specialized operation lemma above. -/
theorem InstrNext.instantiate_of_typeArgsTagEq {lhs rhs : List Ty}
    (hargs : TypeArgsTagEq lhs rhs) (instr : Instr)
    {s s' : MoveState} (h : InstrNext (instr.instantiate lhs) s s') :
    InstrNext (instr.instantiate rhs) s s' := by
  cases instr with
  | load => simpa [Instr.instantiate] using h
  | assign => simpa [Instr.instantiate] using h
  | nop => simpa [Instr.instantiate] using h
  | call dsts op srcs =>
      exact InstrNext.call_instantiate_of_typeArgsTagEq hargs op h

/-- Any aborting non-call instruction transports across tag-equivalent
substitutions. -/
theorem InstrStop.instantiate_of_typeArgsTagEq {lhs rhs : List Ty}
    (hargs : TypeArgsTagEq lhs rhs) (instr : Instr)
    {s : MoveState} {outcome : FrameOutcome}
    (h : InstrStop (instr.instantiate lhs) s outcome) :
    InstrStop (instr.instantiate rhs) s outcome := by
  cases instr with
  | load => simpa [Instr.instantiate] using h
  | assign => simpa [Instr.instantiate] using h
  | nop => simpa [Instr.instantiate] using h
  | call dsts op srcs =>
      exact InstrStop.call_instantiate_of_typeArgsTagEq hargs op h

/-- A finite continuing straight-line path transports pointwise across
tag-equivalent substitutions. -/
theorem InstrPath.map_instantiate_of_typeArgsTagEq {lhs rhs : List Ty}
    (hargs : TypeArgsTagEq lhs rhs) : ∀ {instrs : List Instr} {s s' : MoveState},
    InstrPath (instrs.map (Instr.instantiate lhs)) s s' →
      InstrPath (instrs.map (Instr.instantiate rhs)) s s'
  | [], _, _, h => by simpa using h
  | instr :: rest, _, _, h => by
      simp only [List.map_cons] at h ⊢
      cases h with
      | cons hhead hrest =>
          exact .cons
            (hhead.instantiate_of_typeArgsTagEq hargs instr)
            (map_instantiate_of_typeArgsTagEq hargs hrest)

end MoveModel.IR
