-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerMove.Frontend.Decode

open LeanerMove.Frontend

private def literal (metadata : String) : Except String Xast.Exp := do
  let json ← Lean.Json.parse <|
    "{\"ty\":0,\"loc\":0,\"kind\":\"value\",\"value\":{\"number\":\"3\"}" ++
      metadata ++ "}"
  Decode.decodeExp json { types := #[.u256], locs := #[⟨0, 1, 2⟩] }

private def isDefaulted (result : Except String Xast.Exp) : Bool :=
  match result with
  | .ok (.mk .u256 _ (.value (.number 3) none defaulted)) => defaulted
  | _ => false

-- Equal source types and values must retain their different inference origins.
#guard isDefaulted (literal ",\"defaulted_num\":true")
#guard !(isDefaulted (literal ",\"defaulted_num\":false"))
#guard !(isDefaulted (literal ""))
#guard match literal ",\"defaulted_num\":\"true\"" with
  | .error _ => true
  | .ok _ => false
