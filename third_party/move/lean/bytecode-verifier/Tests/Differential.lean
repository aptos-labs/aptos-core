-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Import
import Tests.Import

/-!
# Differential tests against the Move VM and MonoVM (plan §5.2, Validation)

`Tests/fixtures/diff_*.json` record, for each call of a
`Tests/differential/*.spec.json`, the outcome on the legacy Move VM and on
MonoVM (`vm-oracle/`). The reference semantics must agree with both:

- equal return values, or the same abort code;
- an arithmetic failure where the legacy VM reports `ARITHMETIC_ERROR`, the
  module lock where it reports `RUNTIME_DISPATCH_ERROR`, a variant mismatch
  (R1) where it reports `STRUCT_VARIANT_MISMATCH`, and a vector length
  mismatch (R2) where it reports `VECTOR_OPERATION_ERROR`;
- against MonoVM, whose error kinds are coarser, only the outcome class of a
  non-abort failure is compared.

A disagreement is a finding to triage (plan §10.1): it locates a bug in the
semantics or in a VM, or an ambiguity in the language definition. This checks
the specification; the guarantees do not depend on either VM.

Every module must also pass the Rust verifier, since the semantics is the
contract for verified code.
-/

namespace MoveBytecode.Tests

open MoveBytecode.Import

mutual
/-- A value of the semantics is the one a VM reports: the same scalar, or a
vector of matching elements. VMs do not report a vector's element type. -/
def matchesPayload : Value → Payload → Bool
  | .bool b, .bool b' => b == b'
  | .int t v, .int t' v' => t == t' && v == v'
  | .address a, .address a' => a == a'
  | .vector _ vs, .vector ps => matchesPayloads vs ps
  | _, _ => false

def matchesPayloads : List Value → List Payload → Bool
  | [], [] => true
  | v :: vs, p :: ps => matchesPayload v p && matchesPayloads vs ps
  | _, _ => false
end

/-- Whether the reference semantics agrees with a VM outcome. `exactStatus`
compares failure statuses (legacy VM); otherwise only the class. VMs report
only the code of an abort with a message. -/
def agreesWithVm (lean : RunResult) (vm : VmOutcome) (exactStatus : Bool) : Bool :=
  match lean, vm with
  | .returned vs _, .returned ws => matchesPayloads vs ws
  | .failed (.abort c), .aborted c' => c == c'
  | .failed (.abortMessage c _), .aborted c' => c == c'
  | .failed .arithmetic, .failed s => !exactStatus || s == "ARITHMETIC_ERROR"
  | .failed .moduleLocked, .failed s => !exactStatus || s == "RUNTIME_DISPATCH_ERROR"
  | .failed .variantMismatch, .failed s => !exactStatus || s == "STRUCT_VARIANT_MISMATCH"
  | .failed .vectorLength, .failed s => !exactStatus || s == "VECTOR_OPERATION_ERROR"
  | _, _ => false

def runDifferential (fixture : String) : IO Unit := do
  let doc ← IO.ofExcept (diffDocument (← readFixture fixture))
  let P : Program := ⟨doc.modules⟩
  for d in doc.rustVerifier do
    check (d.local_ == "ok" && d.linking == "ok") s!"{fixture}: the Rust verifier rejects {d.module}"
  let mut mismatches : Array String := #[]
  for c in doc.calls do
    let f ← IO.ofExcept (findFunction P c.function)
    let r := match initialState P f c.args with
      | some s => run P noNatives 1000000 s
      | none => .wrong (.unsupported "no initial state")
    unless agreesWithVm r c.legacy true do
      mismatches := mismatches.push
        s!"{c.function} {repr c.args}: semantics {repr r}, legacy VM {repr c.legacy}"
    unless agreesWithVm r c.monovm false do
      mismatches := mismatches.push
        s!"{c.function} {repr c.args}: semantics {repr r}, MonoVM {repr c.monovm}"
  check mismatches.isEmpty s!"{fixture}:\n{String.intercalate "\n" mismatches.toList}"

#eval runDifferential "diff_core.json"
#eval runDifferential "diff_data.json"

/-- The specification's decoding of constants agrees with the production
decoder (deliverable A4). Each module of the fixture holds one constant; the
Rust verifier accepts it exactly when the specification gives its type a
runtime type and its bytes decode, with none left over, to a value of that
type, and reports `INVALID_CONSTANT_TYPE` exactly for a type that is not a
constant type. -/
def checkConstants (fixture : String) : IO Unit := do
  let doc ← IO.ofExcept (diffDocument (← readFixture fixture))
  check (doc.modules.size == doc.rustVerifier.size && doc.modules.size > 0)
    s!"{fixture}: one decision per module"
  let mut mismatches : Array String := #[]
  for (m, d) in doc.modules.toList.zip doc.rustVerifier.toList do
    match m.constantPool.toList with
    | [c] =>
      let spec : String :=
        if !c.type.isConstantType then "INVALID_CONSTANT_TYPE"
        else match RuntimeType.ofToken m c.type with
          | none => "no runtime type"
          | some t =>
            match decodeConstant t c.data with
            | none => "MALFORMED_CONSTANT_DATA"
            | some v => if v.conforms ⟨#[m]⟩ t then "ok" else "a value of another type"
      if spec != d.local_ then
        mismatches := mismatches.push s!"{d.module}: specification {spec}, Rust {d.local_}"
    | _ => mismatches := mismatches.push s!"{d.module}: expected exactly one constant"
  check mismatches.isEmpty s!"{fixture}:\n{String.intercalate "\n" mismatches.toList}"

#eval checkConstants "diff_constants.json"

end MoveBytecode.Tests
