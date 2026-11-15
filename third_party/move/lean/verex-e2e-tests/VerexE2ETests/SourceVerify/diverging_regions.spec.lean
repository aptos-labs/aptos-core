-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- A region is one panic, permitted by a clause. The contract of a diverging
-- callee is not consulted: `spin` states none.

spec die where
  panics_if true

spec own_function where
  panics_if c == 0
  ensures result == c

spec spinning_function where
  panics_if c == 0
  ensures result == c

spec explicit where
  panics_if c == 0
  ensures result == c

spec process_abort where
  panics_if c == 0
  ensures result == c

spec computes where
  panics_if c == 0
  ensures result == c

spec nested where
  panics_if a && b
  ensures result == (if a then 1 else 0)

spec arm where
  panics_if color is Red
  ensures color is Green ==> result == 1
  ensures color is Blue ==> result == 2

-- The impossible default of the switch owes no panic.
spec no_region where
  ensures result <= 2

spec loop_exit where
  panics_if false
  ensures result == limit
  loop 1 where
    invariant count <= limit

spec only_exit where
  panics_if c
  ensures false
  loop 1 where
    invariant true
