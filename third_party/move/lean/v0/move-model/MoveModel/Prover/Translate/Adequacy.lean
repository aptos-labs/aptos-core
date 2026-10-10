-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.Prover.Ivl.WpSound
import MoveModel.IR.Semantics
import MoveModel.Prover.Translate.Compile
import MoveModel.Prover.Translate.Sim

/-!
# End-to-End Adequacy

The final theorem of the layered architecture: if every function of a
well-typed program verifies — i.e. the Lean-side WP of its compiled program
holds for all boundary states — then every function satisfies its declared
contract.

The proof composes the results from `Sim.lean`.  Its master induction,
`sim_aux`, follows the big-step execution derivation.  This handles recursive
calls directly instead of inducting on a call graph.  The simulation carries
contract conformance and type preservation through the compiled execution.

`wpB_sound` and `wpB_safe` exclude the assertion-failure branch.  The
remaining exit assertions establish `SatisfiesContract`, including the
biconditional interpretation of `aborts_if`.

The theorem has two well-formedness hypotheses.  `hwfP : WfProg` supplies the
source typing guaranteed by bytecode verification.  `hwf` supplies
`WfProgram` for each compiled function, ensuring proper loop structure and
complete loop targets.
-/

namespace MoveModel.Prover.Translate

open MoveModel.Prover.Ivl
open MoveModel.IR

/-- **The prover is sound**: if the program is well-typed, the loop
metadata of every declared function is well-formed for its compiled
program, and every declared function's verification condition holds, then
every declared function satisfies its contract (on well-typed boundary
states). -/
theorem prover_sound (P : Program)
    (hwfP : WfProg P)
    (hwf : ∀ f d, P.funs f = some d →
      WfProgram (compileFun P d) (compAnns P d) (fun l => l))
    (hverified : ∀ f d, P.funs f = some d → Verified P f) :
    ∀ f d, P.funs f = some d → SatisfiesContract P f d := by
  intro f d hd m args htyargs htymem hreq
  constructor
  · intro m' rets hexec
    exact funExec_conforms P hwfP hverified hwf hd htyargs htymem hreq hexec
  · intro m' code hexec
    exact funExec_conforms P hwfP hverified hwf hd htyargs htymem hreq hexec

end MoveModel.Prover.Translate
