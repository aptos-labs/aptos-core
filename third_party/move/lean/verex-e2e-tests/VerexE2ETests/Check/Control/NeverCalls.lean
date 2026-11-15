-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-!
# A call of a function returning `!`

Nothing follows such a call: a caller owes the callee's abort and nothing
after it.
-/

verex namespace never_calls where
  fun stop() -> Never := abort(7)
  spec stop where
    panics_if true

  fun after(value : u64) -> u64 := do
    stop();
    value
  spec after where
    panics_if true
    ensures result == value + 1
