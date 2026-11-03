-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.RefStableSteps
import Proofs.RefDeadSteps
import Proofs.BorrowInside
import Proofs.DanglingM3

/-!
# D4c and D4e for verified programs

UNTRUSTED (checked by the kernel). D4c (`ImmutableStable`) and D4e
(`DeadSlotsUnborrowed`) for the programs the model accepts, at milestone M3,
from an admissible initial state, whatever the natives do within their
contracts: the references the analysis tracks are held (`heldFacts_tracked`),
derived references are inside the ones they derive from
(`traced_borrowsInside`), and each step keeps writes off tracked `&`
references (`stepsStable_tracked`) and ends no slot a tracked reference
points into (`stepsDead_tracked`).
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model

/-- **D4c for M3 programs**: `ImmutableStable` for the programs the model
accepts, on the supported subset, whose function handles are no struct-API
mutable borrows. -/
def ImmutableStableM3 (V : VerifierInterface) : Prop :=
  ∀ (P : Program) (N : NativeRel) (ν : NativeFlows) (s₀ : State),
    WellFormedBy V VerifierConfig.production P → P.SupportedVersions → P.SupportedNonGeneric → P.M3 →
    NativesRespect P N → AdmissibleInitial P s₀ → ImmutableStable P N ν s₀

theorem immutableStable_M3 : ImmutableStableM3 verifier := by
  intro P N ν s₀ hwf _ hs hm3 hN h0
  exact immutableStable_of_steps h0 (heldFacts_tracked hwf hs hm3 hN h0)
    (fun _ _ _ _ _ _ _ hS => traced_borrowsInside hS) (stepsStable_tracked hwf hs hm3 hN h0)

/-- **D4e for M3 programs**: `DeadSlotsUnborrowed` for the programs the model
accepts, on the supported subset, whose function handles are no struct-API
mutable borrows. -/
def DeadSlotsUnborrowedM3 (V : VerifierInterface) : Prop :=
  ∀ (P : Program) (N : NativeRel) (ν : NativeFlows) (s₀ : State),
    WellFormedBy V VerifierConfig.production P → P.SupportedVersions → P.SupportedNonGeneric → P.M3 →
    NativesRespect P N → AdmissibleInitial P s₀ → DeadSlotsUnborrowed P N ν s₀

theorem deadSlotsUnborrowed_M3 : DeadSlotsUnborrowedM3 verifier := by
  intro P N ν s₀ hwf _ hs hm3 hN h0
  exact deadSlots_of_steps h0 (heldFacts_tracked hwf hs hm3 hN h0)
    (fun _ _ _ _ _ _ _ hS => traced_borrowsInside hS) (stepsDead_tracked hwf hs hm3 hN h0)

end MoveBytecode.Proofs
