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
