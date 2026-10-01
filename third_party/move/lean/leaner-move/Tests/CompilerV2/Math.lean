-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerMove

/-! A Leaner Move library another Leaner module calls: compiler-v2 loads it
as a dependency of `Client.lean`. -/

leaner module 0x42::Math where
  public fun identity {T}(value : T) -> T := value
  spec identity where
    ensures result == value
