-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "bitwise_table.move"
verex proofs where

open VerexIR VerexIR.Proofs VerexIR.Proofs.Denote

private theorem different_literal_keys (owner : StructHandle) :
    RuntimeValue.nominal owner none #[.vector #[.integer 1, .integer 2]] ≠
      RuntimeValue.nominal owner none #[.vector #[.integer 2, .integer 3]] := by
  intro same
  have := congrArg (fun fields => fields.toList) (RuntimeValue.nominal.inj same).2.2
  simp only [List.cons.injEq, RuntimeValue.vector.injEq, Array.mk.injEq,
    RuntimeValue.integer.injEq] at this
  omega

verify add_R by
  verex_obligations
  verex_on
  | ensures_3 =>
    verex_prepare only
    subst old_value'' old_value' old_value new_result
    simp only [Maps.valueAt_update, Maps.valueAt_build, Maps.get, Maps.asInt_integer, ite_true] at *
    simp only [if_neg (different_literal_keys _), RuntimeValue.integer.injEq] at consumed
    rw [← consumed]
    decide
  | _ => first | verex_decide | (verex_prepare only; all_goals verex_decide)
