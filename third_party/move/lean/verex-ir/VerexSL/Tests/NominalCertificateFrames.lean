-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

-- This test asserts what the Given route does; it selects the route.
set_option verex.givenRoute true


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
-/
#guard_msgs in
verify 0x42::generic_certificate_frames::read
