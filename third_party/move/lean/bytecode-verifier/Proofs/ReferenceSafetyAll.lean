-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.HeldAll
import Proofs.StepsAll
import Proofs.EscapeAll
import Proofs.ReferenceSafetyM3

/-!
# D4: reference safety

UNTRUSTED (checked by the kernel). D4 for every program the model accepts on
the supported subset, generic code included (`referenceSafety`): the ghost
layer's accounting, and D4a–e, each from its one-step form over the
references the analysis tracks, with the struct-API mutable borrows, generic
or not: the call and the wrapper's steps access only the wrapper's own local,
inspect and borrow through its argument, and return a reference derived from
it. The non-generic subset is a corollary (`referenceSafety_nonGeneric`).
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model

/-- **D4, reference safety**, for every program the model accepts on the
supported subset. -/
theorem referenceSafety : ReferenceSafety verifier := by
  intro P N ν π s₀ hwf hv hs hN hown h0
  have hapi := apiVersions_of hv
  have hH := heldFacts_tracked' (ν := ν) hwf hs hapi hN h0
  exact ⟨shadowAccounts_admissible hown h0, noDanglingUse_all hwf hs hapi hN h0,
    mutableContained_of_steps h0 hH (stepsContained_tracked' hwf hs hapi hN h0),
    parametersExclusive_all hwf hs hapi hN h0,
    immutableStable_of_steps h0 hH (fun _ _ _ _ _ _ _ hS => traced_borrowsInside hS)
      (stepsStable_tracked' hwf hs hapi hN h0),
    noEscape_all hwf hs hapi hN h0,
    deadSlots_of_steps h0 hH (fun _ _ _ _ _ _ _ hS => traced_borrowsInside hS)
      (stepsDead_tracked' hwf hs hapi hN h0)⟩

/-- **D4, reference safety**, for every program the model accepts on the
non-generic subset, a corollary. -/
theorem referenceSafety_nonGeneric : ReferenceSafetyNonGeneric verifier := by
  intro P N ν π s₀ hwf hv hs hN hown h0
  exact referenceSafety P N ν π s₀ hwf hv (supported_of_nonGeneric hwf hs) hN hown h0

end MoveBytecode.Proofs
