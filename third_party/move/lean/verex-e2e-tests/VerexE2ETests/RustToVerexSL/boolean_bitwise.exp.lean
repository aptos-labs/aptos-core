-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace boolean_bitwise where
  fun bitwise(left : Bool, right : Bool) -> (Bool, Bool, Bool) :=
    (left & right, left | right, left ^ right)

-- no denotation for `bitwise`: primitive VerexIR.PrimitiveOperation.bitwiseAnd at this type or arity is not carried by the denotation
