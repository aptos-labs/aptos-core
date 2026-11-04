-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace array_repeat where
  fun repeat(value : u32) -> Vector<u32, const 4> := #[value; 4]
