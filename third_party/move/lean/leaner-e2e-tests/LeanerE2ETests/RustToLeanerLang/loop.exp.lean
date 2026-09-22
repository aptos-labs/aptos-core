-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner namespace «loop» where
  fun clear(mut flag : Bool) -> Bool := do
    while flag do
      flag := false
      continue
    return flag
