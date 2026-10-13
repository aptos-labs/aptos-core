-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-! A defined invocation label must not assume a successful execution. -/

namespace VerexSL.Tests.Check.Specifications.InvocationStateLabelErrors

verex module 0x42::invocation_state_label_errors where
  struct R has Key where
    value : u64

  fun fail() -> u64 := abort(1)
  spec fail where
    pragma opaque
    aborts_if true
    ensures false

  fun caller(addr : Address) -> u64 := fail()
  spec caller where
    aborts_if false -- error: the call always aborts
    ensures do
      let value := ..S |~ result_of<function[Fn() -> u64](fail)>()
      (S |~ exists<R>(addr)) && result == value

end VerexSL.Tests.Check.Specifications.InvocationStateLabelErrors
