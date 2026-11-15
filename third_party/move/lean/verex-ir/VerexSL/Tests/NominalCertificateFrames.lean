-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

-- The invariant belongs to the generic read's frame. Its nonphantom field
-- ensures that selecting the declaration cannot identify the two frames by
-- their scalar fields alone.
verex module 0x42::generic_certificate_frames where
  pragma verify = false
  struct Tally {T has Store} has Key where
    count : u64
    payload : T
  spec Tally where
    invariant count > 0

  fun read {T has Store}(address : Address) -> u64 := Tally<T>[address].count
  spec read where
    aborts_if !exists<Tally<T> >(address)
    ensures result > 0

/--
info: `read` uses concrete verification: Given interpretation does not yet support storage interpretation
---
error: the specification clause `ensures result > 0` is not established
---
error: verex verification failed: the automatic verification of `read` failed; provide a proof in the module:
  verify read by lean =>
    verex_obligations
    verex_on
    | ensures_1 => verex_prepare; sorry
    | _ => verex_decide
  (`verex_prepare` states a leaf in source form; `trace_state` after it shows the goal)
-/
#guard_msgs in
verify 0x42::generic_certificate_frames::read
