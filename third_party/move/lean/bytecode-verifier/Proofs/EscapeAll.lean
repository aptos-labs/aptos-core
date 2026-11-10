-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.Exclusive
import Proofs.ParamFacts
import Proofs.ExclusiveM3
import Proofs.HeldAll

/-!
# D4d and D4b″ for verified programs

UNTRUSTED (checked by the kernel). D4d (`noEscape_all`, in
`Proofs/ReturnsDerived.lean`) and D4b″ (`parametersExclusive_all`) for every
program the model accepts on the supported subset, generic code and struct-API
mutable borrows included. While a struct-API wrapper runs (`ApiCore`, generic
or not) it runs only its own
code: it holds its argument or the field borrow of it, hands back the field
borrow, rooted below it, borrows nothing without a parent and acquires
nothing (`Proofs/EscapeApi.lean`); its one parameter is the caller's
argument, which the caller tracks (`paramChain_callApi`).
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model

/-- **D4b″ for verified programs**: while a frame is active, no access
conflicts with a live `&mut` reference parameter of the frame. -/
theorem parametersExclusive_all {P : Program} {N : NativeRel} {ν : NativeFlows} {s₀ : State}
    (hwf : WellFormedBy verifier VerifierConfig.production P) (hs : P.Supported)
    (hapi : ∀ m ∈ P.modules.toList, ∀ fh ∈ m.functionHandles.toList, ∀ off,
      ReferenceSafety.borrowFieldMutable fh.attributes = some off → 10 ≤ m.version)
    (hN : NativesRespect P N) (h0 : AdmissibleInitial P s₀) : ParametersExclusive P N ν s₀ :=
  parametersExclusive_of_steps h0 (heldFacts_tracked' hwf hs hapi hN h0)
    (fun _ _ _ _ _ _ _ hS => traced_borrowsInside hS)
    (stepsExclusive' hwf hs hapi hN h0 fun _ _ _ hT => paramChain_trace' hwf hs hapi hN h0 hT)

end MoveBytecode.Proofs
