-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

/-! A `pragma bv` target may rest on `bv_decide`'s certificates, axioms
stating that a natively evaluated checker accepts an UNSAT proof. An axiom
named and shaped like one, stating that the checker accepts an empty proof
for a satisfiable formula, proves `False`; the audit evaluates the checker
again and rejects it. -/

axiom forged._native.bv_decide.ax_1 :
  Std.Tactic.BVDecide.Reflect.verifyBVExpr (.const true) "" = true

leaner module 0x4b::forged where
  fun one() -> u64 := 1

  spec one where
    pragma bv = b"0"
    ensures result == 2

  verify one by
    have unsat := Std.Tactic.BVDecide.Reflect.unsat_of_verifyBVExpr_eq_true _ _
      forged._native.bv_decide.ax_1
    have : False := absurd (unsat (Lean.RArray.leaf { bv := 0#0 })) (by simp)
    all_goals exact this.elim
