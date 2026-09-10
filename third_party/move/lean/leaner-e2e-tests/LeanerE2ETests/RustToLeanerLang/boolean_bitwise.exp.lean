-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner namespace boolean_bitwise where
  fun bitwise(left : Bool, right : Bool) -> (Bool, Bool, Bool) :=
    (left & right, left | right, left ^ right)
