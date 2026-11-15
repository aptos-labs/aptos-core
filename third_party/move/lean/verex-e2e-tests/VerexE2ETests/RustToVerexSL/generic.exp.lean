-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

verex namespace generic where
  @[method_evidence = 1]
  fun step_twice {T}(
    value : &mut T, __trait_Step_step : Fn(&mut T) -> Unit has Copy, Drop
  ) -> Unit := do
    invoke(__trait_Step_step, &mut *value)
    invoke(__trait_Step_step, &mut *value)
