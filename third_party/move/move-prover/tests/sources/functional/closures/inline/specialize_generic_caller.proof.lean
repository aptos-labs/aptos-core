-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "specialize_generic_caller.move"
verex proofs where

open VerexIR.Proofs.Denote

-- Inlining count_all at u64 emits its loop invariant over the u64 fold
-- specialization without importing its proof block; prove the count by
-- induction on the prefix length to discharge entry, exit, and back edge.
verify use_concrete by lean =>
  verex_obligations
  all_goals (first | verex_decide | (verex_prepare; all_goals try verex_decide))
  all_goals
    have counts (n : Int) (lo : 0 ≤ n) (hi : n ≤ (v.values.size : Int)) :
        «spec_fold$lambda$0».at_u64.spec unit
          NominalPackage.package.readingTable callLayoutsComplete
          (Skolems.runtime unit).toCarriers
          (Skolems.runtime unit).type
          (v, 0, n, ()) = n := by
      have step : ∀ (k : Nat) (m : Int), m.toNat = k → 0 ≤ m → m ≤ (v.values.size : Int) →
          «spec_fold$lambda$0».at_u64.spec unit
            NominalPackage.package.readingTable callLayoutsComplete
            (Skolems.runtime unit).toCarriers
            (Skolems.runtime unit).type
            (v, 0, m, ()) = m := by
        have bound := v.bounded
        intro k
        induction k using Nat.strongRecOn with
        | _ k ih =>
          intro m hk hlo hhi
          rw [«spec_fold$lambda$0».at_u64.spec.unfold]
          simp only
          split
          · split
            · omega
            · have prev := ih (m - 1).toNat (by omega) (m - 1) rfl (by omega) (by omega)
              (try split) <;> (try rw [prev]) <;> omega
          · omega
      exact step n.toNat n rfl lo hi
    first
    | exact (counts 0 (by omega) (by omega)).symm
    | (have current := counts i.val (by omega) (by omega)
       first
       | omega
       | (have next := counts (i.val + 1) (by omega) (by omega)
          omega))
