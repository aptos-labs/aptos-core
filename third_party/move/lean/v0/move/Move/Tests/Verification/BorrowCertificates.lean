-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move

/-!
# Source borrow certificates in generated proofs

These tests exercise the retained-source extractor, summary transport, and
the proof-facing declarations emitted beside a `sourceSpec`.
-/

namespace Move.Tests.Verification.BorrowCertificates

open Move
open scoped Move Move.Spec

fun identity_ref (input : &U64) : Action (&U64) := do
  pure input

spec identity_ref (input : U64) where
  ensures result = input

#guard identity_ref.borrowProgram.summary.returns == #[{
  parameter := 0
  kind := .immutable
  phase := .unactivated }]

example : Move.Verify.Borrow.WellBorrowed identity_ref.borrowProgram :=
  identity_ref.wellBorrowed

end Move.Tests.Verification.BorrowCertificates
