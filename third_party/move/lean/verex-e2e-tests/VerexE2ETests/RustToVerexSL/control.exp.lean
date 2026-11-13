-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace control where
  fun choose(flag : Bool, when_true : u32, when_false : u32) -> u32 :=
    if flag then when_true else when_false
