-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- Each holds only if the `return` did not leave: then the loop runs on and no
-- result follows, so the claim would be vacuous.
spec in_while where
  ensures a && !b && c ==> result == 2
  loop 1 where
    invariant x <= 2

spec after_a_branch where
  ensures a && !b && c ==> result == 2
  loop 1 where
    invariant x <= 2
