-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.Certified
import Proofs.Traversal

/-!
# Soundness of the model (D1): the shared imports

UNTRUSTED (checked by the kernel). The soundness argument from certificates
(`Proofs.Certified`) with the scheduler's facts (`Proofs.Traversal`). The
model's certificates for the whole supported subset are in
`Proofs.SoundnessGen` (`certified_of_wellFormed_gen`), with the results for
programs by value and their non-generic milestones
(`certified_of_wellFormed`, `soundness_byValue`, `certificatesConsistent`,
`certificatesHold_byValue`) as corollaries.
-/
