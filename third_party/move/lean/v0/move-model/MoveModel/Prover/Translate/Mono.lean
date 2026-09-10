-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.IR.Mono.Transform
import MoveModel.Prover.Translate.Adequacy

/-!
# Verification of finite generic instances

This module is the proof-facing boundary between verification
monomorphization and the existing monomorphic IVL adequacy theorem.  A
`MonoVerification` records four independently checkable facts:

* the executable plan validator accepted the plan;
* the plan covers the runtime type-tag interaction quotient;
* the materialized program is well typed and has sound loop metadata; and
* every generated declaration verifies in the IVL.

`specializedSound` then applies the existing `prover_sound` theorem once to
the generated module, establishing the contracts of every given, collision,
combined-collision, and call-closure instance.
-/

namespace MoveModel.Prover.Translate

open MoveModel.IR

/-- All obligations needed to run the monomorphic prover over a finite
generic verification plan.  Keeping the semantic coverage certificate in
this bundle prevents callers from presenting successful IVL results for an
arbitrary, incomplete list of instances. -/
structure MonoVerification (m : Module) (plan : MonoPlan) : Prop where
  validPlan : plan.validate m = .ok ()
  coverage : MonoPlan.Certificate m plan
  wfProgram : WfProg (m.monomorphize plan).program
  wfLoops : ∀ f d, (m.monomorphize plan).program.funs f = some d →
    MoveModel.Prover.Ivl.WfProgram
      (compileFun (m.monomorphize plan).program d)
      (compAnns (m.monomorphize plan).program d)
      (fun label => label)
  verified : ∀ f d, (m.monomorphize plan).program.funs f = some d →
    Verified (m.monomorphize plan).program f

/-- Every materialized representative satisfies its specialized contract.
This is the point at which all discovered runtime-tag combinations are fed
through the complete, already-proved monomorphic IVL chain. -/
theorem MonoVerification.specializedSound {m : Module} {plan : MonoPlan}
    (h : MonoVerification m plan) :
    ∀ f d, (m.monomorphize plan).program.funs f = some d →
      SatisfiesContract (m.monomorphize plan).program f d :=
  prover_sound (m.monomorphize plan).program
    h.wfProgram h.wfLoops h.verified

end MoveModel.Prover.Translate
