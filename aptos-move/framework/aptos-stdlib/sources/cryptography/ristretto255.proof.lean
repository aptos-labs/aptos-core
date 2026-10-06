-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- Decode the option payload, then use the byte-vector codec's round trip.
-- The scalar operations themselves retain their existing native contracts.
verify new_scalar_reduced_from_32_bytes by
  all_goals
    simp_all [LeanerIR.Proofs.Denote.variantPayload,
      Option.bind_eq_some_iff,
      LeanerIR.Proofs.Denote.variantCarrier.first, LeanerIR.Proofs.Denote.variantCarrier.later]
  all_goals
    apply LeanerIR.Proofs.Denote.boundedVector_tight
      (LeanerIR.Proofs.Denote.specInt_tight (.bits 8) false)
    grind

verify new_scalar_uniform_from_64_bytes by
  all_goals
    simp_all [LeanerIR.Proofs.Denote.variantPayload,
      Option.bind_eq_some_iff,
      LeanerIR.Proofs.Denote.variantCarrier.first, LeanerIR.Proofs.Denote.variantCarrier.later]
  all_goals
    apply LeanerIR.Proofs.Denote.boundedVector_tight
      (LeanerIR.Proofs.Denote.specInt_tight (.bits 8) false)
    grind

verify scalar_invert by
  all_goals
    simp_all [LeanerIR.Proofs.Denote.variantPayload,
      Option.bind_eq_some_iff,
      LeanerIR.Proofs.Denote.variantCarrier.first, LeanerIR.Proofs.Denote.variantCarrier.later]
  all_goals
    apply LeanerIR.Proofs.Denote.boundedVector_tight
      (LeanerIR.Proofs.Denote.specInt_tight (.bits 8) false)
    grind
