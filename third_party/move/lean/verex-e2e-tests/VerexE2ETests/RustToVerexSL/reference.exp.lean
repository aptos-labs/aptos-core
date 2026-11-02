-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace reference where
  fun read(value : &u32) -> u32 := *value
