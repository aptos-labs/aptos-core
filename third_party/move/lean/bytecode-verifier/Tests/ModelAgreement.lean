-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Model
import Tests.Differential

/-!
# Agreement of the model with the Rust verifier (deliverable A2)

`vm-oracle` records the Rust verifier's decision on every module of a
differential fixture, under the recorded production configuration. The model
must decide the same way: accept what Rust accepts, and reject with the same
status what Rust rejects.

Both local verification and link checks (against the fixture's other
modules) are compared. The model covers only some passes so far
(`Model/Verifier.lean`), so the fixtures use modules whose rejection, if any,
comes from a modeled pass. A module Rust accepts and the model rejects would
be a gap in the soundness argument; one Rust rejects and the model accepts
must come from an unmodeled pass.
-/

namespace MoveBytecode.Tests

open MoveBytecode.Import MoveBytecode.Model

def decision : VResult Unit → String
  | .ok () => "ok"
  | .error (.status code) => code
  | .error (.panic what) => s!"panic: {what}"
  | .error (.fuel what) => s!"fuel: {what}"

def checkAgreement (fixture : String) : IO Unit := do
  let (modules, decisions) ← IO.ofExcept (verifierDocument (← readFixture fixture))
  check (modules.size == decisions.size) s!"{fixture}: one decision per module"
  let P : Program := ⟨modules⟩
  let mut mismatches : Array String := #[]
  for ((m, rust), i) in (modules.toList.zip decisions.toList).zipIdx do
    let local_ := decision (verifyModule VerifierConfig.production m)
    if local_ != rust.local_ then
      mismatches := mismatches.push s!"{rust.module}: model {local_}, Rust {rust.local_}"
    let linking := decision (verifyLinking VerifierConfig.production m (P.others i))
    if linking != rust.linking then
      mismatches := mismatches.push s!"{rust.module} linking: model {linking}, Rust {rust.linking}"
  check mismatches.isEmpty s!"{fixture}:\n{String.intercalate "\n" mismatches.toList}"

#eval checkAgreement "diff_core.json"
#eval checkAgreement "diff_verifier.json"
#eval checkAgreement "diff_passes.json"
#eval checkAgreement "diff_data.json"
#eval checkAgreement "diff_databad.json"
#eval checkAgreement "diff_constants.json"
#eval checkAgreement "diff_refs.json"
#eval checkAgreement "diff_generics.json"
#eval checkAgreement "diff_closures.json"

end MoveBytecode.Tests
