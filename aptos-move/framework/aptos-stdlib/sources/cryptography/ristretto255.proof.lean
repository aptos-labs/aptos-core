-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- Decode the option payload, then use the byte-vector codec's round trip.
-- The scalar operations themselves retain their existing native contracts.
verify new_scalar_reduced_from_32_bytes by
  verex_obligations
  all_goals (first | verex_decide | (verex_prepare; all_goals try verex_decide))
  all_goals
    simp_all [VerexIR.Proofs.Denote.variantPayload,
      Option.bind_eq_some_iff,
      VerexIR.Proofs.Denote.variantCarrier.first, VerexIR.Proofs.Denote.variantCarrier.later]
  all_goals
    apply VerexIR.Proofs.Denote.boundedVector_tight
      (VerexIR.Proofs.Denote.specInt_tight (.bits 8) false)
    grind

verify new_scalar_uniform_from_64_bytes by
  verex_obligations
  all_goals (first | verex_decide | (verex_prepare; all_goals try verex_decide))
  all_goals
    simp_all [VerexIR.Proofs.Denote.variantPayload,
      Option.bind_eq_some_iff,
      VerexIR.Proofs.Denote.variantCarrier.first, VerexIR.Proofs.Denote.variantCarrier.later]
  all_goals
    apply VerexIR.Proofs.Denote.boundedVector_tight
      (VerexIR.Proofs.Denote.specInt_tight (.bits 8) false)
    grind

verify scalar_invert by
  verex_obligations
  all_goals (first | verex_decide | (verex_prepare; all_goals try verex_decide))
  all_goals
    simp_all [VerexIR.Proofs.Denote.variantPayload,
      Option.bind_eq_some_iff,
      VerexIR.Proofs.Denote.variantCarrier.first, VerexIR.Proofs.Denote.variantCarrier.later]
  all_goals
    apply VerexIR.Proofs.Denote.boundedVector_tight
      (VerexIR.Proofs.Denote.specInt_tight (.bits 8) false)
    grind
