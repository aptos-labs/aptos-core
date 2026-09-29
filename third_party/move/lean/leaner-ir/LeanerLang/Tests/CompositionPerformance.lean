-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
# Modular-call scaling

One, two, and four calls to the same verified mutable callee, beside an
untouched mutable parameter. Keep the bodies fixed: this checks accumulated
call facts, separately from the feature-port corpus.
-/

namespace LeanerLang.Tests.CompositionPerformance

set_option Elab.async false
set_option maxHeartbeats 180000
set_option leaner.verifyHeartbeats 180000

#leaner_measure

leaner module 0x42::composition_perf where
  fun set_seven(slot : &mut u64) -> Unit := *slot := 7
  spec set_seven where
    ensures *slot == 7
    aborts_if false

  fun once(untouched : &mut u64, value : u64) -> u64 := do
    let mut local := value
    set_seven(&mut local)
    local
  spec once where
    ensures result == 7 && *untouched == old(*untouched)
    aborts_if false

  fun twice(untouched : &mut u64, value : u64) -> u64 := do
    let mut local := value
    set_seven(&mut local)
    set_seven(&mut local)
    local
  spec twice where
    ensures result == 7 && *untouched == old(*untouched)
    aborts_if false

  fun four_times(untouched : &mut u64, value : u64) -> u64 := do
    let mut local := value
    set_seven(&mut local)
    set_seven(&mut local)
    set_seven(&mut local)
    set_seven(&mut local)
    local
  spec four_times where
    ensures result == 7 && *untouched == old(*untouched)
    aborts_if false

#leaner_perf "LeanerLang/Tests/CompositionPerformance.exp"

end LeanerLang.Tests.CompositionPerformance
