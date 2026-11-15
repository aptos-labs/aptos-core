-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace call where
  fun recurse(again : Bool, value : u32) -> u32 :=
    if again then recurse(false, value) else value
