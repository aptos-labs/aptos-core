-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- `Counter` is the scope of an inherent impl, not a module.
mod Counter where
  spec get where
    ensures result == self.value
