-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.MutableContained

/-!
# D4a for verified programs

UNTRUSTED (checked by the kernel). D4a (`NoDanglingUse`) for the programs the
model accepts, at milestone M3, directly from D1's invariant: a step uses only
references the analysis tracks (`heldFacts_tracked`), and every reference the
analysis tracks is valid (`tracked_refOk`). It also follows from D4b′, D4c and
D4e (`noDanglingFromContainment`).
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model

/-- **D4a for M3 programs**: `NoDanglingUse` for the programs the model
accepts, on the supported subset, whose function handles are no struct-API
mutable borrows. -/
def NoDanglingUseM3 (V : VerifierInterface) : Prop :=
  ∀ (P : Program) (N : NativeRel) (ν : NativeFlows) (s₀ : State),
    WellFormedBy V VerifierConfig.production P → P.SupportedVersions → P.SupportedNonGeneric → P.M3 →
    NativesRespect P N → AdmissibleInitial P s₀ → NoDanglingUse P N ν s₀

theorem noDanglingUse_M3 : NoDanglingUseM3 verifier := by
  intro P N ν s₀ hwf _ hs hm3 hN h0 s h evs hT o post evs' hS
  have hinv := invM3_reachable hwf hs hm3 (certified_of_wellFormed schedulerSound hwf hs) hN h0 s
    (trace_reachable hT)
  unfold RefSafety.usesValid
  rw [List.all_eq_true]
  intro e he
  cases e
  case used i r =>
    have ht := (heldFacts_tracked (ν := ν) hwf hs hm3 hN h0).used s h evs hT o post evs' hS i r he
    obtain ⟨fr, -, hok⟩ := tracked_refOk hwf hs hm3 hinv ht
    exact hok.valid
  all_goals rfl

end MoveBytecode.Proofs
