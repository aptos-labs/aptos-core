-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "folds_of_map_intrinsic.move"
verex proofs where

open VerexIR.Maps VerexIR.Proofs VerexIR.Proofs.Denote

-- The old prefix remains safe, and the current removal extends it by one.
verify remove_all_must_exist by
  verex_obligations
  all_goals verex_prepare only
  case invariant_3_entry_2 =>
    conv => rhs; rw [«spec_fold$lambda$2».spec.unfold]
    simp only [vir_denote_norm]
    simp only [if_pos (by decide : (0 : Int) ≤ 2 ^ 64 - 1)]
  case ensures_1 =>
    simpa only [show i.val = (v.values.size : Int) by omega] using invariant_3_1
  case invariant_3_step_1 =>
    by_cases last : _j_20 = i.val
    · subst _j_20
      rw [← invariant_3_1]
      obtain ⟨entry, read, present⟩ : ∃ entry : SpecInt (.bits 64) false,
          v.values[i.val.toNat]? = some entry ∧ hasKey mm (.integer entry.val) = true :=
        ⟨_, by assumption, by assumption⟩
      simpa only [read, Option.getD_some] using present
    · exact invariant_3_2 _j_20 (by omega) (by omega)
  case invariant_3_step_2 =>
    obtain ⟨entry, read, removed⟩ : ∃ entry : SpecInt (.bits 64) false,
        v.values[i.val.toNat]? = some entry ∧ mm' = remove
          _ .sequence mm (.integer entry.val) := ⟨_, by assumption, by assumption⟩
    rw [removed, invariant_3_1]
    conv => rhs; rw [«spec_fold$lambda$2».spec.unfold]
    simp only [vir_denote_norm]
    have fits : 0 ≤ i.val + 1 ∧ i.val + 1 ≤ 2 ^ 64 - 1 := by omega
    simp only [if_pos fits, if_neg (show i.val + 1 ≠ 0 by omega), Int.add_sub_cancel,
      read, Option.map_some, Option.getD_some, specInt_encode, RuntimeValue.asInt]
  all_goals verex_decide

verify remove_all by
  verex_obligations
  all_goals verex_prepare only
  case invariant_3_entry_2 =>
    conv => rhs; rw [«spec_fold$lambda$2».spec.unfold]
    simp only [vir_denote_norm]
    simp only [if_pos (by decide : (0 : Int) ≤ 2 ^ 64 - 1)]
  case ensures_1 =>
    simpa only [show i.val = (v.values.size : Int) by omega] using invariant_3_1
  case invariant_3_step_1 =>
    by_cases last : _j_16 = i.val
    · subst _j_16
      rw [← invariant_3_1]
      obtain ⟨entry, read, present⟩ : ∃ entry : SpecInt (.bits 64) false,
          v.values[i.val.toNat]? = some entry ∧ hasKey mm (.integer entry.val) = true :=
        ⟨_, by assumption, by assumption⟩
      simpa only [read, Option.getD_some] using present
    · exact invariant_3_2 _j_16 (by omega) (by omega)
  case invariant_3_step_2 =>
    obtain ⟨entry, read, removed⟩ : ∃ entry : SpecInt (.bits 64) false,
        v.values[i.val.toNat]? = some entry ∧ mm' = remove
          _ .sequence mm (.integer entry.val) := ⟨_, by assumption, by assumption⟩
    rw [removed, invariant_3_1]
    conv => rhs; rw [«spec_fold$lambda$2».spec.unfold]
    simp only [vir_denote_norm]
    have fits : 0 ≤ i.val + 1 ∧ i.val + 1 ≤ 2 ^ 64 - 1 := by omega
    simp only [if_pos fits, if_neg (show i.val + 1 ≠ 0 by omega), Int.add_sub_cancel,
      read, Option.map_some, Option.getD_some, specInt_encode, RuntimeValue.asInt]
  all_goals verex_decide

verify collect_distinct by
  verex_obligations
  all_goals verex_prepare only
  case invariant_3_entry_2 =>
    conv => rhs; rw [«spec_fold$lambda$1».spec.unfold]
    simp only [vir_denote_norm]
    simp only [if_pos (by decide : (0 : Int) ≤ 2 ^ 64 - 1)]
  case ensures_1 =>
    simpa only [show i.val = (v.values.size : Int) by omega, new_result_eq] using invariant_3_1
  case invariant_3_step_1 =>
    by_cases last : _j_8 = i.val
    · subst _j_8
      rw [← invariant_3_1]
      obtain ⟨entry, read, present⟩ : ∃ entry : SpecInt (.bits 64) false,
          v.values[i.val.toNat]? = some entry ∧ hasKey m (.integer entry.val) = false :=
        ⟨_, by assumption, by assumption⟩
      simpa only [read, Option.getD_some] using present
    · exact invariant_3_2 _j_8 (by omega) (by omega)
  case invariant_3_step_2 =>
    obtain ⟨entry, read, removed⟩ : ∃ entry : SpecInt (.bits 64) false,
        v.values[i.val.toNat]? = some entry ∧ m' = update
          _ .sequence m (.integer entry.val) (.integer 0) := ⟨_, by assumption, by assumption⟩
    rw [removed, invariant_3_1]
    conv => rhs; rw [«spec_fold$lambda$1».spec.unfold]
    simp only [vir_denote_norm]
    have fits : 0 ≤ i.val + 1 ∧ i.val + 1 ≤ 2 ^ 64 - 1 := by omega
    simp only [if_pos fits, if_neg (show i.val + 1 ≠ 0 by omega), Int.add_sub_cancel,
      read, Option.map_some, Option.getD_some, specInt_encode, RuntimeValue.asInt]
  all_goals verex_decide
