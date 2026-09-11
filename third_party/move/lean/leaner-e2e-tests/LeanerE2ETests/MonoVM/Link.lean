-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean.Data.Json
import LeanerE2ETests.MonoVM.Payload

/-!
# Linked MonoVM execution boundary

The Lean half of the `mono-move-lean-link` design: an opaque external
request/response call into the adapter staticlib linked into this package's
test driver. Elaborating this module needs no native symbols; evaluating it
does, so the harness runs only through the linked `LeanerE2ETestDriver`
executable, never under `lake env lean`.
-/

namespace LeanerE2ETests.MonoVM

open Lean

/-- The linked adapter call. Only owned bytes cross the boundary; domain
failures travel inside the response payload, so every ordinary failure
decodes into a response outcome. -/
@[extern "leaner_monovm_run_bytes"]
opaque runBytes (request : @& ByteArray) : IO ByteArray

/-- Runs one request against the linked MonoVM adapter, checking that the
native ABI and payload versions match this package's expectations. -/
def run (request : Request) : IO Response := do
  let json := encodeRequest request
  let bytes ← runBytes (Json.compress json).toUTF8
  let json ← match Json.parse (String.fromUTF8? bytes |>.getD "") with
    | .ok json => pure json
    | .error error => throw <| IO.userError s!"adapter response is not JSON: {error}"
  match decodeResponse json with
  | .ok response =>
      unless response.identity.abi == expectedAbiVersion do
        throw <| IO.userError
          s!"the linked adapter reports native ABI version \
            {response.identity.abi}; this package expects {expectedAbiVersion}; \
            rebuild the mono-move-lean-link staticlib with the lake native \
            target and relink"
      return response
  | .error error => throw <| IO.userError s!"malformed adapter response: {error}"
