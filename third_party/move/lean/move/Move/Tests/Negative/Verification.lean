-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
-- Test category: expected failures and diagnostics.

import Move

/-! Regression tests for proof failures that must surface as concise,
source-oriented diagnostics. -/

open Move
open scoped Move Move.Spec

module Tests.Negative.Verification where

  fun wrong_increment (value : U64) : U64 := value + 1

  spec wrong_increment (value : U64) where
    ensures result = value

  /--
  error: verification failed for `Tests.Negative.Verification.wrong_increment`: the implementation does not prove its contract; use `verify Tests.Negative.Verification.wrong_increment by` to inspect and prove the remaining obligation
  -/
  #guard_msgs in
  verify wrong_increment

  fun wrong_action (value : U64) : Action U64 := pure value

  spec wrong_action (value : U64) where
    ensures result = value + 1;
    aborts_if False

  /--
  error: verification failed for `Tests.Negative.Verification.wrong_action`: the implementation does not prove its contract; use `verify Tests.Negative.Verification.wrong_action by` to inspect and prove the remaining obligation
  -/
  #guard_msgs in
  verify wrong_action
