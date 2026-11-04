-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- A region is one panic, permitted by a clause. The contract of a diverging
-- callee is not consulted: `spin` states none.

spec die where
  aborts_if true

spec own_function where
  aborts_if c == 0
  ensures result == c

spec spinning_function where
  aborts_if c == 0
  ensures result == c

spec explicit where
  aborts_if c == 0
  ensures result == c

spec process_abort where
  aborts_if c == 0
  ensures result == c

spec computes where
  aborts_if c == 0
  ensures result == c

spec nested where
  aborts_if a && b
  ensures result == (if a then 1 else 0)

spec arm where
  aborts_if color is Red
  ensures color is Green ==> result == 1
  ensures color is Blue ==> result == 2

-- The impossible default of the switch owes no panic.
spec no_region where
  ensures result <= 2

spec loop_exit where
  aborts_if false
  ensures result == limit
  loop 1 where
    invariant count <= limit

spec only_exit where
  aborts_if c
  ensures false
  loop 1 where
    invariant true
