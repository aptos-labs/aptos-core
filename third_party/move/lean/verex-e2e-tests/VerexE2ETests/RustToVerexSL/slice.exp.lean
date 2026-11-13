-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace slice where
  fun first(values : &Slice<u32>) -> u32 := values[0usize]
