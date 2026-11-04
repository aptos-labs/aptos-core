-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace bitwise_not where
  fun invert_unsigned(value : u8) -> u8 := ~value

  fun invert_signed(value : i8) -> i8 := ~value

-- no denotation for `invert_unsigned`: primitive VerexIR.PrimitiveOperation.bitwiseNot at this type or arity is not carried by the denotation
-- no denotation for `invert_signed`: primitive VerexIR.PrimitiveOperation.bitwiseNot at this type or arity is not carried by the denotation
