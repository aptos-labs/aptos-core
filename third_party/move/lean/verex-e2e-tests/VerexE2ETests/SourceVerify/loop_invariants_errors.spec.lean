-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- A wrong invariant fails at its clause in this file.
spec wrong_step where
  loop 1 where
    invariant i <= n
    invariant i == 0

spec wrong_entry where
  loop 1 where
    invariant i <= n
    invariant i == 1

spec wrong_inner where
  loop 1 where
    invariant row <= rows
  loop 2 where
    invariant column <= columns
    invariant column == 0

spec overflow where
  panics_if false
  loop 1 where
    invariant i <= n

-- A failure in a loop's body is reported at the Rust source, whichever way the
-- rendering holds the loop's invariants.
spec positive where
  requires x > 0

spec call_in_exiting_loop where
  loop 1 where
    invariant i <= n

spec call_in_while where
  loop 1 where
    invariant i <= n
