-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace unary where
  fun negate(value : i32) -> i32 := core.prim.checkedNegatePanic(value)
