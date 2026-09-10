-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
-- Test category: expected failures and diagnostics.

import Move

/-! Diagnostics for specifications which do not denote valid resource or
pre-state observations. -/

open Move
open scoped Move Move.Spec

module Tests.Negative.Specifications where

  fun unchanged (address : Address) : Action Bool := pure (address == address)

  /--
  error: `existsAt<…>` expects a resource type
  -/
  #guard_msgs in
  spec unchanged (address : Address) where
    ensures existsAt<U64>(address);
    aborts_if False

  /--
  error: `old` expects a global resource place
  -/
  #guard_msgs in
  spec unchanged (address : Address) where
    ensures old(address) = address;
    aborts_if False

  /--
  error: `modifies` expects a resource family
  -/
  #guard_msgs in
  spec unchanged (address : Address) where
    modifies U64[address];
    ensures True;
    aborts_if False
