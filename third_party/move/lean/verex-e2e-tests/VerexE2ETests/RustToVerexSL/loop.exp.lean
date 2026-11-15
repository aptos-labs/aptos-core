-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace «loop» where
  fun clear(mut flag : Bool) -> Bool := do
    while flag do
      flag := false
      continue
    flag
