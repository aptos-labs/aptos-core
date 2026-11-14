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
Unset, a generic function is verified exactly as one the Given
interpretation refuses is: concretely, at its own carriers.  The route is to
be parked until the concrete route is restored for generic functions
(`doc/plan/plain-values.md`, U2); until then it is what verifies them, so
the option is set by default. -/
register_option verex.givenRoute : Bool := {
  defValue := true
  descr := "verify a generic function through the Given interpretation and transfer its \
    proof to the concrete route; unset, every function is verified concretely"
}
