-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean

/-!
# Route switches

The options that select a verification route, rather than a parameter of
one.  They are registered in a module of their own because an option cannot
be read in the module that registers it.
-/

/-- Verify a generic function through the Given interpretation, its proof
transferred to the concrete route (`doc/designs/given-parameters.md`).
Unset (the default), every function is verified concretely at its own
carriers. -/
register_option verex.givenRoute : Bool := {
  defValue := false
  descr := "verify eligible generic functions through the Given interpretation and \
    explicit transfer proposition rather than the concrete route"
}
