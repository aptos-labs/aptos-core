-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace generic where
  @[refused = "call of `generic::Step::step` in `generic::step_twice` at line 9:5 depends on a type parameter and needs trait evidence: rustc selects its implementation only at a concrete type"]
  native fun step_twice {T}(value : &mut T) -> Unit

-- no denotation for `step_twice`: a function without a body is not carried by the denotation
