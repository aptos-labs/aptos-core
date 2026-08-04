-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move.Prover.Ivl.WpSound
import Move.IR.Semantics
import Move.Prover.Translate.Compile
import Move.Prover.Translate.Sim

/-!
# End-to-End Adequacy

The final theorem of the layered architecture: if every function of a
well-typed program verifies — i.e. the Lean-side WP of its compiled program
holds for all boundary states — then every function satisfies its declared
contract.

The proof composes the pieces of `Sim.lean`: the master induction
(`sim_aux`, over the big-step execution derivation — which is how the
mutual recursion between functions is untangled, rather than by induction
over a call graph) represents each source execution inside the compiled
program while carrying contract conformance and type preservation;
`wpB_sound`/`wpB_safe` (proven) exclude the assertion-failing branch, so
the exit assertions recorded along the represented execution deliver
`SatisfiesContract`, including the biconditional reading of `aborts_if`
(`funExec_conforms`).

Hypotheses: `hwfP` is the source-typing discipline (`WfProg`,
`IR/TypedCode.lean`) — what the bytecode verifier guarantees for the code
the prover verifies; `hwf` supplies the `WfProgram` side conditions of
`wpB_sound` for each compiled function: the declared loop structure is
proper and the declared loop targets are complete — the semantic
counterparts of the prover's fat-loop recognition and loop-target
analysis.
-/

namespace Move.Prover.Translate

open Move.Prover.Ivl
open Move.IR

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

end Move.Prover.Translate
