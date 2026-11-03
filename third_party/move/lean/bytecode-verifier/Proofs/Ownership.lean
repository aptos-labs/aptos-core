-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.ModeledAnnotated

/-!
# D2 and D3

UNTRUSTED (checked by the kernel). Ownership accounting (D2) and
encapsulation and unforgeability (D3) for the model's verifier, in the
statements' shapes (`Spec.Properties.Theorems`).

Neither uses the verifier's acceptance: both follow from the semantics, the
native contracts (`NativesRespectOwnership`) and the transaction premises
(`EntryInitial`, E1 and E2). The ghost layer annotates every step that
continues (`Proofs.GhostExec`, `Proofs.GhostNative`), the trace invariant
holds at a transaction's initial state (`Proofs.GhostInitial`) and after
every annotated step (`Proofs.Accounting`), and D2 and D3 follow from it.
For verified programs, D1 (`Proofs.Soundness`) excludes `wrong`, so every
step of a verified program's execution either fails, rolling back, or is
accounted.
-/

namespace MoveBytecode.Proofs

/-- **D2**: ownership accounting. -/
theorem ownershipAccounting : OwnershipAccounting verifier := by
  intro P N ν π auth A s₀ _ _ _ _ hown hE
  exact ownershipAccounted_of (A := A) (auth := auth) (nativeAnnotated hown) (entry_inv hE)

/-- **D3**: encapsulation and unforgeability. -/
theorem encapsulation : Encapsulation verifier := by
  intro P N ν π auth A s₀ _ _ _ _ hown hE
  exact encapsulated_of (nativeAnnotated hown) (entry_inv hE)

end MoveBytecode.Proofs
