-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

namespace LeanerLang.Tests.Check.Control.LoopVerification

leaner module 0x44::verification_loops where
  public fun count_to(limit : u64) -> u64 := do
    let mut current : u64 := 0
    loop if current < limit then do
      current := current + 1
    else break
    spec do
      invariant current <= limit
    current
  spec count_to where
    ensures result == limit
    aborts_if false

  public fun count_to_with_continue(limit : u64) -> u64 := do
    let mut current : u64 := 0
    loop if current < limit then do
      current := current + 1
      continue
    else break
    spec do
      invariant current <= limit
    current
  spec count_to_with_continue where
    ensures result == limit
    aborts_if false

end LeanerLang.Tests.Check.Control.LoopVerification
