-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- rustc folds `step` away, so that no local has its name.
spec inner_local where
  loop 1 where
    invariant step <= n
