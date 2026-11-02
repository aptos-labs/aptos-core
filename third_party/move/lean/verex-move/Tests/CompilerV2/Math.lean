-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove

/-! A Verex Move library another Verex module calls: compiler-v2 loads it
as a dependency of `Client.lean`. -/

verex module 0x42::Math where
  public fun identity {T}(value : T) -> T := value
  spec identity where
    ensures result == value
