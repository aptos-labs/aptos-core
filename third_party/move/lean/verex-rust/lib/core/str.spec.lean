-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

mod primitive where
  impl str where
    spec is_empty where
      ensures result == (_1.length == 0)

    spec as_bytes where
      ensures result == spec.stringBytes(_1)
      ensures result.length == _1.length
