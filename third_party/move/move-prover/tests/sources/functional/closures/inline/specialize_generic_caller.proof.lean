-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

open VerexIR.Proofs.Denote

-- Inlining count_all does not import its proof block. Instantiate its proved
-- count lemma at u64 to discharge the caller's entry, exit, and back edge.
verify use_concrete by
  verex_obligations
  all_goals (first | verex_decide | (verex_prepare; all_goals try verex_decide))
  all_goals
    have counts (n : Int) (lo : 0 ≤ n) (hi : n ≤ (v.values.size : Int)) :
        «spec_fold$lambda$0».spec
          (RuntimeValue.vector
            (v.values.map (Proofs.Codec.specInt (.bits 64) false).encode),
            0, n, ()) = n := by
      have fact := count_is_len.lemma unit
        (Skolems.instantiate
          ⟨.cons (.int 64 false) .nil, by decide⟩
          (Skolems.runtime unit))
        (RuntimeValue.vector
          (v.values.map (Proofs.Codec.specInt (.bits 64) false).encode)) n (by
            simp only [count_is_len.lemmaRequires, Proofs.Obligation_iff]
            refine ⟨⟨lo, ?_⟩, ?_, ?_⟩
            · have bound := v.bounded
              omega
            · exact ⟨v, rfl⟩
            · simpa only [vir_denote_norm] using hi)
      simpa only [count_is_len.lemmaEnsures, Proofs.Obligation_iff] using fact
    first
    | exact (counts 0 (by omega) (by omega)).symm
    | (have current := counts i.val (by omega) (by omega)
       first
       | omega
       | (have next := counts (i.val + 1) (by omega) (by omega)
          omega))
