-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import MoveModel.Frontend.XIR.Json
import Move.Tests.Account
import Move.Tests.Enums

/-! XIR metadata and schema-versioned JSON round trips. -/

namespace Tests.MovePrograms.XIR

open MoveModel.Frontend.XIR
open Tests.MovePrograms.Account
open scoped Move.Compiler

/-! ## Tests -/

private def explicitAccount : MModule :=
  lowerToIR ``Tests.MovePrograms.Account

#guard explicitAccount.name == "Account"
#test explicitAccount.encodeJson.toOption = compiled.encodeJson.toOption

private def roundTrip : Except String String := do
  let encoded ← compiled.encodeJson
  let decoded ← decodeMModule encoded
  decoded.encodeJson

#test roundTrip.toOption = compiled.encodeJson.toOption

private def enumRoundTrip : Except String String := do
  let encoded ← Tests.MovePrograms.Enums.compiled.encodeJson
  let decoded ← decodeMModule encoded
  decoded.encodeJson

#test enumRoundTrip.toOption = Tests.MovePrograms.Enums.compiled.encodeJson.toOption

#guard compiled.name == "Account"
#guard compiled.address == 0
#guard compiled.structMeta.length == 2
#guard match compiled.structMeta[1]? with
  | some info => info.name == "Balance" && info.abilities.key
  | none => false
#guard match compiled.funMeta with
  | deposit :: withdraw :: _ =>
      deposit.isEntry && withdraw.isEntry &&
      deposit.visibility == .public_ && withdraw.visibility == .public_
  | _ => false

#test decodeMModule "{\"schema\":\"move-xir-module\",\"version\":99}"
  matches .error _

end Tests.MovePrograms.XIR
