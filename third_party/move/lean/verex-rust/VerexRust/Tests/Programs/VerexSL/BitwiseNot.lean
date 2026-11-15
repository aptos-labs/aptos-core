-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace bitwise_not where
  fun invert_unsigned(value : u8) -> u8 := ~value

  fun invert_signed(value : i8) -> i8 := ~value
