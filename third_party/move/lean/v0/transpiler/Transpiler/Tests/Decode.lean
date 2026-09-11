-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Transpiler

/-! The decoder over a real export: `basic_coin.move` is exported by the Aptos
CLI (`aptos move exchange --format ast`) at elaboration time and decoded; the
checked facts pin the module identity, the declaration counts, the spec
shapes, and the recovered comments. -/

open Transpiler Transpiler.Xast

def describeBasicCoin : IO Unit := do
  let m ← Cli.exportMoveFile "Transpiler/Tests/Programs/basic_coin.move"
  let withdraw := m.functions.find? (·.name == "withdraw")
  let conditions := withdraw.map (·.spec.conditions.length) |>.getD 0
  let withCode := withdraw.map (fun f => f.spec.conditions.filter (·.abortCode.isSome) |>.length) |>.getD 0
  let pragmas := m.functions.map fun f => f.pragmas.map (·.name)
  IO.println s!"{m.name}@{m.address} alias={m.addressAlias} functions={m.functions.length} \
    structs={m.structs.length} specFuns={m.specFuns.length} comments={m.comments.length} \
    sources={m.sources.map fun s => (s.splitOn "/").getLast!} withdrawConditions={conditions} \
    withCode={withCode} pragmas={pragmas} ownLine={m.comments.map (·.ownLine)}"

/--
info: basic_coin@0x42 alias=none functions=2 structs=1 specFuns=1 comments=3 sources=[basic_coin.move] withdrawConditions=3 withCode=1 pragmas=[[], [aborts_if_is_partial]] ownLine=[true, false, true]
-/
#guard_msgs in
#eval describeBasicCoin

/-- error: unsupported XAST version 99 (this transpiler reads version 4) -/
#guard_msgs in
#eval show IO Unit from do
  match Decode.parseModule "{\"schema\": \"move-xast-module\", \"version\": 99}" with
  | .ok _ => throw (IO.userError "accepted a future version")
  | .error e => throw (IO.userError e)

/-- error: unexpected XAST schema `other` (expected `move-xast-module`) -/
#guard_msgs in
#eval show IO Unit from do
  match Decode.parseModule "{\"schema\": \"other\", \"version\": 4}" with
  | .ok _ => throw (IO.userError "accepted a foreign schema")
  | .error e => throw (IO.userError e)

private def decodeOperationText (source : String) : Except String Operation := do
  let json ← Lean.Json.parse source
  Transpiler.Decode.decodeOperation json {}

#guard match decodeOperationText
    "{\"behavior\":{\"kind\":\"result_of\",\"range\":{\"pre\":1,\"post\":2}}}" with
  | .ok (.behavior .resultOf range) => range.pre == some 1 && range.post == some 2
  | _ => false

#guard match decodeOperationText
    "{\"behavior\":{\"kind\":{\"write_of\":3},\"range\":{\"pre\":null,\"post\":2}}}" with
  | .ok (.behavior (.writeOf 3) range) => range.pre.isNone && range.post == some 2
  | _ => false
