-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace associated_generic_call where
  @[refused = "call of `std::ops::Index::index` in `associated_generic_call::first` at line 5:12 depends on a type parameter and needs trait evidence: rustc selects its implementation only at a concrete type"]
  native fun first {T}(values : &Vector<T>) -> &T

-- no denotation for `first`: a function without a body is not carried by the denotation
