-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.MonoVM.Link

/-!
# Linked MonoVM smoke suite

The M2 gate of the link design: one successful scalar call and one abort
through the linked test executable, with the adapter's build identity
logged. Runs only through `LeanerE2ETestDriver`; the opaque extern has no
implementation under `lake env lean`.
-/

namespace LeanerE2ETests.MonoVM

private def smokeSource : String :=
  "
module 0x42::smoke {
    public fun add(x: u64, y: u64): u64 {
        x + y
    }

    public fun abort_if_large(x: u64): u64 {
        assert!(x <= 100, 42);
        x
    }
}
"

private def addArgs : Array Value :=
  #[Value.integer 64 false "1", Value.integer 64 false "2"]

private def abortArgs : Array Value := #[Value.integer 64 false "200"]

private def smokeCalls : Array Call :=
  #[Call.mk "0x42::smoke::add" #[] addArgs,
    Call.mk "0x42::smoke::abort_if_large" #[] abortArgs]

private def smokeCompile : CompileSpec :=
  CompileSpec.mk #[SourceFile.mk "smoke.move" smokeSource] #[] 2

/-- Runs one scalar call and one abort through the linked adapter and checks
the normalized outcomes. -/
def testSmoke : IO Unit := do
  let request : Request :=
    Request.mk payloadVersion smokeCompile (Limits.mk 10000000000 none) smokeCalls
  let response ← run request
  IO.println
    s!"mono-move adapter identity: abi {response.identity.abi}, \
      profile {response.identity.profile}, {response.identity.rustc}"
  unless response.outcomes.size == 2 do
    throw <| IO.userError s!"expected two outcomes, got {repr response.outcomes}"
  match response.outcomes[0]! with
  | .returned values _ _ =>
      unless values == #[Value.integer 64 false "3"] do
        throw <| IO.userError s!"expected the addition to return 3, got {repr values}"
  | other => throw <| IO.userError s!"expected the addition to return, got {repr other}"
  match response.outcomes[1]! with
  | .aborted code _ _ =>
      unless code == 42 do
        throw <| IO.userError s!"expected the guard to abort with 42, got {repr code}"
  | other => throw <| IO.userError s!"expected the guard to abort, got {repr other}"

end LeanerE2ETests.MonoVM
