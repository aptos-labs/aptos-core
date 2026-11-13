-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- A possible panic owes `no panic`, reported at the `spec` item with its site.

spec divide where
  ensures true

spec guarded where
  ensures true

-- A clause permitting the panic replaces `no panic`, and keeps its number.
spec permitted where
  panics_if divisor == 0

spec wrong_permission where
  panics_if dividend == 0

spec sum where
  ensures result == left + right

spec calls_divide where
  ensures true

spec two_divisions where
  requires a != 0

-- One of two sites by its selector, the other by the default. A selector
-- no path takes lists the sites.
spec routed where
  requires a <= 10 && b <= 10

verify routed by
  on no panic when aborts at op 1 {
    assert b <= 10
  }
  on no panic when aborts at op 2 {
    assert a <= 10
  }

spec unrouted where
  requires a <= 10

verify unrouted by
  on no panic when aborts at op 2 {
  }
  on _ { }

-- A function without a `spec` item that is verified owes `no panic` at itself.
spec nonzero where
  requires a != 0

-- The form a failed verification asks for.
spec requested where
  requires a <= 10

verify requested by lean =>
  verex_obligations
  verex_on
  | no_panic when aborts at op 1 => verex_solve
  | _ => verex_decide

-- A site inside a loop.
spec in_loop where
  requires n <= 10
