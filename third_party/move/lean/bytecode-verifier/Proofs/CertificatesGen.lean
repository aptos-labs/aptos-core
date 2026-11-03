-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.TypingGen
import Proofs.Soundness

/-!
# C1 and C2 on the supported subset, generic code included

UNTRUSTED (checked by the kernel). On the whole supported subset, generic
code and closures included, the model's type certificate is consistent, up to
the assignability of closure types, at every instance whose type arguments
satisfy its function's constraints (`modelTypes_consistent_gen`), and is
expressed over each function's type parameters (`modelTypes_parametric`);
the availability certificate, which depends on no type, is consistent
(`modelAvail_consistent`).
-/

namespace MoveBytecode.Proofs

open MoveBytecode.Model

/-- **C1, C2** (static) on the supported subset, generic code included: the
model's type certificate is consistent and parametric, and its availability
certificate consistent. -/
theorem certificatesConsistent_gen (P : Program)
    (hwf : WellFormedBy verifier VerifierConfig.production P) (_ : P.SupportedVersions)
    (hs : P.Supported) :
    (modelTypes P).Consistent P ∧ (modelTypes P).Parametric P ∧ (modelAvail P).Consistent P :=
  ⟨modelTypes_consistent_gen hwf hs, modelTypes_parametric P,
    modelAvail_consistent schedulerSound hwf⟩

end MoveBytecode.Proofs
