-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "nested_loop_inv.move"
verex proofs where

open VerexIR VerexIR.Proofs VerexIR.Proofs.Denote

verify assert_no_duplicate by
  verex_obligations
  verex_on
  | aborts_if_1 =>
    verex_prepare only
    all_goals try verex_decide
    all_goals refine ⟨i.val, by omega, by omega, j.val, by omega, by omega, by omega, by omega, by omega, ?_⟩
    all_goals simp_all only [Option.getD_some]
  | leaf =>
    verex_prepare only
    all_goals try omega
    all_goals
      rintro ⟨x, xlo, xhi, y, ylo, yhi, hx, hy, ne, same⟩
      by_cases before : x < y
      · exact invariant_6 x xlo xhi y ylo yhi (by omega) before hy same
      · exact invariant_6 y ylo yhi x xlo xhi (by omega) (by omega) hx same.symm
  | invariant_6_step =>
    verex_prepare only
    by_cases before : x < i.val
    · exact invariant_6 x (by omega) (by omega) y (by omega) (by omega) before (by omega) (by omega)
    · have same : x = i.val := by omega
      subst x
      exact invariant_4 y (by omega) (by omega) (by omega) (by omega)
  | invariant_4_step =>
    verex_prepare only
    by_cases before : y < j.val
    · exact invariant_4 y (by omega) (by omega) (by omega) before
    · have same : y = j.val := by omega
      subst y
      grind only [Option.getD_some]

  | _ => verex_decide
