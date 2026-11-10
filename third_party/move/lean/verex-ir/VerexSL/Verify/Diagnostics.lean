-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL.Verify.Contracts
import VerexIR.Proofs.Denote.Solve.Base

namespace VerexSL.Verify

open Lean VerexIR.Validation VerexIR.Proofs.Denote

/-- The name a proof of a function spells: its scoped name in its own namespace,
and the full name it is verified under from another's. -/
def proofTarget (function own : String) : String :=
  if function.endsWith s!"::{own}" then function else own

/-- What a failed verification asks for: an authored proof did not close
its obligations; an automatic one asks for a proof, including when its
bounded attempt ran out. `function` is the function's name within its
namespace, scopes included (`SpecTypes.scopedSpelling`); `target` is the same
name as the requested proof spells it in source. -/
def verificationFailureMessage (ns : ValidatedNamespace) (function target : String) (authored : Bool)
    (overBudget : Bool) (budget : Nat) (leaves : Array String)
    (stop : Option BudgetStop := none) : MessageData :=
  if authored then
    m!"verex verification failed: the proof of `{function}` does not establish its \
      specification"
  else
    let attempt := if overBudget then
        let position : MessageData := match stop with
          | none => m!""
          | some s =>
            let callPart : MessageData := match s.callee, s.callNumber with
              | some callee, some k => m!" at call {k} of `{callee}`"
              | some callee, none   => m!" at a call of `{callee}`"
              | _, _                => m!""
            let stagePart : MessageData := match s.lastStage with
              | some stage => m!" (stage {stage})"
              | none       => m!""
            m!" after deciding {s.leavesDecided} leaves{callPart}{stagePart}"
        m!"the automatic verification of `{function}` exceeded its budget of {budget} \
          maxHeartbeats{position}"
      else m!"the automatic verification of `{function}` failed"
    m!"verex verification failed: {attempt}; {proofRequest ns target leaves}"

end VerexSL.Verify
