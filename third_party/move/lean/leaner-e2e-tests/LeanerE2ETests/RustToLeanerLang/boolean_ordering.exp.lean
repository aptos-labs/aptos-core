-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner namespace boolean_ordering where
  fun ordering(left : Bool, right : Bool) -> (Bool, Bool, Bool, Bool) :=
    (left < right, left <= right, left > right, left >= right)
