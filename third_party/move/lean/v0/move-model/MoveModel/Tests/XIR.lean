-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.Frontend.XIR.FromIR
import MoveModel.Frontend.XIR.Json
import MoveModel.Tests.Common

/-! Deployable XIR is tested at its MoveModel exchange boundary, separate from
the semantic Move-source compiler tests. -/

namespace Tests.XIR

open MoveModel.IR
open MoveModel.Frontend.XIR

private def contract : Contract where
  requires := .value (.bool true)
  aborts := none
  ensures := .value (.bool true)
  modifies := []

private def fixture : MModule where
  structs := [{
    name := "Balance"
    fields := [("value", .u64)] }]
  funs := [{
    name := "deposit"
    params := 1
    locals := [.u64]
    returns := []
    blocks := [{ instrs := [], term := .ret [] }]
    loops := []
    spec := {
      requires := [.value (.bool true)]
      abortsIf := []
      ensures := [.value (.bool true)]
      modifies := [] } }]
  address := 0
  name := "Account"
  dialect := .stackless
  structMeta := [{
    name := "Balance"
    fieldNames := ["value"]
    abilities := { key := true } }]
  funMeta := [{
    name := "deposit"
    visibility := .public_
    isEntry := true
    acquires := []
    localNames := [some "amount"]
    sourceMap := some {
      span := some { start := 10, «end» := 30 }
      blocks := [{ instrs := [], term := some { start := 20, «end» := 26 } }]
    } }]

private def roundTrip : Except String String := do
  let encoded ← fixture.encodeJson
  let decoded ← decodeMModule encoded
  decoded.encodeJson

#guard fixture.name == "Account"
#test roundTrip.toOption = fixture.encodeJson.toOption
#test decodeMModule "{\"schema\":\"move-xir-module\",\"version\":99}"
  matches .error _

/-- The fixture with its struct made public. -/
private def publicStruct : MModule :=
  { fixture with structMeta := fixture.structMeta.map ({ · with visibility := .public_ }) }

private def decodedVisibility (text : Except String String) : Option Visibility :=
  match text >>= decodeMModule with
  | .ok m => m.structMeta.head?.map (·.visibility)
  | .error _ => none

-- A round trip only shows the codec agrees with itself; check the value.
#guard decodedVisibility publicStruct.encodeJson == some .public_

/-- `json` as a version 6 document, whose structs have no visibility. -/
private def asVersion6 (json : Lean.Json) : Lean.Json :=
  let structs := match json.getObjVal? "structs" with
    | .ok (.arr xs) => Lean.Json.arr (xs.map fun
        | .obj fields => .obj (fields.erase "visibility")
        | other => other)
    | _ => .arr #[]
  (json.setObjVal! "structs" structs).setObjVal! "version" (Lean.toJson (6 : Nat))

private def version6Text : Except String String := do
  let json ← Lean.Json.parse (← publicStruct.encodeJson)
  pure (asVersion6 json).compress

#guard decodedVisibility version6Text == some .private_

/-- A struct attribute argument written `name = value`, as Move's
`#[resource_group_member(group = ...)]` is. The decoder had no case for it,
which put every framework module using `resource_group_member` out of reach. -/
private def interop : MModule where
  structs := [{
    name := "Store"
    fields := [("value", .u64)] }]
  funs := []
  address := 0
  name := "Interop"
  dialect := .stackless
  structMeta := [{
    name := "Store"
    fieldNames := ["value"]
    abilities := { key := true }
    attributes := [{
      name := "resource_group_member"
      args := [.assign "group" (.name "0x1::object::ObjectGroup" [])] }] }]
  funMeta := []

private def interopRoundTrip : Except String String := do
  let encoded ← interop.encodeJson
  let decoded ← decodeMModule encoded
  decoded.encodeJson

#test interopRoundTrip.toOption = interop.encodeJson.toOption

-- The round trip above only shows the codec agrees with itself: a field dropped
-- on both sides is still self-consistent. This checks what actually came back.
#guard (match interop.encodeJson >>= decodeMModule with
  | .ok m => m.structMeta.head?.map (·.attributes.map (·.args))
  | .error _ => none) ==
    some [[.assign "group" (.name "0x1::object::ObjectGroup" [])]]

private def semantic : Module :=
  Module.ofLists 0 "Account"
    [{ fields := [.u64] }]
    [FunDecl.ofLists [] 0 [] [] [{ instrs := [], term := .ret [] }] 0 contract]
    [{ name := "Balance", fieldNames := ["value"], abilities := { key := true } }]
    [{ name := "deposit", visibility := .public_, isEntry := true, acquires := [] }]

#guard match MModule.ofIR semantic with
  | .ok module =>
      module.name == "Account" && module.structs.length == 1 &&
        module.funs.length == 1 && module.funMeta[0]?.any (·.isEntry)
  | .error _ => false

/-- The fixture with a recorded call, as an interface carries one. -/
private def withCalls : MModule :=
  { fixture with funMeta := fixture.funMeta.map ({ · with calls := [0] }) }

-- Decoded by value: a round trip alone would pass with `calls` dropped on both
-- sides.
#guard (match withCalls.encodeJson >>= decodeMModule with
  | .ok m => m.funMeta.head?.map (·.calls)
  | .error _ => none) == some [0]

/-- `json` stamped with `version`. -/
private def atVersion (version : Nat) (json : Lean.Json) : String :=
  (json.setObjVal! "version" (Lean.toJson version)).compress

private def decodesAt (version : Nat) (module : MModule) : Bool :=
  match module.encodeJson >>= Lean.Json.parse with
  | .ok json => (decodeMModule (atVersion version json)).toOption.isSome
  | .error _ => false

-- A field is refused in a document older than the field, as in Rust.
#guard decodesAt 9 withCalls
#guard !decodesAt 8 withCalls
#guard decodesAt 9 { fixture with friends := [{ address := 1, moduleName := "buddy" }] }
#guard !decodesAt 8 { fixture with friends := [{ address := 1, moduleName := "buddy" }] }

/-- The fixture with its functions' bodies removed, as an interface has them. -/
private def bodyless : MModule :=
  { fixture with funs := fixture.funs.map ({ · with blocks := [] }) }

-- Without a body, a function's calls are all there is of what it reaches, and
-- they arrived in version 9.
#guard decodesAt 9 bodyless
#guard !decodesAt 8 bodyless

/-- The fixture calling itself. A call is `{"function": id}`, the key a function
type also uses, so the gate must not take it for one. -/
private def withCallInstr : MModule :=
  { fixture with
    funs := fixture.funs.map fun f =>
      { f with blocks := [{ instrs := [.call [] (.function 0) [0]], term := .ret [] }] }
    funMeta := fixture.funMeta.map ({ · with sourceMap := none }) }

#guard decodesAt 7 withCallInstr

/-! Each gate names its field when a document is one version too old. Function
types and closures fail to decode at any version, so those checks look at
the message rather than at failure alone. -/

/-- The decode error for `json` at `version`, or "" when it decodes. -/
private def errorAt (version : Nat) (json : Lean.Json) : String :=
  match decodeMModule (atVersion version json) with
  | .ok _ => ""
  | .error e => e

private def mentions (text part : String) : Bool :=
  (text.splitOn part).length > 1

private def fixtureJson : Lean.Json :=
  match fixture.encodeJson >>= Lean.Json.parse with
  | .ok json => json
  | .error _ => .null

private def mapArray (key : String) (f : Lean.Json → Lean.Json) (json : Lean.Json) : Lean.Json :=
  match json.getObjVal? key with
  | .ok (.arr items) => json.setObjVal! key (.arr (items.map f))
  | _ => json

private def positional : Lean.Json :=
  fixtureJson |> mapArray "structs" (·.setObjVal! "fields"
    (.arr #[Lean.Json.mkObj [("name", "0"), ("ty", "u64")]]))
#guard mentions (errorAt 8 positional) "`positional field names`"
#guard errorAt 9 positional == ""

#guard !decodesAt 8 interop
#guard mentions ((interop.encodeJson >>= Lean.Json.parse).toOption.map (errorAt 8) |>.getD "")
  "`assign`"

private def publicStructJson : Lean.Json :=
  fixtureJson |> mapArray "structs" (·.setObjVal! "visibility" "public")
#guard mentions (errorAt 6 publicStructJson) "`visibility`"

private def functionTypeLocal : Lean.Json :=
  fixtureJson |> mapArray "functions" (·.setObjVal! "locals"
    (.arr #[Lean.Json.mkObj [("function", .arr #[.arr #[], .arr #[], .arr #[]])]]))
#guard mentions (errorAt 7 functionTypeLocal) "`fun`"

private def invokes : Lean.Json :=
  fixtureJson |> mapArray "functions" (·.setObjVal! "blocks" (.arr #[Lean.Json.mkObj [
    ("instrs", .arr #[Lean.Json.mkObj [("call", .arr #[.arr #[], "invoke", .arr #[]])]]),
    ("term", Lean.Json.mkObj [("ret", .arr #[])])]]))
#guard mentions (errorAt 7 invokes) "`closure operations`"

private def withExternalStruct : Lean.Json :=
  fixtureJson.setObjVal! "external_structs" (.arr #[Lean.Json.mkObj
    [("address", "0x1"), ("module", "string"), ("name", "String")]])
#guard mentions (errorAt 5 withExternalStruct) "`external_structs`"

#guard mentions (errorAt 4 fixtureJson) "`local_names`"

private def withoutLocalNames : Lean.Json :=
  fixtureJson |> mapArray "functions" (·.setObjVal! "local_names" (.arr #[]))
#guard mentions (errorAt 3 withoutLocalNames) "`source_map`"

/-- The fixture's JSON with an inline function body as source, which an
interface carries. -/
private def withSource (version : Nat) : Bool :=
  match fixture.encodeJson >>= Lean.Json.parse with
  | .ok json =>
      let functions := match json.getObjVal? "functions" with
        | .ok (.arr fs) => Lean.Json.arr (fs.map (·.setObjVal! "source" (.str "inline fun f() {}")))
        | _ => .arr #[]
      (decodeMModule (atVersion version (json.setObjVal! "functions" functions))).toOption.isSome
  | .error _ => false

#guard withSource 10
#guard !withSource 9

-- Decoded by value: the gate above passes even if `source` is dropped.
private def withInlineBody : MModule :=
  { fixture with funMeta := fixture.funMeta.map ({ · with source := some "inline fun f() {}" }) }
#guard (match withInlineBody.encodeJson >>= decodeMModule with
  | .ok m => m.funMeta.head?.bind (·.source)
  | .error _ => none) == some "inline fun f() {}"

end Tests.XIR
