-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

open VerexIR.Proofs.Denote

-- Decode the option payload, then use the byte-vector codec's round trip.
-- The scalar operations themselves retain their existing native contracts.
verify new_scalar_reduced_from_32_bytes by
  verex_obligations
  all_goals (first | verex_decide | (verex_prepare; all_goals try verex_decide))
  all_goals
    simp_all [variantPayload,
      Option.bind_eq_some_iff,
      variantCarrier.first, variantCarrier.later]
  all_goals
    apply boundedVector_tight
      (specInt_tight (.bits 8) false)
    grind

verify new_scalar_uniform_from_64_bytes by
  verex_obligations
  all_goals (first | verex_decide | (verex_prepare; all_goals try verex_decide))
  all_goals
    simp_all [variantPayload,
      Option.bind_eq_some_iff,
      variantCarrier.first, variantCarrier.later]
  all_goals
    apply boundedVector_tight
      (specInt_tight (.bits 8) false)
    grind

verify scalar_invert by
  verex_obligations
  all_goals (first | verex_decide | (verex_prepare; all_goals try verex_decide))
  all_goals
    simp_all [variantPayload,
      Option.bind_eq_some_iff,
      variantCarrier.first, variantCarrier.later]
  all_goals
    apply boundedVector_tight
      (specInt_tight (.bits 8) false)
    grind
