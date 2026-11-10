-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Proofs.RefModel
import Tests.ModelAgreement

/-!
# The milestones of D1 with references on the reference corpus

`soundness_M3` covers programs whose function handles are no struct-API
mutable borrows (`Program.M3`). On the corpus of reference fixtures
(`diff_refs.json`), every module the Rust verifier accepts satisfies M3, and
the accepted modules pass references to calls and borrow global storage, so
the theorem covers them without their being values-only or free of global
borrows (M1, M2).
-/

namespace MoveBytecode.Tests

open MoveBytecode.Import MoveBytecode.Model MoveBytecode.Proofs ReferenceSafety

/-- `CompiledModule.M3`, decided. -/
def m3Bool (m : CompiledModule) : Bool :=
  m.functionHandles.toList.all fun fh => (borrowFieldMutable fh.attributes).isNone

theorem m3Bool_iff (m : CompiledModule) : m3Bool m = true ↔ m.M3 := by
  unfold m3Bool CompiledModule.M3
  rw [List.all_eq_true]
  simp only [Option.isNone_iff_eq_none]

/-- Code that borrows global storage. -/
def borrowsGlobal (m : CompiledModule) : Bool :=
  m.functionDefs.toList.any fun d => match d.code with
    | some code => code.code.toList.any fun i => match i with
      | .mutBorrowGlobal _ | .immBorrowGlobal _ => true
      | _ => false
    | none => false

/-- A function handle with a reference parameter or result. -/
def refSignatures (m : CompiledModule) : Bool :=
  m.functionHandles.toList.any fun fh =>
    (m.signatures[fh.parameters]?.getD []).any (fun t => (refKind t).isSome) ||
      (m.signatures[fh.returns]?.getD []).any (fun t => (refKind t).isSome)

def checkMilestones (fixture : String) : IO Unit := do
  let (modules, decisions) ← IO.ofExcept (verifierDocument (← readFixture fixture))
  let accepted := (modules.toList.zip decisions.toList).filterMap fun (m, d) =>
    if d.local_ == "ok" && d.linking == "ok" then some m else none
  check (!accepted.isEmpty) s!"{fixture}: no accepted module"
  for m in accepted do
    check (m3Bool m) s!"{fixture}: an accepted module is outside M3"
  check (accepted.any borrowsGlobal) s!"{fixture}: no accepted module borrows global storage"
  check (accepted.any refSignatures)
    s!"{fixture}: no accepted module has a reference parameter or result"

#eval checkMilestones "diff_refs.json"

end MoveBytecode.Tests
