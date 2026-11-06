-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.Exclusive
import Proofs.ParamFacts
import Proofs.ExclusiveM3
import Proofs.MutableContained
import Proofs.DanglingM3
import Proofs.ReferencesM3
import Proofs.ModeledAnnotated

/-!
# D4 for M3 programs

UNTRUSTED (checked by the kernel). D4b″ for M3 programs
(`parametersExclusive_M3`): it reduces to one step (`parametersExclusive_of_steps`),
where the tracked references and the frames' parameter facts give it
(`stepsExclusive_M3`, `paramChain_trace`). With D4a, D4b′, D4c, D4d and D4e,
and the ghost layer's accounting, this is reference safety for M3 programs
(`referenceSafety_M3`).
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model

/-- **D4b″ for M3 programs**: `ParametersExclusive` for the programs the model
accepts, on the supported subset, whose function handles are no struct-API
mutable borrows. -/
def ParametersExclusiveM3 (V : VerifierInterface) : Prop :=
  ∀ (P : Program) (N : NativeRel) (ν : NativeFlows) (s₀ : State),
    WellFormedBy V VerifierConfig.production P → P.SupportedVersions → P.SupportedNonGeneric → P.M3 →
    NativesRespect P N → AdmissibleInitial P s₀ → ParametersExclusive P N ν s₀

theorem parametersExclusive_M3 : ParametersExclusiveM3 verifier := by
  intro P N ν s₀ hwf _ hs hm3 hN h0
  exact parametersExclusive_of_steps h0 (heldFacts_tracked hwf hs hm3 hN h0)
    (fun _ _ _ _ _ _ _ hS => traced_borrowsInside hS)
    (stepsExclusive_M3 hwf hs hm3 hN h0 fun _ _ _ hT => paramChain_trace hwf hs hm3 hN h0 hT)

/-- **D4 for M3 programs**: `ReferenceSafety` with the premise `P.M3`. -/
def ReferenceSafetyM3 (V : VerifierInterface) : Prop :=
  ∀ (P : Program) (N : NativeRel) (ν : NativeFlows) (π : NativePolicy) (s₀ : State),
    WellFormedBy V VerifierConfig.production P → P.SupportedVersions → P.SupportedNonGeneric → P.M3 →
    NativesRespect P N → NativesRespectOwnership P N ν π → AdmissibleInitial P s₀ →
    ReferencesSafe P N ν s₀

/-- **Reference safety for M3 programs.** -/
theorem referenceSafety_M3 : ReferenceSafetyM3 verifier := by
  intro P N ν π s₀ hwf hv hs hm3 hN hown h0
  exact ⟨shadowAccounts_admissible hown h0, noDanglingUse_M3 P N ν s₀ hwf hv hs hm3 hN h0,
    mutableContained_M3 P N ν s₀ hwf hv hs hm3 hN h0,
    parametersExclusive_M3 P N ν s₀ hwf hv hs hm3 hN h0,
    immutableStable_M3 P N ν s₀ hwf hv hs hm3 hN h0, noEscape_M3 hwf hs hm3 hN h0,
    deadSlotsUnborrowed_M3 P N ν s₀ hwf hv hs hm3 hN h0⟩

end MoveBytecode.Proofs
