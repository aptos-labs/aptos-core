-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Semantics.Runtime

/-! Structural runtime equality is lawful for every value constructor.
The proof is shared; verifying a vector comparison never unfolds its elements. -/

namespace LeanerIR

instance : LawfulBEq FunctionHandle where
  eq_of_beq {a b} h := by
    rcases a with ⟨⟨an⟩, ⟨af⟩⟩
    rcases b with ⟨⟨bn⟩, ⟨bf⟩⟩
    simpa [BEq.beq, instBEqFunctionHandle.beq,
      instBEqFunctionId.beq] using h
  rfl {a} := by
    rcases a with ⟨⟨an⟩, ⟨af⟩⟩
    simp [BEq.beq, instBEqFunctionHandle.beq,
      instBEqFunctionId.beq]

private theorem array_measure (a : Array RuntimeValue) : sizeOf a = 1 + sizeOf a.toList := by
  cases a
  simp

mutual
theorem RuntimeValue.beq_eq_true (a b : RuntimeValue) : RuntimeValue.beq a b = true ↔ a = b := by
  cases a <;> cases b <;> simp only [RuntimeValue.beq, Bool.and_eq_true] <;> try simp
  all_goals
    first | rw [RuntimeValue.beqList_eq_true _ _] | rw [RuntimeValue.beq_eq_true _ _]
    simp [Array.toList_inj, and_assoc]
termination_by sizeOf a + sizeOf b
decreasing_by all_goals (simp_wf; (try simp only [array_measure]); omega)

theorem RuntimeValue.beqList_eq_true (a b : List RuntimeValue) :
    RuntimeValue.beqList a b = true ↔ a = b := by
  cases a <;> cases b <;> simp only [RuntimeValue.beqList, Bool.and_eq_true] <;> try simp
  rw [RuntimeValue.beq_eq_true _ _, RuntimeValue.beqList_eq_true _ _]
termination_by sizeOf a + sizeOf b
decreasing_by all_goals (simp_wf; omega)
end

instance : LawfulBEq RuntimeValue where
  eq_of_beq := RuntimeValue.beq_eq_true _ _ |>.mp
  rfl := RuntimeValue.beq_eq_true _ _ |>.mpr rfl

end LeanerIR

