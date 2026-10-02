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

/-- Interface metadata that only a Rust-produced module carries: a non-private
type, and an attribute argument written `name = value`.

Neither survived the codec before. A struct's visibility was simply not encoded,
so a module decoded and re-emitted here came back `private` and silently lost
the access it granted; an `assign` argument had no decoder case at all, which
put every framework module using `resource_group_member` out of reach. -/
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
    visibility := .public_
    attributes := [{
      name := "resource_group_member"
      args := [.assign "group" (.name "0x1::object::ObjectGroup" [])] }] }]
  funMeta := []

private def interopRoundTrip : Except String String := do
  let encoded ← interop.encodeJson
  let decoded ← decodeMModule encoded
  decoded.encodeJson

#test interopRoundTrip.toOption = interop.encodeJson.toOption

private def interopMeta : Option StructMeta :=
  match interop.encodeJson >>= decodeMModule with
  | .ok m => m.structMeta.head?
  | .error _ => none

-- The round trip above only shows the codec agrees with itself: a field dropped
-- on both sides is still self-consistent. This checks what actually came back.
#guard interopMeta.any (fun info =>
  info.visibility == .public_ &&
    info.attributes.map (·.args) ==
      [[.assign "group" (.name "0x1::object::ObjectGroup" [])]])

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

end Tests.XIR
